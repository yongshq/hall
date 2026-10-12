//! scripts/test-setup and scripts/test-drive run for real, but against a fake
//! `herdr` on PATH and a fake HOME, all under target/. This exercises what a dry
//! run cannot: reading herdr's JSON, seeding the world file, recording panes so
//! the stand-in's session locations match, and the status checks. No real herdr
//! session, reeve home or claude is involved.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

/// Logs each call (and whether the herdr env vars leaked through) and answers
/// like herdr 0.9.0 does for the calls the scripts make.
const FAKE_HERDR: &str = r#"#!/usr/bin/env bash
set -euo pipefail
dir=$(dirname "$0")
printf '%s|socket=%s|session=%s\n' "$*" "${HERDR_SOCKET_PATH-unset}" "${HERDR_SESSION-unset}" >>"$dir/calls.log"
case "$*" in
  "session list --json")
    if [ -e "$dir/stopped" ]; then running=false; else running=true; fi
    printf '{"sessions":[{"default":true,"name":"default","running":true,"session_dir":"%s/live","socket_path":"%s/live/herdr.sock"},{"default":false,"name":"hall-dev","running":%s,"session_dir":"%s/hd","socket_path":"%s/hd/herdr.sock"},{"default":false,"name":"hall-dev-2","running":%s,"session_dir":"%s/hd2","socket_path":"%s/hd2/herdr.sock"}]}\n' \
      "$dir" "$dir" "$running" "$dir" "$dir" "$running" "$dir" "$dir"
    ;;
  "workspace create "*)
    n=$(( $(cat "$dir/n" 2>/dev/null || echo 0) + 1 )); echo "$n" >"$dir/n"
    # A pane-id file, if present, is the pane id handed out instead.
    pane=$(cat "$dir/pane-id" 2>/dev/null || echo "w$n:p1")
    printf '{"result":{"workspace":{"workspace_id":"w%s"},"root_pane":{"pane_id":"%s"}}}\n' "$n" "$pane"
    ;;
  "session stop "*) touch "$dir/stopped"; echo '{"result":{}}' ;;
  "agent explain "*) echo '{"state":"idle","evaluated_rules":[{"id":"live_prompt_box","matched":true}]}' ;;
  *) echo '{"result":{}}' ;;
esac
"#;

struct Rig {
    user: PathBuf,
    home: PathBuf,
    bin: PathBuf,
    /// The stand-in the scripts run: the real one unless a test swaps it.
    standin: PathBuf,
}

impl Rig {
    fn new(test: &str) -> Rig {
        let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("scripts-fake-herdr")
            .join(test);
        let _ = std::fs::remove_dir_all(&root);
        let user = root.join("user");
        let bin = root.join("bin");
        std::fs::create_dir_all(user.join(".reeve/state")).unwrap();
        std::fs::write(user.join(".reeve/state/unrelated.meta"), "nothing here\n").unwrap();
        std::fs::create_dir_all(&bin).unwrap();
        let herdr = bin.join("herdr");
        std::fs::write(&herdr, FAKE_HERDR).unwrap();
        std::fs::set_permissions(&herdr, std::fs::Permissions::from_mode(0o755)).unwrap();
        let home = user.join(".reeve-hall-dev");
        // Claude's folder trust: granted for one test reeve's directory only.
        let trusted = home.join("work/ansel");
        std::fs::write(
            user.join(".claude.json"),
            format!(
                r#"{{"projects":{{"{}":{{"hasTrustDialogAccepted":true}}}}}}"#,
                trusted.display()
            ),
        )
        .unwrap();
        Rig {
            user,
            home,
            bin,
            standin: PathBuf::from(env!("CARGO_BIN_EXE_hall-standin")),
        }
    }

