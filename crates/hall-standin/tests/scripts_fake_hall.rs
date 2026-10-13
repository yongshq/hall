//! scripts/test-setup and scripts/test-drive run for real, but against a fake
//! hall binary and a fake HOME, all under target/. This exercises what a dry run
//! cannot: starting the test hall session, reading hall's JSON, seeding the world
//! file, placing each test hand as a tab in its reeve's workspace, recording
//! panes so the stand-in's session locations match, and the status checks. A
//! fake `herdr` sits first on PATH and records any call: there must be none
//! (decision 33). No real hall or herdr server, reeve home or claude is involved.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

/// Logs each call (and whether the HALL_* names hall reads leaked through) and
/// answers like hall does for the calls the scripts make. `--session <s>
/// server` marks the session running and exits, as a detached server would
/// return control; `session stop` marks it stopped.
const FAKE_HALL: &str = r#"#!/usr/bin/env bash
set -euo pipefail
dir=$(dirname "$0")
printf '%s|socket=%s|session=%s|pane=%s|env=%s|config=%s|client=%s\n' "$*" \
  "${HALL_SOCKET_PATH-unset}" "${HALL_SESSION-unset}" "${HALL_PANE_ID-unset}" \
  "${HALL_ENV-unset}" "${HALL_CONFIG_PATH-unset}" "${HALL_CLIENT_SOCKET_PATH-unset}" >>"$dir/calls.log"
next() { n=$(( $(cat "$dir/n" 2>/dev/null || echo 0) + 1 )); echo "$n" >"$dir/n"; }
case "$*" in
  "--session hall-dev server")
    [ -e "$dir/no-start" ] || touch "$dir/running"
    ;;
  "session list --json")
    if [ -e "$dir/running" ]; then running=true; else running=false; fi
    printf '{"sessions":[{"default":true,"name":"default","running":false,"session_dir":"%s/cfg","socket_path":"%s/cfg/herdr.sock"},{"default":false,"name":"hall-dev","running":%s,"session_dir":"%s/cfg/sessions/hall-dev","socket_path":"%s/cfg/sessions/hall-dev/herdr.sock"}]}\n' \
      "$dir" "$dir" "$running" "$dir" "$dir"
    ;;
  "workspace create "*)
    next
    # A pane-id file, if present, is the pane id handed out instead.
    pane=$(cat "$dir/pane-id" 2>/dev/null || echo "w$n:p1")
    echo "$pane w$n w$n:t1" >>"$dir/panes"
    printf '{"result":{"type":"workspace_created","workspace":{"workspace_id":"w%s"},"tab":{"tab_id":"w%s:t1"},"root_pane":{"pane_id":"%s","workspace_id":"w%s","tab_id":"w%s:t1"}}}\n' "$n" "$n" "$pane" "$n" "$n"
    ;;
  "tab create --workspace "*)
    next
    ws=$4
    pane="$ws:t$n:p1"
    echo "$pane $ws $ws:t$n" >>"$dir/panes"
    printf '{"result":{"type":"tab_created","tab":{"tab_id":"%s:t%s"},"root_pane":{"pane_id":"%s","workspace_id":"%s","tab_id":"%s:t%s"}}}\n' "$ws" "$n" "$pane" "$ws" "$ws" "$n"
    ;;
  "pane get "*)
    read -r _ ws tab < <(grep "^$3 " "$dir/panes") || exit 1
    printf '{"result":{"type":"pane_info","pane":{"pane_id":"%s","workspace_id":"%s","tab_id":"%s"}}}\n' "$3" "$ws" "$tab"
    ;;
  "session stop "*) rm -f "$dir/running"; echo '{"result":{}}' ;;
  "agent explain "*) echo '{"state":"idle","evaluated_rules":[{"id":"live_prompt_box","matched":true}]}' ;;
  *) echo '{"result":{}}' ;;
esac
"#;

/// Any herdr call at all is a failure: this one only records it.
const FAKE_HERDR: &str = r#"#!/bin/sh
printf '%s\n' "$*" >>"$(dirname "$0")/herdr-calls.log"
exit 1
"#;

struct Rig {
    user: PathBuf,
    home: PathBuf,
    bin: PathBuf,
    var_tmp: PathBuf,
    /// The stand-in the scripts run: the real one unless a test swaps it.
    standin: PathBuf,
}

