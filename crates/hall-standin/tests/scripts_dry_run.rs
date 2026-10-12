//! scripts/test-setup and scripts/test-drive, run in --dry-run only: every
//! command they would run is printed, and checked here against the test setup's
//! rules (ADR 0003, decisions 24 and 26). Nothing is started and no herdr
//! session is touched.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const SESSIONS: [&str; 2] = ["hall-dev", "hall-dev-2"];
const MODEL: &str = "claude-haiku-4-5-20251001";
const HERDR: &str = "+ env -u HERDR_SOCKET_PATH -u HERDR_SESSION herdr ";
const REEVES: [&str; 2] = ["ansel", "bertil"];
const ROSTER: usize = 5;

fn scripts() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts")
}

/// This test's scratch directory under target/, as resolved on disk, since the
/// scripts print the reeve home as the stand-in resolves it.
fn tmp(test: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("scripts-dry-run")
        .join(test);
    std::fs::create_dir_all(&d).unwrap();
    d.canonicalize().unwrap()
}

/// A test reeve home under target/ that does not exist, so a dry run that
/// created anything would show.
fn home(test: &str) -> PathBuf {
    let h = tmp(test).join(".reeve-hall-dev");
    let _ = std::fs::remove_dir_all(&h);
    h
}

fn command(name: &str, args: &[&str]) -> Command {
    let mut c = Command::new("bash");
    c.arg(scripts().join(name))
        .args(args)
        .env("HALL_STANDIN_BIN", env!("CARGO_BIN_EXE_hall-standin"));
    c
}

fn script(name: &str, args: &[&str]) -> Output {
    command(name, args).output().unwrap()
}

/// Like [`script`], with HOME set to `user`.
fn script_as(user: &Path, name: &str, args: &[&str]) -> Output {
    command(name, args).env("HOME", user).output().unwrap()
}

/// Runs a dry run that must succeed, checks every printed command, and returns them.
fn dry(name: &str, home: &Path, args: &[&str]) -> Vec<String> {
    let mut all = vec!["--dry-run", "--home", home.to_str().unwrap()];
    all.extend_from_slice(args);
    dry_output(name, home, args, script(name, &all))
}

fn dry_output(name: &str, home: &Path, args: &[&str], out: Output) -> Vec<String> {
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        out.status.success(),
        "{name} {args:?}: {}\n{stdout}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!home.exists(), "{name} {args:?} created {}", home.display());

    // The inputs are printed before anything else.
    let first: Vec<&str> = stdout.lines().take(3).collect();
    assert_eq!(first[0], "test setup inputs:");
    assert!(first[1].contains(home.to_str().unwrap()), "{stdout}");
    assert!(first[2].ends_with("hall-dev hall-dev-2"), "{stdout}");

    assert!(
        !stdout.to_lowercase().contains("default"),
        "{name} {args:?} mentions default:\n{stdout}"
    );
    let cmds: Vec<String> = stdout
        .lines()
        .filter(|l| l.starts_with("+ "))
        .map(String::from)
        .collect();
    assert!(!cmds.is_empty());
    for c in &cmds {
        check_command(c, home);
    }
    cmds
}

fn check_command(c: &str, home: &Path) {
    assert!(!c.contains("server stop"), "{c}");
    if c.split_whitespace().any(|w| w == "herdr") {
        assert!(c.starts_with(HERDR), "herdr without a clean env: {c}");
        let rest = &c[HERDR.len()..];
        let named = SESSIONS
            .iter()
            .any(|s| rest.ends_with(&format!(" --session {s}")));
        let session_cmd = rest == "session list --json"
            || SESSIONS.iter().any(|s| {
                rest == format!("session stop {s} --json")
                    || rest == format!("session delete {s} --json")
            });
        assert!(
            named || session_cmd,
            "herdr call names no test session: {c}"
        );
    }
    if c.contains("claude ") {
        let h = home.to_str().unwrap();
        assert!(c.contains(&format!("REEVE_HOME=\"{h}\" claude ")), "{c}");
        assert!(c.contains(&format!("claude --model {MODEL}")), "{c}");
        assert!(c.contains(&format!("cd \"{h}/work/")), "{c}");
    }
}

fn count(cmds: &[String], needle: &str) -> usize {
    cmds.iter().filter(|c| c.contains(needle)).count()
}