    fn run(&self, script: &str, args: &[&str]) -> Output {
        let path = format!("{}:{}", self.bin.display(), std::env::var("PATH").unwrap());
        Command::new("bash")
            .arg(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../scripts")
                    .join(script),
            )
            .arg("--home")
            .arg(&self.home)
            .args(args)
            .env("HOME", &self.user)
            .env("PATH", path)
            .env("HALL_STANDIN_BIN", &self.standin)
            // As inside a live herdr pane: the scripts must not pass these on.
            .env("HERDR_SOCKET_PATH", "/live/herdr.sock")
            .env("HERDR_SESSION", "default")
            .output()
            .unwrap()
    }

    fn ok(&self, script: &str, args: &[&str]) -> String {
        let out = self.run(script, args);
        let stdout = String::from_utf8(out.stdout).unwrap();
        assert!(
            out.status.success(),
            "{script} {args:?}: {}\n{stdout}",
            String::from_utf8_lossy(&out.stderr)
        );
        stdout
    }

    fn calls(&self) -> Vec<String> {
        std::fs::read_to_string(self.bin.join("calls.log"))
            .unwrap_or_default()
            .lines()
            .map(String::from)
            .collect()
    }

    fn listing(&self) -> Value {
        let out = Command::new(env!("CARGO_BIN_EXE_hall-standin"))
            .arg("--home")
            .arg(&self.home)
            .env("HOME", &self.user)
            .output()
            .unwrap();
        assert!(out.status.success(), "{out:?}");
        serde_json::from_slice(&out.stdout).unwrap()
    }
}

#[test]
fn up_status_drive_down() {
    let rig = Rig::new("cycle");
    let up = rig.ok("test-setup", &["up"]);
    assert_eq!(up.matches("work/ansel: accepted").count(), 1, "{up}");
    assert_eq!(up.matches(": NOT accepted").count(), 4, "{up}");
    // Reading trust never grants it.
    let claude_json = std::fs::read_to_string(rig.user.join(".claude.json")).unwrap();
    assert_eq!(claude_json.matches("hasTrustDialogAccepted").count(), 1);

    // Every herdr call had the inherited herdr env removed and named a test session.
    let calls = rig.calls();
    assert!(!calls.is_empty());
    for c in &calls {
        assert!(c.ends_with("|socket=unset|session=unset"), "{c}");
        assert!(!c.contains("default"), "{c}");
    }
    let runs: Vec<&String> = calls
        .iter()
        .filter(|c| c.starts_with("pane run "))
        .collect();
    assert_eq!(runs.len(), 5);
    let h = rig.home.to_str().unwrap();
    for r in &runs {
        assert!(
            r.contains(&format!(
                "REEVE_HOME=\"{h}\" claude --model claude-haiku-4-5-20251001"
            )),
            "{r}"
        );
    }

    // The listing's session locations are the panes herdr handed out.
    let l = rig.listing();
    let ansel = &l["reeves"][0];
    assert_eq!(ansel["name"], "ansel");
    assert_eq!(ansel["session"]["herdr_session"], "hall-dev");
    let pane = ansel["session"]["pane_id"].as_str().unwrap();
    assert!(
        calls
            .iter()
            .any(|c| c.starts_with(&format!("pane run {pane} "))
                && c.contains("--session hall-dev|")),
        "{calls:?}"
    );
    assert_eq!(l["reeves"][1]["session"]["herdr_session"], "hall-dev-2");
    assert_eq!(l["errands"].as_array().unwrap().len(), 3);
    for e in l["errands"].as_array().unwrap() {
        assert!(e["session"]["pane_id"].is_string(), "{e}");
    }
    assert!(rig.home.join("work/ansel-review").is_dir());
    assert!(rig.home.join("standin/live-baseline.json").is_file());

    // A second up finds every session already up and starts nothing new.
    let again = rig.ok("test-setup", &["up"]);
    assert_eq!(again.matches("already up in pane").count(), 5, "{again}");
    assert_eq!(
        rig.calls()
            .iter()
            .filter(|c| c.starts_with("pane run "))
            .count(),
        5
    );

    let status = rig.ok("test-setup", &["status"]);
    assert!(status.contains("unchanged since up's baseline"), "{status}");
    assert!(status.contains("test home: none"), "{status}");
    assert!(status.contains("test names: none"), "{status}");
    assert!(!status.contains("CHECK"), "{status}");
    assert_eq!(
        status.matches("reads as ready for input").count(),
        5,
        "{status}"
    );

    let drive = rig.ok("test-drive", &["ansel", "dialog"]);
    assert!(
        drive.contains(&format!("pane {pane} in hall-dev")),
        "{drive}"
    );
    let turn = rig.ok("test-drive", &["bertil", "turn-end"]);
    assert!(turn.contains("reads as ready for input"), "{turn}");
    // A ready agent is not sent Esc before the question.
    assert!(
        !rig.calls()
            .iter()
            .any(|c| c.starts_with("agent send-keys "))
    );

    rig.ok("test-setup", &["down"]);
    let calls = rig.calls();
    for s in ["hall-dev", "hall-dev-2"] {
        assert!(
            calls
                .iter()
                .any(|c| c.starts_with(&format!("session stop {s} --json|")))
        );
    }
    let l = rig.listing();
    assert_eq!(l["reeves"][0]["session"]["pane_id"], Value::Null);
    assert_eq!(l["reeves"][0]["session"]["herdr_session"], "hall-dev");
}

