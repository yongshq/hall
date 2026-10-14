//! scripts/test-setup and scripts/test-drive run for real, but against a fake
//! hall binary and a fake HOME, all under target/. This exercises what a dry run
//! cannot: starting the test hall session, reading hall's JSON, seeding the world
//! file, placing each test hand as a tab in its reeve's workspace, recording
//! panes so the stand-in's session locations match, and the status checks. A
//! fake `herdr` sits first on PATH and records any call: there must be none
//! (decision 33). A fake `claude` beside it records the environment each claude
//! line gives it (decision 48). No real hall or herdr server, reeve home or
//! claude is involved.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

/// Logs each call (and whether the HALL_* names hall reads leaked through) and
/// answers like hall does for the calls the scripts make. `--session <s>
/// server` logs the identity names it was started with (decision 48) and its
/// REEVE_HOME, which every tab it opens holds (a hand a test reeve dispatches
/// among them), marks
/// the session running and exits, as a detached server would return control;
/// `session stop` marks it stopped. `pane run` runs the line in a shell, with
/// the HERDR_* and HALL_* names and REEVE_BACKEND hall sets into a pane, as a
/// pane's shell would, and with
/// the names in a `server-env` file, if present, as a pane of a hall server
/// someone else started would hold them.
const FAKE_HALL: &str = r#"#!/usr/bin/env bash
set -euo pipefail
dir=$(dirname "$0")
printf '%s|socket=%s|session=%s|pane=%s|env=%s|config=%s|client=%s|bin=%s|tab=%s|ws=%s|backend=%s\n' "$*" \
  "${HALL_SOCKET_PATH-unset}" "${HALL_SESSION-unset}" "${HALL_PANE_ID-unset}" \
  "${HALL_ENV-unset}" "${HALL_CONFIG_PATH-unset}" "${HALL_CLIENT_SOCKET_PATH-unset}" \
  "${HALL_BIN_PATH-unset}" "${HALL_TAB_ID-unset}" "${HALL_WORKSPACE_ID-unset}" \
  "${REEVE_BACKEND-unset}" >>"$dir/calls.log"
next() { n=$(( $(cat "$dir/n" 2>/dev/null || echo 0) + 1 )); echo "$n" >"$dir/n"; }
case "$*" in
  "--session hall-dev server")
    env | sed -nE 's/^((REEVE_HAND|REEVE_HALL_[A-Z_]*|CLAUDECODE|CLAUDE_[A-Z_]*|HERDR_[A-Z_]*)=.*)/\1/p' | sort >>"$dir/server-env.log"
    printf '%s\n' "${REEVE_HOME-unset}" >>"$dir/server-reeve-home.log"
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
  "pane run "*)
    if [ -e "$dir/server-env" ]; then set -a; . "$dir/server-env"; set +a; fi
    read -r _ ws tab < <(grep "^$3 " "$dir/panes")
    sock="$dir/cfg/sessions/hall-dev/herdr.sock"
    HERDR_ENV=1 HERDR_PANE_ID=$3 HERDR_TAB_ID=$tab HERDR_WORKSPACE_ID=$ws HERDR_BIN_PATH=$0 \
      HERDR_SOCKET_PATH=$sock HALL_ENV=1 HALL_SESSION=hall-dev HALL_PANE_ID=$3 HALL_TAB_ID=$tab \
      HALL_WORKSPACE_ID=$ws HALL_BIN_PATH=$0 HALL_SOCKET_PATH=$sock REEVE_BACKEND=hall bash -c "$4"
    echo '{"result":{}}'
    ;;
  "session stop "*) rm -f "$dir/running"; echo '{"result":{}}' ;;
  # A blocked file stands for a dialog, which Esc clears.
  "agent explain "*)
    if [ -e "$dir/blocked" ]; then
      echo '{"state":"blocked","evaluated_rules":[]}'
    else
      echo '{"state":"idle","evaluated_rules":[{"id":"live_prompt_box","matched":true}]}'
    fi
    ;;
  "agent send-keys "*" esc --session "*) rm -f "$dir/blocked"; echo '{"result":{}}' ;;
  *) echo '{"result":{}}' ;;