#[test]
fn up() {
    let h = home("up");
    let cmds = dry("test-setup", &h, &["up"]);
    assert_eq!(count(&cmds, "claude --model"), ROSTER);
    assert_eq!(count(&cmds, " workspace create "), ROSTER);
    assert_eq!(count(&cmds, " place "), ROSTER);
    for s in SESSIONS {
        assert!(
            count(&cmds, &format!("--session {s}")) > 0,
            "nothing in {s}"
        );
    }
    assert_eq!(count(&cmds, "+ write "), 2);
    assert_eq!(count(&cmds, "cargo build"), 0);

    // Without HALL_STANDIN_BIN, up builds the stand-in it found in target/debug.
    let built = scripts().join("../target/debug/hall-standin");
    if built.canonicalize().ok()
        == Path::new(env!("CARGO_BIN_EXE_hall-standin"))
            .canonicalize()
            .ok()
    {
        let out = command(
            "test-setup",
            &["--dry-run", "--home", h.to_str().unwrap(), "up"],
        )
        .env_remove("HALL_STANDIN_BIN")
        .output()
        .unwrap();
        let cmds = dry_output("test-setup", &h, &["up"], out);
        assert_eq!(count(&cmds, "+ cargo build "), 1);
        assert_eq!(count(&cmds, "claude --model"), ROSTER);
    }
}

#[test]
fn down() {
    let h = home("down");
    let cmds = dry("test-setup", &h, &["down"]);
    for s in SESSIONS {
        assert_eq!(count(&cmds, &format!("herdr session stop {s} --json")), 1);
    }
    assert_eq!(count(&cmds, "session delete"), 0);
    let cmds = dry("test-setup", &h, &["down", "--delete"]);
    assert_eq!(count(&cmds, "session delete"), 2);
}

#[test]
fn status_is_read_only() {
    let h = home("status");
    let cmds = dry("test-setup", &h, &["status"]);
    for c in &cmds {
        let read = [
            "herdr session list --json",
            " pane get ",
            " agent explain ",
            "+ grep ",
            "+ jq ",
            " locate ",
            " listing",
        ]
        .iter()
        .any(|r| c.contains(r));
        assert!(read, "status runs something that is not a read: {c}");
    }
    assert!(count(&cmds, ".reeve/state") >= 2);
}

#[test]
fn drive() {
    let h = home("drive");
    for reeve in REEVES {
        let cmds = dry("test-drive", &h, &[reeve, "dialog"]);
        assert_eq!(count(&cmds, " agent prompt "), 1);
        assert_eq!(count(&cmds, " agent wait "), 1);
        assert_eq!(count(&cmds, " agent explain "), 1);

        let cmds = dry("test-drive", &h, &[reeve, "turn-end"]);
        assert_eq!(count(&cmds, " agent prompt "), 1);
        assert_eq!(count(&cmds, " --wait "), 1);
        assert_eq!(count(&cmds, " agent explain "), 2);

        let cmds = dry("test-drive", &h, &[reeve, "deny"]);
        assert_eq!(count(&cmds, " agent send-keys "), 1);
    }
    // No key that could answer a dialog with allow is ever sent.
    for action in ["dialog", "turn-end", "deny"] {
        for c in dry("test-drive", &h, &["ansel", action]) {
            if c.contains(" send-keys ") {
                assert!(c.contains(" esc --session "), "{c}");
            }
        }
    }
}

fn refused(out: &Output, needle: &str) {
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.lines().any(|l| l.starts_with("+ ")),
        "ran before refusing: {stdout}"
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains(needle), "stderr {err:?} lacks {needle:?}");
}

const ALL: [&[&str]; 6] = [
    &["test-setup", "up"],
    &["test-setup", "down"],
    &["test-setup", "status"],
    &["test-drive", "ansel", "dialog"],
    &["test-drive", "ansel", "turn-end"],
    &["test-drive", "ansel", "deny"],
];

#[test]
fn refuses_the_live_reeve_home() {
    let live = format!("{}/.reeve", std::env::var("HOME").unwrap());
    for cmd in ALL {
        for h in [live.clone(), format!("{live}/.reeve-hall-dev")] {
            let mut args = vec!["--dry-run", "--home", h.as_str()];
            args.extend_from_slice(&cmd[1..]);
            refused(&script(cmd[0], &args), "live reeve home");
        }
        let other = home("other").with_file_name("not-a-test-home");
        let mut args = vec!["--dry-run", "--home", other.to_str().unwrap()];
        args.extend_from_slice(&cmd[1..]);
        refused(&script(cmd[0], &args), "starts with .reeve-hall-dev");
    }
}

/// Runs every subcommand in --dry-run with `--home home` (and HOME set to
/// `user`, if given) and checks each is refused with `needle` on stderr.
fn refused_everywhere(user: Option<&Path>, home: &str, needle: &str) {
    for cmd in ALL {
        let mut args = vec!["--dry-run", "--home", home];
        args.extend_from_slice(&cmd[1..]);
        let out = match user {
            Some(u) => script_as(u, cmd[0], &args),
            None => script(cmd[0], &args),
        };
        refused(&out, needle);
    }
}

/// A fake HOME under target/, with a live reeve home `.reeve` in it when `with_reeve`.
fn fake_user(test: &str, with_reeve: bool) -> PathBuf {
    let u = tmp(test).join("user");
    let _ = std::fs::remove_dir_all(&u);
    std::fs::create_dir_all(&u).unwrap();
    if with_reeve {
        std::fs::create_dir_all(u.join(".reeve/state")).unwrap();
    }
    u
}