#[test]
fn status_flags_the_live_reeve_state_mentioning_the_test_setup() {
    let rig = Rig::new("leak");
    rig.ok("test-setup", &["up"]);
    std::fs::write(
        rig.user.join(".reeve/state/phantom.meta"),
        format!("home={}\n", rig.home.display()),
    )
    .unwrap();
    std::fs::write(rig.user.join(".reeve/state/name.meta"), "reeve bertil\n").unwrap();
    let status = rig.ok("test-setup", &["status"]);
    assert!(
        status.contains("test home: ") && status.contains("phantom.meta"),
        "{status}"
    );
    assert!(status.contains("name.meta"), "{status}");
}

#[test]
fn up_refuses_a_pane_id_that_reads_as_an_option() {
    let rig = Rig::new("bad-pane");
    std::fs::write(rig.bin.join("pane-id"), "--session=default").unwrap();
    let out = rig.run("test-setup", &["up"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("refusing pane id"), "{err}");
    let calls = rig.calls();
    assert!(
        calls.iter().any(|c| c.starts_with("workspace create ")),
        "{calls:?}"
    );
    assert!(
        !calls.iter().any(|c| c.starts_with("pane run ")),
        "{calls:?}"
    );
    // Nothing was recorded either.
    for r in rig.listing()["reeves"].as_array().unwrap() {
        assert_eq!(r["session"]["pane_id"], Value::Null, "{r}");
    }
}

#[test]
fn up_refuses_a_symlink_inside_the_home() {
    let rig = Rig::new("inner-link");
    let live = rig.user.join(".reeve");
    for name in ["standin", "work"] {
        let _ = std::fs::remove_dir_all(&rig.home);
        std::fs::create_dir_all(&rig.home).unwrap();
        std::os::unix::fs::symlink(&live, rig.home.join(name)).unwrap();
        let out = rig.run("test-setup", &["up"]);
        assert_eq!(out.status.code(), Some(1), "{out:?}");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(err.contains("is a symlink"), "{err}");
    }
    // No herdr call was made and nothing landed in the live reeve home.
    assert!(rig.calls().is_empty(), "{:?}", rig.calls());
    let mut names: Vec<_> = std::fs::read_dir(&live)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    names.sort();
    assert_eq!(names, ["state"]);
}

#[test]
fn down_and_status_go_on_past_a_dirtied_home() {
    let rig = Rig::new("dirtied");
    rig.ok("test-setup", &["up"]);
    // What `npm install` leaves in a test claude's working directory.
    let bin = rig.home.join("work/ansel/node_modules/.bin");
    std::fs::create_dir_all(&bin).unwrap();
    std::os::unix::fs::symlink("../pkg/cli.js", bin.join("pkg")).unwrap();

    let out = rig.run("test-setup", &["up"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");

    let status = rig.ok("test-setup", &["status"]);
    assert!(status.contains("CHECK: up's baseline not read"), "{status}");
    assert!(
        status.contains("CHECK: recorded panes not read"),
        "{status}"
    );
    assert!(!status.contains("reads as"), "{status}");
    assert!(status.contains("listing: not read"), "{status}");

    let before = rig.calls().len();
    let down = rig.ok("test-setup", &["down"]);
    assert!(down.contains("recorded panes not forgotten"), "{down}");
    let calls = rig.calls();
    for s in ["hall-dev", "hall-dev-2"] {
        assert!(
            calls[before..]
                .iter()
                .any(|c| c.starts_with(&format!("session stop {s} --json|"))),
            "{calls:?}"
        );
    }
    // The world file was left alone: the panes up recorded are still there.
    std::fs::remove_file(bin.join("pkg")).unwrap();
    assert!(rig.listing()["reeves"][0]["session"]["pane_id"].is_string());
}

#[test]
fn up_refuses_to_write_through_a_symlink() {
    let mut rig = Rig::new("write-link");
    // A stand-in whose check-root and check-home pass any home, so up's own check behind it
    // is what is tested; every other command goes to the real stand-in.
    rig.standin = rig.bin.join("lax-standin");
    std::fs::write(
        &rig.standin,
        format!(
            "#!/bin/sh\ncase $1 in check-home | check-root) printf '%s\\n' \"$3\"; exit 0 ;; esac\nexec '{}' \"$@\"\n",
            env!("CARGO_BIN_EXE_hall-standin")
        ),
    )
    .unwrap();
    std::fs::set_permissions(&rig.standin, std::fs::Permissions::from_mode(0o755)).unwrap();
    let live = rig.user.join(".reeve");
    // Dangling links: a plain -e test reads them as absent, and a write follows them.
    for (name, target) in [
        ("standin/world.toml", live.join("world.toml")),
        (
            "standin/live-baseline.json",
            live.join("live-baseline.json"),
        ),
        ("work/ansel", live.join("ansel")),
    ] {
        let _ = std::fs::remove_dir_all(&rig.home);
        std::fs::create_dir_all(rig.home.join("standin")).unwrap();
        std::fs::create_dir_all(rig.home.join("work")).unwrap();
        if name != "standin/world.toml" {
            std::fs::write(rig.home.join("standin/world.toml"), "").unwrap();
        }
        std::os::unix::fs::symlink(&target, rig.home.join(name)).unwrap();
        let out = rig.run("test-setup", &["up"]);
        assert_eq!(out.status.code(), Some(1), "{name}: {out:?}");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(err.contains("refusing to write"), "{name}: {err}");
    }
    // No claude was started and nothing landed in the live reeve home.
    assert!(
        !rig.calls().iter().any(|c| c.starts_with("pane run ")),
        "{:?}",
        rig.calls()
    );
    let mut names: Vec<_> = std::fs::read_dir(&live)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    names.sort();
    assert_eq!(names, ["state"]);
}

#[test]
fn up_stops_when_a_test_session_is_not_running() {
    let rig = Rig::new("not-running");
    std::fs::write(rig.bin.join("stopped"), "").unwrap();
    let out = rig.run("test-setup", &["up"]);
    assert_eq!(out.status.code(), Some(3), "{out:?}");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("env -u HERDR_SOCKET_PATH -u HERDR_SESSION herdr --session hall-dev\n"),
        "{stdout}"
    );
    assert!(
        !rig.calls()
            .iter()
            .any(|c| c.starts_with("workspace create"))
    );
    assert!(!rig.home.exists());
}