esac
"#;

/// Records, per claude line it is started from, its working directory's name,
/// the names that say who it is or where it writes, the six HERDR_* names
/// hall sets into a pane, and three of hall's own: REEVE_BACKEND, HALL_BIN_PATH
/// and HALL_PANE_ID.
const FAKE_CLAUDE: &str = r#"#!/usr/bin/env bash
printf '%s|hand=%s|ident=%s|reeve_home=%s|xdg_config=%s|xdg_state=%s|hall_socket=%s|herdr_pane=%s|herdr=%s,%s,%s,%s,%s|hall=%s,%s,%s\n' \
  "$(basename "$PWD")" "${REEVE_HAND-unset}" \
  "$(env | sed -nE 's/^((REEVE_HALL_[A-Z_]*|CLAUDECODE|CLAUDE_[A-Z_]*|HERDR_[A-Z_]*)=.*)/\1/p' | grep -v -E '^HERDR_(ENV|SOCKET_PATH|BIN_PATH|WORKSPACE_ID|TAB_ID|PANE_ID)=' | sort | tr '\n' ' ')" \
  "${REEVE_HOME-unset}" "${XDG_CONFIG_HOME-unset}" "${XDG_STATE_HOME-unset}" "${HALL_SOCKET_PATH-unset}" \
  "${HERDR_PANE_ID-unset}" "${HERDR_ENV-unset}" "${HERDR_SOCKET_PATH-unset}" "${HERDR_BIN_PATH-unset}" \
  "${HERDR_WORKSPACE_ID-unset}" "${HERDR_TAB_ID-unset}" \
  "${REEVE_BACKEND-unset}" "${HALL_BIN_PATH-unset}" "${HALL_PANE_ID-unset}" >>"$(dirname "$0")/claude.log"
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
        executable(&bin.join("claude"), FAKE_CLAUDE);
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
        self.run_with(script, args, &[])
    }

    fn run_with(&self, script: &str, args: &[&str], envs: &[(&str, &str)]) -> Output {
        let path = format!("{}:{}", self.bin.display(), std::env::var("PATH").unwrap());
        let mut c = Command::new("bash");
        // Who runs the tests does not matter: their own identity names go.
        for (k, _) in std::env::vars() {
            if k.starts_with("CLAUDE")
                || k.starts_with("HERDR_")
                || k.starts_with("REEVE_HALL_")
                || k == "REEVE_HAND"
            {
                c.env_remove(k);
            }
        }
        c.arg(
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
        .env("HALL_BIN_PATH", "/live/hall")
        .env("HALL_TAB_ID", "w9:t9")
        .env("HALL_WORKSPACE_ID", "w9")
        .env("REEVE_BACKEND", "hall")
        .env("HERDR_SOCKET_PATH", "/live/herdr.sock")
        .env("HERDR_SESSION", "default")
        .env("HERDR_PANE_ID", "w9:p9")
        // As run from a live claude, itself a hand (decision 48): none of
        // this may reach the test hall server or a test claude.
        .env("REEVE_HAND", "starter-hand")
        // A starter's own reeve home, the live one: the test hall server must
        // carry the test home instead.
        .env("REEVE_HOME", self.user.join(".reeve"))
        // Reeve's hall backend overrides, read before hall's own names: a
        // test reeve's dispatches would follow them to the live hall.
        .env("REEVE_HALL_SOCKET", "/live/hall.sock")
        .env("REEVE_HALL_SESSION", "default")
        .env("REEVE_HALL_BIN", "/live/hall")
        .env("CLAUDECODE", "1")
        .env("CLAUDE_PID", "4242")
        .env("CLAUDE_EFFORT", "high")
        .env("CLAUDE_CODE_SESSION_ID", "starter-session")
        .env("CLAUDE_CODE_MESSAGING_SOCKET", "/live/cc.sock")
        .env("CLAUDE_CODE_MESSAGING_TOKEN", "starter-token")
        .envs(envs.iter().copied())
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

    fn read_bin(&self, name: &str) -> String {
        std::fs::read_to_string(self.bin.join(name)).unwrap_or_default()
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

const CLEAN: &str = "|socket=unset|session=unset|pane=unset|env=unset|config=unset|client=unset|bin=unset|tab=unset|ws=unset|backend=unset";

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
                "REEVE_HOME=\"{h}\" XDG_CONFIG_HOME=\"{h}/xdg-config\" XDG_STATE_HOME=\"{h}/xdg-state\" HALL_SOCKET_PATH=\"{socket}\" claude --model claude-haiku-4-5-20251001"
            )),
            "{r}"
        );
    }
    assert!(rig.home.join("xdg-config").is_dir());
    assert!(rig.home.join("xdg-state").is_dir());
    // The hall client config that turns the reeve tree on names the stand-in's
    // listing of this home (decision 49), and up says to attach with it.
    let client_config = std::fs::read_to_string(rig.home.join("standin/hall.toml")).unwrap();
    assert!(
        client_config.contains(&format!(
            "[reeve]\nlisting_command = [\"{}\", \"--home\", \"{h}\"]\n",
            rig.standin.display()
        )),
        "{client_config}"
    );
    assert!(
        up.contains(&format!("HALL_CONFIG_PATH={h}/standin/hall.toml")),
        "{up}"
    );

    // The test hall server started with none of the starter's identity names
    // (decision 48).
    assert_eq!(rig.read_bin("server-env.log"), "");
    // It carries the test home as REEVE_HOME, not the starter's: a hand a test
    // reeve dispatches opens in a tab with the server's environment, from a
    // launch line that names no REEVE_HOME (finding F1 of step5-shown-working).
    assert_eq!(
        rig.read_bin("server-reeve-home.log"),
        format!("{}\n", rig.home.canonicalize().unwrap().display())
    );
    // Each claude line, run as a pane's shell runs it, gave claude none of them
    // either, though the shell had them all: a test hand has its own errand id
    // as REEVE_HAND, a test reeve none; each has the test home's folders and
    // the session's socket, and the pane id hall set.
    let claudes = rig.read_bin("claude.log");
    let mut seen: Vec<&str> = claudes.lines().collect();
    seen.sort();
    let pane_of = |name: &str| {
        let world = rig.listing();
        world["reeves"]
            .as_array()
            .unwrap()
            .iter()
            .chain(world["errands"].as_array().unwrap())
            .find(|e| e["name"] == name || e["id"] == name)
            .map(|e| e["session"]["pane_id"].as_str().unwrap().to_string())
            .unwrap()
    };
    let mut want: Vec<String> = ["ansel", "ansel-fix", "ansel-review", "bertil", "bertil-probe"]
        .iter()
        .map(|name| {
            let hand = if name.contains('-') { name } else { "unset" };
            let pane = pane_of(name);
            let (ws, tab) = rig.place_of(&pane);
            format!(
                "{name}|hand={hand}|ident=|reeve_home={h}|xdg_config={h}/xdg-config|xdg_state={h}/xdg-state|hall_socket={socket}|herdr_pane={pane}|herdr=1,{socket},{bin},{ws},{tab}|hall=hall,{bin},{pane}",
                bin = rig.bin.join("hall").display()
            )
        })
        .collect();
    want.sort();
    assert_eq!(seen, want);
    assert!(!rig.home.join("standin/live-baseline.json").exists());

    // The listing's session locations are the panes hall handed out: each test
    // reeve in a workspace of its own, each test hand a tab in its reeve's.
    let l = rig.listing();
    let reeve_ws = |i: usize| {
        let r = &l["reeves"][i];
        assert_eq!(r["session"]["backend"], "hall", "{r}");
        assert_eq!(r["session"]["name"], "hall-dev", "{r}");
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
        assert_eq!(e["session"]["backend"], "hall", "{e}");
        assert_eq!(e["session"]["name"], "hall-dev", "{e}");
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

    // A standing dialog is denied with Esc, and the question waits until hall
    // no longer reads it, since hall refuses a prompt to a blocked agent.
    std::fs::write(rig.bin.join("blocked"), "").unwrap();
    let before = rig.calls().len();
    let turn = rig.ok("test-drive", &["ansel", "turn-end"]);
    assert!(turn.contains("reads as at a dialog"), "{turn}");
    assert!(turn.contains("reads as ready for input"), "{turn}");
    let calls = rig.calls();
    let drive: Vec<&str> = calls[before..]
        .iter()
        .map(|c| c.split(' ').take(2).collect::<Vec<_>>().join(" "))
        .filter(|c| c != "agent explain")
        .map(|c| match c.as_str() {
            "agent send-keys" => "esc",
            "agent wait" => "wait",
            "agent prompt" => "prompt",
            _ => "other",
        })
        .collect();
    assert_eq!(drive, ["esc", "wait", "prompt"], "{calls:?}");

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
    assert_eq!(l["reeves"][0]["session"]["name"], "hall-dev");
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
fn up_and_drive_go_on_past_a_link_inside_the_xdg_folders() {
    let rig = Rig::new("xdg-links");
    rig.ok("test-setup", &["up"]);
    // What fnm makes per shell under a test claude's XDG_STATE_HOME: a link
    // out of the home. The stand-in walks only herdr's folder inside either
    // folder (decision 48's amendments).
    for name in ["xdg-config", "xdg-state"] {
        let dir = rig.home.join(name).join("fnm_multishells");
        std::fs::create_dir_all(&dir).unwrap();
        std::os::unix::fs::symlink(rig.user.join(".reeve"), dir.join("1_2")).unwrap();
    }
    let again = rig.ok("test-setup", &["up"]);
    assert_eq!(again.matches("already up in pane").count(), 5, "{again}");
    let turn = rig.ok("test-drive", &["ansel", "turn-end"]);
    assert!(turn.contains("reads as ready for input"), "{turn}");

    // Either folder itself a link is still refused.
    for name in ["xdg-config", "xdg-state"] {
        let dir = rig.home.join(name);
        std::fs::rename(&dir, rig.bin.join(name)).unwrap();
        std::os::unix::fs::symlink(rig.bin.join(name), &dir).unwrap();
        for (script, args) in [
            ("test-setup", &["up"][..]),
            ("test-drive", &["ansel", "turn-end"]),
        ] {
            let out = rig.run(script, args);
            assert_eq!(out.status.code(), Some(1), "{name} {script}: {out:?}");
            let err = String::from_utf8_lossy(&out.stderr);
            assert!(err.contains("is a symlink"), "{name} {script}: {err}");
        }
        std::fs::remove_file(&dir).unwrap();
        std::fs::rename(rig.bin.join(name), &dir).unwrap();
    }
}

#[test]
fn a_claude_line_drops_what_a_server_up_did_not_start_holds() {
    let rig = Rig::new("foreign-server");
    // A hall-dev server someone else started, from a claude: its panes hold
    // names the runner of up lacks (decision 48's second amendment).
    std::fs::write(
        rig.bin.join("server-env"),
        "REEVE_HAND=server-hand\nREEVE_HALL_SOCKET=/other/hall.sock\nREEVE_HALL_SESSION=other\nCLAUDECODE=1\nCLAUDE_PID=7\nCLAUDE_EFFORT=low\nCLAUDE_CODE_CHILD_SESSION=1\nCLAUDE_CODE_MESSAGING_SOCKET=/other/cc.sock\nCLAUDE_CODE_MESSAGING_TOKEN=server-token\nHERDR_SESSION=other\nHERDR_CLIENT_SOCKET_PATH=/other/client.sock\n",
    )
    .unwrap();
    rig.ok("test-setup", &["up"]);
    // The lines name none of them: each pane's shell works out its own drop.
    let runs: Vec<String> = rig
        .calls()
        .into_iter()
        .filter(|c| c.starts_with("pane run "))
        .collect();
    assert_eq!(runs.len(), 5);
    for r in &runs {
        assert!(!r.contains("CHILD_SESSION"), "{r}");
        assert!(!r.contains("-u HERDR_"), "{r}");
    }
    // No claude got any of them; a test hand has its own errand id, a test
    // reeve none, and each keeps the six names hall set into its pane.
    let claudes = rig.read_bin("claude.log");
    assert_eq!(claudes.lines().count(), 5, "{claudes}");
    for line in claudes.lines() {
        let name = line.split('|').next().unwrap();
        let hand = if name.contains('-') { name } else { "unset" };
        assert!(
            line.starts_with(&format!("{name}|hand={hand}|ident=|")),
            "{line}"
        );
        assert!(!line.contains("server-hand"), "{line}");
        assert!(!line.contains(",unset"), "{line}");
    }
}

#[test]
fn up_and_drive_refuse_a_link_in_herdrs_folder_inside_the_xdg_folders() {
    let rig = Rig::new("xdg-herdr-links");
    rig.ok("test-setup", &["up"]);
    let live = rig.user.join(".config/herdr");
    std::fs::create_dir_all(&live).unwrap();
    std::fs::write(live.join("endpoints.json"), "").unwrap();
    // Herdr's own folder under a test claude's XDG_CONFIG_HOME or XDG_STATE_HOME
    // is walked in full (decision 48's second amendment): herdr's folder itself
    // a link to the live herdr's, or a hard link to a live herdr file in it.
    for name in ["xdg-config", "xdg-state"] {
        let herdr = rig.home.join(name).join("herdr");
        let _ = std::fs::remove_dir_all(&herdr);
        std::os::unix::fs::symlink(&live, &herdr).unwrap();
        for (script, args) in [
            ("test-setup", &["up"][..]),
            ("test-drive", &["ansel", "turn-end"]),
        ] {
            let out = rig.run(script, args);
            assert_eq!(out.status.code(), Some(1), "{name} {script}: {out:?}");
            let err = String::from_utf8_lossy(&out.stderr);
            assert!(err.contains("herdr is a symlink"), "{name} {script}: {err}");
        }
        std::fs::remove_file(&herdr).unwrap();

        std::fs::create_dir(&herdr).unwrap();
        std::fs::hard_link(live.join("endpoints.json"), herdr.join("endpoints.json")).unwrap();
        for (script, args) in [
            ("test-setup", &["up"][..]),
            ("test-drive", &["ansel", "turn-end"]),
        ] {
            let out = rig.run(script, args);
            assert_eq!(out.status.code(), Some(1), "{name} {script}: {out:?}");
            let err = String::from_utf8_lossy(&out.stderr);
            assert!(
                err.contains("herdr/endpoints.json is a hard link"),
                "{name} {script}: {err}"
            );
        }
        std::fs::remove_dir_all(&herdr).unwrap();

        // A case variant is herdr's folder on a case-insensitive filesystem.
        let cased = rig.home.join(name).join("Herdr");
        std::os::unix::fs::symlink(&live, &cased).unwrap();
        for (script, args) in [
            ("test-setup", &["up"][..]),
            ("test-drive", &["ansel", "turn-end"]),
        ] {
            let out = rig.run(script, args);
            assert_eq!(out.status.code(), Some(1), "{name} {script}: {out:?}");
            let err = String::from_utf8_lossy(&out.stderr);
            assert!(err.contains("Herdr is a symlink"), "{name} {script}: {err}");
        }
        std::fs::remove_file(&cased).unwrap();
    }
    // With herdr's folders plain again, both go on.
    let again = rig.ok("test-setup", &["up"]);
    assert_eq!(again.matches("already up in pane").count(), 5, "{again}");
    rig.ok("test-drive", &["ansel", "turn-end"]);
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
fn up_names_the_session_folder_when_the_session_does_not_start() {
    let rig = Rig::new("no-start");
    std::fs::write(rig.bin.join("no-start"), "").unwrap();
    let out = rig.run_with("test-setup", &["up"], &[("HALL_START_WAIT", "3")]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains(&format!(
            "hall session hall-dev did not start within 0.3s; its server log is in its session folder, {}/cfg/sessions/hall-dev\n",
            rig.bin.display()
        )),
        "{err}"
    );
    assert!(
        !rig.calls().iter().any(|c| c.starts_with("pane run ")),
        "{:?}",
        rig.calls()
    );
    assert_eq!(rig.read_bin("claude.log"), "");
    assert_eq!(rig.herdr_calls(), "");
}

#[test]
fn up_refuses_an_old_roster_with_session_numbers() {
    let rig = Rig::new("old-roster");
    let conf = rig.bin.join("old.conf");
    std::fs::write(
        &conf,
        "HALL_DEV_HOME=\"$HOME/.reeve-hall-dev\"\nHALL_DEV_SESSION=hall-dev\nHALL_DEV_LISTING=standin\nHALL_DEV_ROSTER=(\"reeve ansel\" \"errand ansel-fix 1 ansel artificer\")\n",
    )
    .unwrap();
    let out = rig.run("test-setup", &["--config", conf.to_str().unwrap(), "up"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("no session number since step 2"), "{err}");
    assert!(rig.calls().is_empty(), "{:?}", rig.calls());
}

/// Stands in for reeve's own listing command (step 5): logs its arguments and
/// prints a version 1 listing with the test reeve, a hand it dispatched in
/// hall-dev, and a herdr entry under the same id, which no script may use.
const FAKE_REEVE_LISTING: &str = r#"#!/bin/sh
printf '%s\n' "$*" >>"$(dirname "$0")/reeve-listing.log"
cat <<'JSON'
{"version": 1,
 "reeves": [{"name": "ansel", "session": {"backend": "hall", "name": "hall-dev", "pane_id": "w1:p1"}, "live": true}],
 "errands": [
  {"id": "scout-1", "owner_reeve": "ansel", "hand_state": "working",
   "session": {"backend": "herdr", "name": null, "pane_id": "w7:p7"}},
  {"id": "scout-1", "owner_reeve": "ansel", "hand_state": "needs-decision", "question": "Which?", "open": 1,
   "session": {"backend": "hall", "name": "hall-dev", "pane_id": "w1:t5:p1"}}
 ]}
JSON
"#;

#[test]
fn the_reeve_listing_starts_one_test_reeve_and_drives_a_hand_it_dispatched() {
    let rig = Rig::new("reeve-listing");
    let h = rig.home.to_str().unwrap();

    // Before reeve's listing command is there, up refuses before any hall call.
    let out = rig.run("test-setup", &["--listing", "reeve", "up"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("use --listing standin until it lands"),
        "{out:?}"
    );
    assert!(rig.calls().is_empty(), "{:?}", rig.calls());

    let listing_bin = rig.user.join("yongshq/reeve/bin");
    std::fs::create_dir_all(&listing_bin).unwrap();
    let listing = listing_bin.join("reeve-listing");
    executable(&listing, FAKE_REEVE_LISTING);

    let up = rig.ok("test-setup", &["--listing", "reeve", "up"]);
    let calls = rig.calls();
    for c in &calls {
        assert!(c.ends_with(CLEAN), "{c}");
    }
    assert_eq!(rig.herdr_calls(), "");
    let count = |prefix: &str| calls.iter().filter(|c| c.starts_with(prefix)).count();
    assert_eq!(count("workspace create "), 1, "{calls:?}");
    assert_eq!(count("tab create "), 0, "{calls:?}");
    assert_eq!(count("pane run "), 1, "{calls:?}");
    // The hands the test reeve dispatches here open in tabs with the server's
    // environment: it carries the test home as REEVE_HOME.
    assert_eq!(
        rig.read_bin("server-reeve-home.log"),
        format!("{}\n", rig.home.canonicalize().unwrap().display())
    );
    // The one test reeve's claude keeps hall's pane names: reeve's hall backend
    // reads them (decision 68).
    let claudes = rig.read_bin("claude.log");
    assert_eq!(claudes.lines().count(), 1, "{claudes}");
    assert!(claudes.starts_with("ansel|hand=unset|ident=|"), "{claudes}");
    assert!(
        claudes.contains(&format!(
            "|hall=hall,{},w1:p1",
            rig.bin.join("hall").display()
        )),
        "{claudes}"
    );
    // The client config names reeve's listing of the test home, and up printed it.
    let client_config = std::fs::read_to_string(rig.home.join("standin/hall.toml")).unwrap();
    assert!(
        client_config.contains(&format!(
            "[reeve]\nlisting_command = [\"{}\", \"--home\", \"{h}\"]\n",
            listing.display()
        )),
        "{client_config}"
    );
    assert!(up.contains("\"scout-1\""), "{up}");
    assert_eq!(
        std::fs::read_to_string(listing_bin.join("reeve-listing.log")).unwrap(),
        format!("--home {h}\n")
    );
    // The world file records the test reeve's pane, and no hand.
    let world = rig.listing();
    assert_eq!(world["reeves"][0]["session"]["pane_id"], "w1:p1");
    assert_eq!(world["errands"], serde_json::json!([]));

    let status = rig.ok("test-setup", &["--listing", "reeve", "status"]);
    assert!(!status.contains("CHECK"), "{status}");
    assert!(status.contains("\"scout-1\""), "{status}");

    // test-drive finds the hand at the pane reeve's listing places in hall-dev,
    // never at the herdr entry's.
    std::fs::write(rig.bin.join("blocked"), "").unwrap();
    let drive = rig.ok("test-drive", &["--listing", "reeve", "scout-1", "deny"]);
    assert!(
        drive.contains("test hand scout-1: pane w1:t5:p1 in hall-dev"),
        "{drive}"
    );
    assert!(
        rig.calls()
            .iter()
            .any(|c| c.starts_with("agent send-keys w1:t5:p1 esc --session hall-dev|")),
        "{:?}",
        rig.calls()
    );
    assert!(!rig.calls().iter().any(|c| c.contains("w7:p7")));
    let out = rig.run("test-drive", &["--listing", "reeve", "scout-2", "deny"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");

    rig.ok("test-setup", &["--listing", "reeve", "down", "--delete"]);
    assert!(
        rig.calls()
            .iter()
            .any(|c| c.starts_with("session stop hall-dev --json|"))
    );
    assert_eq!(rig.herdr_calls(), "");
}

#[test]
fn up_with_the_stand_in_after_the_reeve_listing_adds_the_roster_to_the_world_file() {
    // The reeve listing seeds the world file with its one test reeve; a later
    // up with the stand-in's listing on the same home brings the whole roster.
    let rig = Rig::new("reeve-then-standin");
    let listing_bin = rig.user.join("yongshq/reeve/bin");
    std::fs::create_dir_all(&listing_bin).unwrap();
    executable(&listing_bin.join("reeve-listing"), FAKE_REEVE_LISTING);
    rig.ok("test-setup", &["--listing", "reeve", "up"]);
    rig.ok("test-setup", &["--listing", "reeve", "down", "--delete"]);

    let up = rig.ok("test-setup", &["up"]);
    assert!(
        up.contains("kept as it is but for the roster lines it lacked: bertil ansel-fix ansel-review bertil-probe"),
        "{up}"
    );
    let world = rig.listing();
    let mut names: Vec<String> = world["reeves"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["name"].as_str().unwrap().to_owned())
        .chain(
            world["errands"]
                .as_array()
                .unwrap()
                .iter()
                .map(|e| e["id"].as_str().unwrap().to_owned()),
        )
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            "ansel",
            "ansel-fix",
            "ansel-review",
            "bertil",
            "bertil-probe"
        ]
    );
    // Every one of them is up, at a pane hall handed out.
    for entry in world["reeves"]
        .as_array()
        .unwrap()
        .iter()
        .chain(world["errands"].as_array().unwrap())
    {
        assert!(entry["session"]["pane_id"].is_string(), "{entry}");
    }

    // A third up finds nothing missing.
    let up = rig.ok("test-setup", &["up"]);
    assert!(up.contains("exists, kept as it is\n"), "{up}");
}