#[test]
fn refuses_the_live_reeve_home_in_any_letter_case() {
    let real = std::env::var("HOME").unwrap();
    for h in [
        format!("{real}/.REEVE"),
        format!("{real}/.Reeve/.reeve-hall-dev"),
    ] {
        refused_everywhere(None, &h, "live reeve home");
    }
    // With a fake HOME, whether its .reeve exists or not.
    for with_reeve in [true, false] {
        let user = fake_user(&format!("case-{with_reeve}"), with_reeve);
        let u = user.to_str().unwrap();
        for h in [format!("{u}/.REEVE/.reeve-hall-dev"), format!("{u}/.rEeVe")] {
            refused_everywhere(Some(&user), &h, "live reeve home");
        }
        assert_eq!(user.join(".reeve").exists(), with_reeve);
    }
}

#[test]
fn refuses_a_symlink_to_the_live_reeve_home() {
    let user = fake_user("symlink", true);
    std::os::unix::fs::symlink(user.join(".reeve"), user.join(".reeve-hall-dev")).unwrap();
    std::os::unix::fs::symlink(user.join(".REEVE"), user.join(".reeve-hall-dev-2")).unwrap();
    std::os::unix::fs::symlink(user.join(".reeve/state"), user.join(".reeve-hall-dev-3")).unwrap();
    for name in [".reeve-hall-dev", ".reeve-hall-dev-2", ".reeve-hall-dev-3"] {
        let h = user.join(name);
        refused_everywhere(Some(&user), h.to_str().unwrap(), "live reeve home");
    }
    // A link whose target does not exist yet could lead there once it does.
    std::os::unix::fs::symlink(user.join(".reeve/new"), user.join(".reeve-hall-dev-4")).unwrap();
    let h = user.join(".reeve-hall-dev-4");
    refused_everywhere(Some(&user), h.to_str().unwrap(), "does not exist");
}

#[test]
fn refuses_the_user_database_home_under_a_wrong_home() {
    let passwd = hall_standin::home::passwd_home().unwrap();
    let user = fake_user("wrong-home", false);
    let h = passwd.join(".reeve/.reeve-hall-dev");
    refused_everywhere(Some(&user), h.to_str().unwrap(), "live reeve home");
}

#[test]
fn refuses_a_home_that_resolves_to_an_unsafe_path() {
    let dir = tmp("unsafe");
    for (i, odd) in ["a\"b", "c$(touch x)d", "e'f"].iter().enumerate() {
        let target = dir.join(odd).join(".reeve-hall-dev");
        std::fs::create_dir_all(&target).unwrap();
        let link = dir.join(format!("link-{i}"));
        let _ = std::fs::remove_dir_all(&link);
        std::fs::create_dir_all(&link).unwrap();
        std::os::unix::fs::symlink(&target, link.join(".reeve-hall-dev")).unwrap();
        let h = link.join(".reeve-hall-dev");
        refused_everywhere(None, h.to_str().unwrap(), "it resolves to");
    }
    assert!(!dir.join("x").exists());
}

#[test]
fn refuses_without_the_stand_in() {
    let h = home("no-standin");
    for cmd in ALL {
        let mut args = vec!["--dry-run", "--home", h.to_str().unwrap()];
        args.extend_from_slice(&cmd[1..]);
        let out = command(cmd[0], &args)
            .env("HALL_STANDIN_BIN", tmp("no-standin").join("missing"))
            .output()
            .unwrap();
        refused(&out, "is not built");
    }
}

#[test]
fn refuses_the_live_herdr_session() {
    let h = home("refuse-default");
    for cmd in ALL {
        for (flag, name) in [
            ("--session-1", "default"),
            ("--session-2", "default"),
            ("--session-1", "Default"),
        ] {
            let mut args = vec!["--dry-run", "--home", h.to_str().unwrap(), flag, name];
            args.extend_from_slice(&cmd[1..]);
            refused(&script(cmd[0], &args), "belongs to the live setup");
        }
        let mut args = vec![
            "--dry-run",
            "--home",
            h.to_str().unwrap(),
            "--session-2",
            "work",
        ];
        args.extend_from_slice(&cmd[1..]);
        refused(
            &script(cmd[0], &args),
            "test herdr sessions are named hall-dev*",
        );
    }
}

#[test]
fn inputs_are_required() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("scripts-dry-run/conf");
    std::fs::create_dir_all(&dir).unwrap();
    let conf = dir.join("no-home.conf");
    std::fs::write(
        &conf,
        "HALL_DEV_SESSION_1=hall-dev\nHALL_DEV_SESSION_2=hall-dev-2\nHALL_DEV_ROSTER=(\"reeve ansel 1\")\n",
    )
    .unwrap();
    let out = script(
        "test-setup",
        &["--dry-run", "--config", conf.to_str().unwrap(), "up"],
    );
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stderr).contains("no reeve home"));
}
