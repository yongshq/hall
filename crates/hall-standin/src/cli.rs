//! Command line for the stand-in. `--home` is required on every call and has no
//! default and no environment fallback.

use std::io::Write;
use std::path::{Path, PathBuf};

use crate::home::{HomeError, check_inner_links, resolve_test_root};
use crate::listing::Listing;
use crate::world::{World, WorldEdit, locate, world_path};

pub const USAGE: &str = "\
usage: hall-standin --home <test reeve home> [command]

  listing                              print the draft listing as JSON (the default)
  set <name> <key>=<value>...          change a reeve or errand in the world file
                                         errand keys: state, question, waiting, idle, wedged, in_flight
                                         reeve keys: live
                                         an empty question= clears it
  place <name> <hall-session> [pane]   record where a reeve's or errand's session sits;
                                         no pane forgets the recorded one
  locate <name>                        print `<hall-session> <pane-id>` for a reeve or errand
  sessions                             print `<kind> <name> <hall-session> <pane-id|->` per
                                         reeve and errand in flight
  check-home                           print the test reeve home as resolved on disk, or
                                         refuse it; lists its directories but
                                         reads no file inside it
  check-root                           as check-home, without the walk inside the home

The world file is <home>/standin/world.toml. --home must be a directory whose
name starts with .reeve-hall-dev, and never ~/.reeve (for HOME or for the
home in the user database), in any letter case or through any symlink. It
may not have .. in it. Every command but check-root then walks the home,
save the contents of xdg-config and xdg-state other than their herdr folder,
and refuses it if anything inside is a symlink, a file with more than one
name (a hard link), or neither a regular file nor a directory.";

#[derive(Debug, PartialEq, Eq)]
pub enum CliError {
    /// Bad arguments: exit status 2.
    Usage(String),
    /// A refusal or a failure: exit status 1.
    Failed(String),
}

impl CliError {
    pub fn exit_code(&self) -> i32 {
        match self {
            CliError::Usage(_) => 2,
            CliError::Failed(_) => 1,
        }
    }

    pub fn message(&self) -> &str {
        match self {
            CliError::Usage(m) | CliError::Failed(m) => m,
        }
    }
}

fn failed(e: impl ToString) -> CliError {
    CliError::Failed(e.to_string())
}

/// Runs the stand-in with `args` (without the program name). `user_homes` are the
/// liege's home directories ([`crate::home::live_user_homes`]), used only to rule
/// out the live reeve home.
pub fn run(
    args: &[String],
    user_homes: Result<Vec<PathBuf>, HomeError>,
    out: &mut dyn Write,
) -> Result<(), CliError> {
    let mut home: Option<PathBuf> = None;
    let mut rest: Vec<&str> = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-h" | "--help" => {
                writeln!(out, "{USAGE}").map_err(failed)?;
                return Ok(());
            }
            "--home" => {
                let v = it
                    .next()
                    .ok_or_else(|| CliError::Usage("--home needs a path".into()))?;
                home = Some(PathBuf::from(v));
            }
            s if s.starts_with("--home=") => home = Some(PathBuf::from(&s["--home=".len()..])),
            s if s.starts_with("--") => {
                return Err(CliError::Usage(format!("unknown option {s}\n\n{USAGE}")));
            }
            s => rest.push(s),
        }
    }

    let home = home.ok_or_else(|| {
        CliError::Usage(format!(
            "--home <test reeve home> is required; there is no default\n\n{USAGE}"
        ))
    })?;
    if home.as_os_str().is_empty() {
        return Err(CliError::Usage("--home is empty".into()));
    }
    let user_homes = user_homes.map_err(failed)?;
    let home = resolve_test_root(&home, &user_homes).map_err(failed)?;
    if rest == ["check-root"] {
        return writeln!(out, "{}", home.display()).map_err(failed);
    }
    check_inner_links(&home).map_err(failed)?;

    match rest.as_slice() {
        ["check-home"] => writeln!(out, "{}", home.display()).map_err(failed),
        [] | ["listing"] => {
            let world = World::load(&home).map_err(failed)?;
            writeln!(out, "{}", Listing::from_world(&world).to_json()).map_err(failed)
        }
        ["set", target, pairs @ ..] if !pairs.is_empty() => edit(&home, |ed| {
            for pair in pairs {
                let (k, v) = pair.split_once('=').ok_or_else(|| {
                    CliError::Usage(format!("expected <key>=<value>, got {pair}"))
                })?;
                ed.set(target, k, v).map_err(failed)?;
            }
            Ok(())
        }),
        ["place", target, session] => {
            edit(&home, |ed| ed.place(target, session, None).map_err(failed))
        }
        ["place", target, session, pane] => edit(&home, |ed| {
            ed.place(target, session, Some(pane)).map_err(failed)
        }),
        ["locate", target] => {
            let world = World::load(&home).map_err(failed)?;
            match locate(&world, target) {
                Some((session, Some(pane))) => writeln!(out, "{session} {pane}").map_err(failed),
                Some((session, None)) => Err(CliError::Failed(format!(
                    "{target} has no pane recorded yet in hall session {session}"
                ))),
                None => Err(CliError::Failed(format!(
                    "no reeve or errand called {target} in the world file"
                ))),
            }
        }
        ["sessions"] => {
            let world = World::load(&home).map_err(failed)?;
            let mut text = String::new();
            for r in &world.reeve {
                let pane = r.pane_id.as_deref().unwrap_or("-");
                text.push_str(&format!("reeve {} {} {pane}\n", r.name, r.herdr_session));
            }
            for e in world.errand.iter().filter(|e| e.in_flight) {
                let pane = e.pane_id.as_deref().unwrap_or("-");
                text.push_str(&format!("errand {} {} {pane}\n", e.id, e.herdr_session));
            }
            out.write_all(text.as_bytes()).map_err(failed)
        }
        _ => Err(CliError::Usage(format!(
            "unknown command {}\n\n{USAGE}",
            rest.join(" ")
        ))),
    }
}

/// Applies `f` to the world file and writes it back only when the result is a valid world.
fn edit(
    home: &Path,
    f: impl FnOnce(&mut WorldEdit) -> Result<(), CliError>,
) -> Result<(), CliError> {
    let path = world_path(home);
    let text = std::fs::read_to_string(&path)
        .map_err(|e| failed(format!("cannot read world file {}: {e}", path.display())))?;
    let mut ed = WorldEdit::parse(&text).map_err(|e| failed(format!("{}: {e}", path.display())))?;
    f(&mut ed)?;
    let new = ed.finish().map_err(failed)?;
    // A leftover temp file is removed, never followed if it is a symlink, and
    // the new one is created afresh (O_EXCL), so the write cannot land
    // anywhere else; the rename replaces world.toml itself, not what it names.
    let tmp = path.with_extension("toml.tmp");
    if let Err(e) = std::fs::remove_file(&tmp)
        && e.kind() != std::io::ErrorKind::NotFound
    {
        return Err(failed(e));
    }
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp)
        .and_then(|mut f| f.write_all(new.as_bytes()))
        .map_err(failed)?;
    std::fs::rename(&tmp, &path).map_err(failed)
}
