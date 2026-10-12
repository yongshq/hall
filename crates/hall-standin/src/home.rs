//! The test reeve home guard. The stand-in only ever reads a test reeve home,
//! never the live one (ADR 0003, decision 10), and there is no default.

use std::ffi::{CStr, OsStr};
use std::fmt;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

/// Every test reeve home's directory name starts with this.
pub const TEST_HOME_PREFIX: &str = ".reeve-hall-dev";

/// The live setup's herdr session. Never part of the test setup.
pub const LIVE_HERDR_SESSION: &str = "default";

#[derive(Debug, PartialEq, Eq)]
pub enum HomeError {
    /// The path is the live reeve home, or inside it.
    LiveHome(PathBuf),
    /// The directory name does not start with [`TEST_HOME_PREFIX`].
    NotTestHome(PathBuf),
    /// The path goes through a symlink whose target does not exist.
    DanglingLink(PathBuf),
    /// The path has a `..` component, which could undo a symlink before it.
    ParentDir(PathBuf),
    /// Something the test setup reads or writes inside the home is a symlink,
    /// so it could lead out of the home (into the live one, say).
    InnerLink(PathBuf),
    /// The liege's own home could not be found, so the live reeve home cannot be ruled out.
    NoUserHome,
    /// The liege's home in the user database could not be read, so the live
    /// reeve home cannot be ruled out.
    NoPasswdHome,
}

impl fmt::Display for HomeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HomeError::LiveHome(p) => write!(
                f,
                "refusing reeve home {}: it is the live reeve home (~/.reeve) or inside it",
                p.display()
            ),
            HomeError::NotTestHome(p) => write!(
                f,
                "refusing reeve home {}: a test reeve home's directory name starts with {TEST_HOME_PREFIX}",
                p.display()
            ),
            HomeError::DanglingLink(p) => write!(
                f,
                "refusing reeve home: {} is a symlink to a path that does not exist, so where it leads cannot be checked",
                p.display()
            ),
            HomeError::ParentDir(p) => write!(
                f,
                "refusing reeve home {}: no .. in the path, since it could undo a symlink before it",
                p.display()
            ),
            HomeError::InnerLink(p) => write!(
                f,
                "refusing reeve home: {} is a symlink, and nothing the test setup uses inside a test reeve home may lead out of it",
                p.display()
            ),
            HomeError::NoUserHome => write!(
                f,
                "refusing: HOME is not set, so the live reeve home cannot be ruled out"
            ),
            HomeError::NoPasswdHome => write!(
                f,
                "refusing: the user database gives no home directory, so the live reeve home cannot be ruled out"
            ),
        }
    }
}

impl std::error::Error for HomeError {}

/// The liege's home directories whose `.reeve` is the live reeve home: `HOME`
/// (`env_home`) and the home the user database records, since either may be
/// what reeve uses. Both are required.
pub fn live_user_homes(env_home: Option<&Path>) -> Result<Vec<PathBuf>, HomeError> {
    let env_home = env_home
        .filter(|h| !h.as_os_str().is_empty())
        .ok_or(HomeError::NoUserHome)?;
    let passwd = passwd_home().ok_or(HomeError::NoPasswdHome)?;
    Ok(vec![env_home.to_path_buf(), passwd])
}

/// The current user's home directory in the user database.
pub fn passwd_home() -> Option<PathBuf> {
    let mut buf: Vec<libc::c_char> = vec![0; 4096];
    loop {
        // SAFETY: an all-zero passwd is a valid value for getpwuid_r to fill in.
        let mut pwd: libc::passwd = unsafe { std::mem::zeroed() };
        let mut found: *mut libc::passwd = std::ptr::null_mut();
        // SAFETY: every pointer is valid for the call, and buf outlives the
        // strings getpwuid_r points pwd at.
        let rc = unsafe {
            libc::getpwuid_r(
                libc::getuid(),
                &mut pwd,
                buf.as_mut_ptr(),
                buf.len(),
                &mut found,
            )
        };
        if rc == libc::ERANGE && buf.len() < 1 << 20 {
            buf.resize(buf.len() * 2, 0);
            continue;
        }
        if rc != 0 || found.is_null() || pwd.pw_dir.is_null() {
            return None;
        }
        // SAFETY: pw_dir is a NUL-terminated string inside buf.
        let dir = unsafe { CStr::from_ptr(pwd.pw_dir) }.to_bytes();
        return (!dir.is_empty()).then(|| PathBuf::from(OsStr::from_bytes(dir)));
    }
}

