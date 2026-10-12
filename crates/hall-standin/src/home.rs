//! The test reeve home guard. The stand-in only ever reads a test reeve home,
//! never the live one (ADR 0003, decision 10), and there is no default.

use std::fmt;
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
    /// The liege's own home could not be found, so the live reeve home cannot be ruled out.
    NoUserHome,
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
            HomeError::NoUserHome => write!(
                f,
                "refusing: HOME is not set, so the live reeve home cannot be ruled out"
            ),
        }
    }
}

impl std::error::Error for HomeError {}

/// Resolves `arg` to an absolute test reeve home, or refuses it.
///
/// `user_home` is the liege's home directory, whose `.reeve` is the live reeve home.
/// Symlinks are followed where the path exists, so a link named `.reeve-hall-dev`
/// that points at `~/.reeve` is refused too.
pub fn resolve_test_home(arg: &Path, user_home: Option<&Path>) -> Result<PathBuf, HomeError> {
    let user_home = user_home.ok_or(HomeError::NoUserHome)?;
    let home = real(&absolute(arg));
    let live = real(&absolute(&user_home.join(".reeve")));

    if home.starts_with(&live) {
        return Err(HomeError::LiveHome(home));
    }
    let named_as_test = home
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with(TEST_HOME_PREFIX));
    if !named_as_test {
        return Err(HomeError::NotTestHome(home));
    }
    Ok(home)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_the_live_home_and_inside_it() {
        let user = Path::new("/nonexistent-user");
        for p in [
            "/nonexistent-user/.reeve",
            "/nonexistent-user/.reeve/",
            "/nonexistent-user/.reeve/.reeve-hall-dev",
            "/nonexistent-user/x/../.reeve",
        ] {
            assert!(
                matches!(
                    resolve_test_home(Path::new(p), Some(user)),
                    Err(HomeError::LiveHome(_))
                ),
                "{p}"
            );
        }
    }

    #[test]
    fn refuses_a_home_not_named_as_a_test_home() {
        let user = Path::new("/nonexistent-user");
        for p in ["/nonexistent-user/.reeve-dev", "/tmp/home", "/"] {
            assert!(
                matches!(
                    resolve_test_home(Path::new(p), Some(user)),
                    Err(HomeError::NotTestHome(_))
                ),
                "{p}"
            );
        }
    }

    #[test]
    fn accepts_test_homes() {
        let user = Path::new("/nonexistent-user");
        for p in [
            "/nonexistent-user/.reeve-hall-dev",
            "/nonexistent-user/.reeve-hall-dev-2",
        ] {
            assert_eq!(
                resolve_test_home(Path::new(p), Some(user)).unwrap(),
                PathBuf::from(p)
            );
        }
    }

    #[test]
    fn refuses_without_a_user_home() {
        assert_eq!(
            resolve_test_home(Path::new("/x/.reeve-hall-dev"), None),
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