fn executable(path: &Path, text: &str) {
    std::fs::write(path, text).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

impl Rig {
    fn new(test: &str) -> Rig {
        let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("scripts-fake-hall")
            .join(test);
        let _ = std::fs::remove_dir_all(&root);
        let user = root.join("user");
        let bin = root.join("bin");
        let var_tmp = root.join("var-tmp");
        std::fs::create_dir_all(user.join(".reeve/state")).unwrap();
        std::fs::write(user.join(".reeve/state/unrelated.meta"), "nothing here\n").unwrap();
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::create_dir_all(&var_tmp).unwrap();
        executable(&bin.join("hall"), FAKE_HALL);
        executable(&bin.join("herdr"), FAKE_HERDR);
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
            var_tmp,
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
            .env("HALL_DEBUG_BIN", self.bin.join("hall"))
            // scripts/live-check, which status runs, reads this instead of /var/tmp.
            .env("HALL_LIVE_CHECK_VAR_TMP", &self.var_tmp)
            // As inside a hall pane, and a live herdr pane before it: the
            // scripts must not pass the HALL_* names on.
            .env("HALL_SOCKET_PATH", "/live/hall.sock")
            .env("HALL_SESSION", "default")
            .env("HALL_PANE_ID", "w9:p9")
            .env("HALL_ENV", "1")
            .env("HALL_CONFIG_PATH", "/live/config.toml")
            .env("HALL_CLIENT_SOCKET_PATH", "/live/hall-client.sock")
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

    fn herdr_calls(&self) -> String {
        std::fs::read_to_string(self.bin.join("herdr-calls.log")).unwrap_or_default()
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

    /// The workspace and tab the fake hall placed `pane` in.
    fn place_of(&self, pane: &str) -> (String, String) {
        let panes = std::fs::read_to_string(self.bin.join("panes")).unwrap();
        let line = panes
            .lines()
            .find(|l| l.starts_with(&format!("{pane} ")))
            .unwrap_or_else(|| panic!("{pane} not in {panes}"));
        let f: Vec<&str> = line.split(' ').collect();
        (f[1].to_string(), f[2].to_string())
    }
}

const CLEAN: &str = "|socket=unset|session=unset|pane=unset|env=unset|config=unset|client=unset";

#[test]
fn up_status_drive_down() {
    let rig = Rig::new("cycle");
    let up = rig.ok("test-setup", &["up"]);
    assert_eq!(up.matches("work/ansel: accepted").count(), 1, "{up}");
    assert_eq!(up.matches(": NOT accepted").count(), 4, "{up}");
    // Reading trust never grants it.
    let claude_json = std::fs::read_to_string(rig.user.join(".claude.json")).unwrap();
    assert_eq!(claude_json.matches("hasTrustDialogAccepted").count(), 1);

    // up started the test session itself, detached, since it was not running.
    let calls = rig.calls();
    assert!(
        calls
            .iter()
            .any(|c| c == &format!("--session hall-dev server{CLEAN}")),
        "{calls:?}"
    );
    // Every hall call had the inherited HALL_* names removed and named the test
    // session; no herdr call was made at all.
    assert!(!calls.is_empty());
    for c in &calls {
        assert!(c.ends_with(CLEAN), "{c}");
        assert!(!c.contains("default"), "{c}");
    }
    assert_eq!(rig.herdr_calls(), "");

    let runs: Vec<&String> = calls
        .iter()
        .filter(|c| c.starts_with("pane run "))
        .collect();
    assert_eq!(runs.len(), 5);
    let h = rig.home.to_str().unwrap();
    let socket = format!("{}/cfg/sessions/hall-dev/herdr.sock", rig.bin.display());
    for r in &runs {
        assert!(
            r.contains(&format!(
                "REEVE_HOME=\"{h}\" XDG_CONFIG_HOME=\"{h}/xdg-config\" HALL_SOCKET_PATH=\"{socket}\" claude --model claude-haiku-4-5-20251001"
            )),
            "{r}"
        );
    }
    assert!(rig.home.join("xdg-config").is_dir());
    assert!(!rig.home.join("standin/live-baseline.json").exists());

    // The listing's session locations are the panes hall handed out: each test
    // reeve in a workspace of its own, each test hand a tab in its reeve's.
    let l = rig.listing();
    let reeve_ws = |i: usize| {
        let r = &l["reeves"][i];
        assert_eq!(r["session"]["herdr_session"], "hall-dev", "{r}");
        let pane = r["session"]["pane_id"].as_str().unwrap();
        assert!(
            calls
                .iter()
                .any(|c| c.starts_with(&format!("pane run {pane} "))
                    && c.contains("--session hall-dev|")),
            "{calls:?}"
        );
        (r["name"].as_str().unwrap().to_string(), rig.place_of(pane))
    };
    let (ansel, (ansel_ws, ansel_tab)) = reeve_ws(0);
    let (bertil, (bertil_ws, _)) = reeve_ws(1);
    assert_eq!((ansel.as_str(), bertil.as_str()), ("ansel", "bertil"));
    assert_ne!(ansel_ws, bertil_ws);
    let errands = l["errands"].as_array().unwrap();
    assert_eq!(errands.len(), 3);
    for e in errands {
        assert_eq!(e["session"]["herdr_session"], "hall-dev", "{e}");
        let pane = e["session"]["pane_id"].as_str().unwrap();
        let (ws, tab) = rig.place_of(pane);
        let owner_ws = if e["owner_reeve"] == "ansel" {
            &ansel_ws
        } else {
            &bertil_ws
        };
        assert_eq!(&ws, owner_ws, "{e}");
        assert_ne!(tab, ansel_tab, "{e}");
        assert!(
            calls.iter().any(|c| c.starts_with(&format!(
                "tab create --workspace {owner_ws} --cwd {h}/work/{} --label {} --no-focus --session hall-dev|",
                e["id"].as_str().unwrap(),
                e["id"].as_str().unwrap()
            ))),
            "{e}: {calls:?}"
        );
    }
    assert_eq!(
        calls
            .iter()
            .filter(|c| c.starts_with("workspace create "))
            .count(),
        2
    );
    assert!(rig.home.join("work/ansel-review").is_dir());

    // A second up finds the session and every claude already up and starts nothing new.
    let again = rig.ok("test-setup", &["up"]);
    assert_eq!(again.matches("already up in pane").count(), 5, "{again}");
    let calls = rig.calls();
    assert_eq!(
        calls.iter().filter(|c| c.starts_with("pane run ")).count(),
        5
    );
    assert_eq!(
        calls
            .iter()
            .filter(|c| c.ends_with("server") || c.contains(" server|"))
            .count(),
        1
    );

    let status = rig.ok("test-setup", &["status"]);
    assert!(!status.contains("CHECK"), "{status}");
    assert_eq!(
        status.matches("reads as ready for input").count(),
        5,
        "{status}"
    );
    assert!(
        status.contains(&format!("ansel-fix: pane {ansel_ws}:")),
        "{status}"
    );
    for part in [
        "config",
        "state",
        "worktrees",
        "pane-graphics",
        "peers",
        "reeve-state",
    ] {
        assert!(status.contains(&format!("{part}: pass")), "{status}");
    }

    let drive = rig.ok("test-drive", &["ansel", "dialog"]);
    let ansel_pane = l["reeves"][0]["session"]["pane_id"].as_str().unwrap();
    assert!(
        drive.contains(&format!("pane {ansel_pane} in hall-dev")),
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
    assert!(
        calls
            .iter()
            .any(|c| c.starts_with("session stop hall-dev --json|"))
    );
    for c in &calls {
        assert!(c.ends_with(CLEAN), "{c}");
    }
    assert_eq!(rig.herdr_calls(), "");
    let l = rig.listing();
    assert_eq!(l["reeves"][0]["session"]["pane_id"], Value::Null);
    assert_eq!(l["reeves"][0]["session"]["herdr_session"], "hall-dev");
}

#[test]
fn status_fails_on_live_reeve_state_mentioning_the_test_setup() {
    let rig = Rig::new("leak");
    rig.ok("test-setup", &["up"]);
    std::fs::write(
        rig.user.join(".reeve/state/phantom.meta"),
        format!("home={}\n", rig.home.display()),
    )
    .unwrap();
    std::fs::write(rig.user.join(".reeve/state/name.meta"), "reeve bertil\n").unwrap();
    let out = rig.run("test-setup", &["status"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stdout.contains("reeve-state: FAIL 2 files"), "{stdout}");
    assert!(
        stdout.contains("CHECK: scripts/live-check failed"),
        "{stdout}"
    );
    assert!(stderr.contains("phantom.meta"), "{stderr}");
    assert!(stderr.contains("name.meta"), "{stderr}");
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
    // No hall call was made and nothing landed in the live reeve home.
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
    assert!(
        calls[before..]
            .iter()
            .any(|c| c.starts_with("session stop hall-dev --json|")),
        "{calls:?}"
    );
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
    executable(
        &rig.standin,
        &format!(
            "#!/bin/sh\ncase $1 in check-home | check-root) printf '%s\\n' \"$3\"; exit 0 ;; esac\nexec '{}' \"$@\"\n",
            env!("CARGO_BIN_EXE_hall-standin")
        ),
    );
    let live = rig.user.join(".reeve");
    // Dangling links: a plain -e test reads them as absent, and a write follows them.
    for (name, target) in [
        ("standin/world.toml", live.join("world.toml")),
        ("xdg-config", live.join("xdg-config")),
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
fn up_keeps_a_running_test_session() {
    let rig = Rig::new("running");
    std::fs::write(rig.bin.join("running"), "").unwrap();
    let up = rig.ok("test-setup", &["up"]);
    assert!(up.contains("hall session hall-dev is running"), "{up}");
    assert!(
        !rig.calls().iter().any(|c| c.contains(" server|")),
        "{:?}",
        rig.calls()
    );
    assert_eq!(rig.herdr_calls(), "");
}

#[test]
fn up_refuses_an_old_roster_with_session_numbers() {
    let rig = Rig::new("old-roster");
    let conf = rig.bin.join("old.conf");
    std::fs::write(
        &conf,
        "HALL_DEV_HOME=\"$HOME/.reeve-hall-dev\"\nHALL_DEV_SESSION=hall-dev\nHALL_DEV_ROSTER=(\"reeve ansel\" \"errand ansel-fix 1 ansel artificer\")\n",
    )
    .unwrap();
    let out = rig.run("test-setup", &["--config", conf.to_str().unwrap(), "up"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("no session number since step 2"), "{err}");
    assert!(rig.calls().is_empty(), "{:?}", rig.calls());
}
