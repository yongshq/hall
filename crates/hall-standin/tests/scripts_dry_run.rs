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

/// A test reeve home under target/ that does not exist, so a dry run that
/// created anything would show.
fn home(test: &str) -> PathBuf {
    let h = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("scripts-dry-run")
        .join(test)
        .join(".reeve-hall-dev");
    let _ = std::fs::remove_dir_all(&h);
    h
}

fn script(name: &str, args: &[&str]) -> Output {
    Command::new("bash")
        .arg(scripts().join(name))
        .args(args)
        .output()
        .unwrap()
}

/// Runs a dry run that must succeed, checks every printed command, and returns them.
fn dry(name: &str, home: &Path, args: &[&str]) -> Vec<String> {
    let mut all = vec!["--dry-run", "--home", home.to_str().unwrap()];
    all.extend_from_slice(args);
    let out = script(name, &all);
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