/// Resolves `arg` to an absolute test reeve home, or refuses it.
///
/// `user_homes` are the liege's home directories, each of whose `.reeve` is the
/// live reeve home ([`live_user_homes`]). Symlinks are followed where the path
/// exists, so a link named `.reeve-hall-dev` that points at `~/.reeve` is
/// refused too. The live reeve home is matched by what the path names on disk,
/// not by its spelling, so a case variant (`~/.REEVE`) or another route to the
/// same directory is refused as well. A `..` in `arg` is refused, since it is
/// only undone after the symlinks before it are followed. So is a home whose
/// `standin` or `work`, or an entry directly inside either, is a symlink
/// ([`check_inner_links`]).
pub fn resolve_test_home(arg: &Path, user_homes: &[PathBuf]) -> Result<PathBuf, HomeError> {
    if user_homes.is_empty() {
        return Err(HomeError::NoUserHome);
    }
    if arg.components().any(|c| c == Component::ParentDir) {
        return Err(HomeError::ParentDir(arg.to_path_buf()));
    }
    let home = real(&absolute(arg));
    let live: Vec<Place> = user_homes
        .iter()
        .filter_map(|u| places(&real(&absolute(&u.join(".reeve")))).pop())
        .collect();

    if places(&home).iter().any(|p| live.contains(p)) {
        return Err(HomeError::LiveHome(home));
    }
    // A symlink whose target does not exist yet could still lead into the live
    // reeve home once something creates it.
    if let Some(link) = home
        .ancestors()
        .find(|a| a.symlink_metadata().is_ok() && a.metadata().is_err())
    {
        return Err(HomeError::DanglingLink(link.to_path_buf()));
    }
    let named_as_test = home
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with(TEST_HOME_PREFIX));
    if !named_as_test {
        return Err(HomeError::NotTestHome(home));
    }
    check_inner_links(&home)?;
    Ok(home)
}

/// Refuses a test reeve home whose `standin` or `work`, or any entry directly
/// inside either, is a symlink: the world file, the live baseline and each
/// claude's working directory sit there, and a symlink would carry a read, a
/// write or a claude session out of the home. Missing entries are fine.
pub fn check_inner_links(home: &Path) -> Result<(), HomeError> {
    for dir in ["standin", "work"].map(|d| home.join(d)) {
        let Ok(meta) = dir.symlink_metadata() else {
            continue;
        };
        if meta.file_type().is_symlink() {
            return Err(HomeError::InnerLink(dir));
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            if entry.file_type().is_ok_and(|t| t.is_symlink()) {
                return Err(HomeError::InnerLink(entry.path()));
            }
        }
    }
    Ok(())
}

/// True when `name` is the live herdr session. Case is ignored, since herdr's
/// session directories may sit on a case-insensitive filesystem.
pub fn is_live_herdr_session(name: &str) -> bool {
    name.eq_ignore_ascii_case(LIVE_HERDR_SESSION)
}

fn absolute(p: &Path) -> PathBuf {
    let joined = if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(p)
    };
    let mut out = PathBuf::new();
    for c in joined.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

/// The canonical path when it exists, else the canonical form of its nearest
/// existing ancestor with the rest appended.
fn real(p: &Path) -> PathBuf {
    if let Ok(c) = p.canonicalize() {
        return c;
    }
    match (p.parent(), p.file_name()) {
        (Some(parent), Some(name)) => real(parent).join(name),
        _ => p.to_path_buf(),
    }
}

/// Where a path sits on disk: its deepest existing ancestor (or itself), by
/// device and inode, and the names below that, lowercased. Two spellings of one
/// directory have the same place: case variants on a case-insensitive volume,
/// and routes through a firmlink or a bind mount.
#[derive(Debug, PartialEq, Eq)]
struct Place {
    dev: u64,
    ino: u64,
    tail: Vec<String>,
}

/// The place of every prefix of the absolute path `p`, from the root down to `p`.
fn places(p: &Path) -> Vec<Place> {
    let mut out: Vec<Place> = Vec::new();
    let mut prefix = PathBuf::new();
    for c in p.components() {
        prefix.push(c);
        let above_exists = out.last().is_none_or(|last| last.tail.is_empty());
        let place = match std::fs::metadata(&prefix) {
            Ok(m) if above_exists => Place {
                dev: m.dev(),
                ino: m.ino(),
                tail: Vec::new(),
            },
            _ => {
                let last = out.last().expect("the root exists");
                let mut tail = last.tail.clone();
                tail.push(c.as_os_str().to_string_lossy().to_lowercase());
                Place {
                    dev: last.dev,
                    ino: last.ino,
                    tail,
                }
            }
        };
        out.push(place);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_the_live_home_and_inside_it() {
        let user = [PathBuf::from("/nonexistent-user")];
        for p in [
            "/nonexistent-user/.reeve",
            "/nonexistent-user/.reeve/",
            "/nonexistent-user/.reeve/.reeve-hall-dev",
            "/nonexistent-user/.REEVE",
            "/nonexistent-user/.Reeve/.reeve-hall-dev",
            "/NonExistent-User/.reeve/.reeve-hall-dev",
            "//nonexistent-user/./.reeve",
        ] {
            assert!(
                matches!(
                    resolve_test_home(Path::new(p), &user),
                    Err(HomeError::LiveHome(_))
                ),
                "{p}"
            );
        }
    }

    #[test]
    fn refuses_a_parent_dir_component() {
        let user = [PathBuf::from("/nonexistent-user")];
        for p in [
            "/nonexistent-user/x/../.reeve",
            "/nonexistent-user/link/../.reeve-hall-dev",
            "link/../.reeve-hall-dev",
            "../.reeve-hall-dev",
            "/nonexistent-user/.reeve-hall-dev/..",
        ] {
            assert!(
                matches!(
                    resolve_test_home(Path::new(p), &user),
                    Err(HomeError::ParentDir(_))
                ),
                "{p}"
            );
        }
    }

    #[test]
    fn refuses_a_home_not_named_as_a_test_home() {
        let user = [PathBuf::from("/nonexistent-user")];
        for p in ["/nonexistent-user/.reeve-dev", "/tmp/home", "/"] {
            assert!(
                matches!(
                    resolve_test_home(Path::new(p), &user),
                    Err(HomeError::NotTestHome(_))
                ),
                "{p}"
            );
        }
    }

    #[test]
    fn accepts_test_homes() {
        let user = [PathBuf::from("/nonexistent-user")];
        for p in [
            "/nonexistent-user/.reeve-hall-dev",
            "/nonexistent-user/.reeve-hall-dev-2",
        ] {
            assert_eq!(
                resolve_test_home(Path::new(p), &user).unwrap(),
                PathBuf::from(p)
            );
        }
    }

    #[test]
    fn refuses_the_live_home_of_every_user_home() {
        let users = [
            PathBuf::from("/nonexistent-env-home"),
            PathBuf::from("/nonexistent-passwd-home"),
        ];
        for p in [
            "/nonexistent-env-home/.reeve/.reeve-hall-dev",
            "/nonexistent-passwd-home/.reeve/.reeve-hall-dev",
        ] {
            assert!(
                matches!(
                    resolve_test_home(Path::new(p), &users),
                    Err(HomeError::LiveHome(_))
                ),
                "{p}"
            );
        }
    }

    #[test]
    fn passwd_home_is_found() {
        assert!(passwd_home().is_some_and(|h| h.is_absolute()));
    }

    #[test]
    fn refuses_without_a_user_home() {
        assert_eq!(
            resolve_test_home(Path::new("/x/.reeve-hall-dev"), &[]),
            Err(HomeError::NoUserHome)
        );
        assert_eq!(live_user_homes(None), Err(HomeError::NoUserHome));
        assert_eq!(
            live_user_homes(Some(Path::new(""))),
            Err(HomeError::NoUserHome)
        );
    }

    #[test]
    fn live_herdr_session_in_any_case() {
        assert!(is_live_herdr_session("default"));
        assert!(is_live_herdr_session("Default"));
        assert!(!is_live_herdr_session("hall-dev"));
    }
}
