//! scripts/live-check, run for real against a fake HOME, a fake /var/tmp and a
//! fake `lsof` on PATH, all under target/. No test reads the live herdr's
//! folders, the real process table's sockets or ~/.reeve.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

/// Logs its arguments and prints `lsof-hall.out` or `lsof-herdr.out` for `-c
/// hall` or `-c herdr`. With no such file it exits 1 with no output, as lsof
/// does when nothing matches; with `lsof-fail` present it exits 1 printing that
/// file to stderr, as lsof does on an error.
const FAKE_LSOF: &str = r#"#!/bin/bash
dir=$(dirname "$0")
printf '%s\n' "$*" >>"$dir/lsof.log"
case " $* " in
  *" -c hall "*) f=hall ;;
  *" -c herdr "*) f=herdr ;;
  *) f=none ;;
esac
if [ -e "$dir/lsof-fail" ]; then cat "$dir/lsof-fail" >&2; exit 1; fi
if [ -e "$dir/lsof-$f.out" ]; then cat "$dir/lsof-$f.out"; exit 0; fi
exit 1
"#;

const HEADER: &str = "COMMAND   PID USER   FD   TYPE             DEVICE SIZE/OFF NODE NAME\n";

struct Rig {
    user: PathBuf,
    var_tmp: PathBuf,
    bin: PathBuf,
}

impl Rig {
    fn new(test: &str) -> Rig {
        let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("live-check")
            .join(test);
        let _ = std::fs::remove_dir_all(&root);
        let user = root.join("user");
        let var_tmp = root.join("var-tmp");
        let bin = root.join("bin");
        for d in [&user, &var_tmp, &bin] {
            std::fs::create_dir_all(d).unwrap();
        }
        let lsof = bin.join("lsof");
        std::fs::write(&lsof, FAKE_LSOF).unwrap();
        std::fs::set_permissions(&lsof, std::fs::Permissions::from_mode(0o755)).unwrap();
        Rig {
            user: user.canonicalize().unwrap(),
            var_tmp: var_tmp.canonicalize().unwrap(),
            bin: bin.canonicalize().unwrap(),
        }
    }

    fn config(&self) -> PathBuf {
        self.user.join(".config/herdr")
    }

    fn state(&self) -> PathBuf {
        self.user.join(".local/state/herdr")
    }

    fn worktrees(&self) -> PathBuf {
        self.user.join(".herdr/worktrees")
    }

    fn pane_graphics(&self) -> PathBuf {
        // SAFETY: getuid has no preconditions.
        let uid = unsafe { libc::getuid() };
        self.var_tmp.join(format!("herdr-pane-graphics-{uid}"))
    }

    fn baseline_path(&self) -> PathBuf {
        self.user.join(".local/state/hall-live-check/baseline.json")
    }

    fn baseline(&self) -> Value {
        serde_json::from_slice(&std::fs::read(self.baseline_path()).unwrap()).unwrap()
    }

    /// A live herdr setup with something in each of its four places.
    fn populated(test: &str) -> Rig {
        let rig = Rig::new(test);
        touch(&rig.config().join("config.toml"));
        touch(&rig.config().join("sessions/work/session.json"));
        touch(&rig.state().join("agent-detection/claude.toml"));
        std::fs::create_dir_all(rig.worktrees().join("portfolio/main")).unwrap();
        std::fs::create_dir_all(rig.pane_graphics().join("server-1-abc")).unwrap();
        rig
    }

    fn lsof_out(&self, which: &str, body: &str) {
        std::fs::write(
            self.bin.join(format!("lsof-{which}.out")),
            format!("{HEADER}{body}"),
        )
        .unwrap();
    }

    fn lsof_log(&self) -> Vec<String> {
        std::fs::read_to_string(self.bin.join("lsof.log"))
            .unwrap_or_default()
            .lines()
            .map(String::from)
            .collect()
    }

    fn run_with_path(&self, args: &[&str], path: &str) -> Output {
        Command::new(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/live-check"))
            .args(args)
            .env_clear()
            .env("HOME", &self.user)
            .env("PATH", path)
            .env("HALL_LIVE_CHECK_VAR_TMP", &self.var_tmp)
            .output()
            .unwrap()
    }

    fn run(&self, args: &[&str]) -> Output {
        self.run_with_path(
            args,
            &format!("{}:/usr/bin:/bin:/usr/sbin:/sbin", self.bin.display()),
        )
    }
}

fn touch(p: &Path) {
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, "x\n").unwrap();
}

fn text(out: &Output) -> (String, String) {
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn passes(out: &Output) -> String {
    let (stdout, stderr) = text(out);
    assert!(out.status.success(), "expected a pass:\n{stdout}\n{stderr}");
    stdout
}

fn fails(out: &Output) -> (String, String) {
    let (stdout, stderr) = text(out);
    assert_eq!(
        out.status.code(),
        Some(1),
        "expected a failure:\n{stdout}\n{stderr}"
    );
    (stdout, stderr)
}

fn line<'a>(stdout: &'a str, part: &str) -> &'a str {
    stdout
        .lines()
        .find(|l| l.starts_with(&format!("{part}: ")))
        .unwrap_or_else(|| panic!("no {part} line in:\n{stdout}"))
}

fn entries(b: &Value, place: &str) -> Vec<String> {
    b["places"][place]["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e.as_str().unwrap().to_string())
        .collect()
}

#[test]
fn first_run_records_baseline_once() {
    let rig = Rig::populated("first-run");
    let out = rig.run(&[]);
    let stdout = passes(&out);
    let (_, stderr) = text(&out);
    assert!(stderr.contains("baseline recorded"), "{stderr}");
    for part in [
        "config",
        "state",
        "worktrees",
        "pane-graphics",
        "peers",
        "reeve-state",
        "reeve-hall-dev",
    ] {
        assert!(line(&stdout, part).contains(": pass"), "{stdout}");
    }
    assert_eq!(stdout.lines().count(), 7, "one line per part:\n{stdout}");
    let b = rig.baseline();
    assert_eq!(
        entries(&b, "config"),
        [
            "config.toml",
            "sessions",
            "sessions/work",
            "sessions/work/session.json"
        ]
    );
    assert_eq!(b["places"]["config"]["exists"], true);
    let first = std::fs::read(rig.baseline_path()).unwrap();

    // A later run, or one with an unknown flag, never rewrites it.
    touch(&rig.config().join("later.txt"));
    fails(&rig.run(&[]));
    assert_eq!(rig.run(&["--rebaseline"]).status.code(), Some(2));
    assert_eq!(std::fs::read(rig.baseline_path()).unwrap(), first);
    let (_, stderr) = text(&rig.run(&[]));
    assert!(!stderr.contains("baseline recorded"), "{stderr}");
}

#[test]
fn absent_place_is_an_empty_listing() {
    let rig = Rig::new("absent");
    let stdout = passes(&rig.run(&[]));
    assert!(
        line(&stdout, "pane-graphics").contains("absent"),
        "{stdout}"
    );
    let b = rig.baseline();
    for place in ["config", "state", "worktrees", "pane-graphics"] {
        assert_eq!(b["places"][place]["exists"], false);
        assert!(entries(&b, place).is_empty());
    }
    // The folder appearing with an entry in it is a new entry.
    std::fs::create_dir_all(rig.pane_graphics().join("server-7-n")).unwrap();
    let (stdout, stderr) = fails(&rig.run(&[]));
    assert!(line(&stdout, "pane-graphics").contains("FAIL"), "{stdout}");
    assert!(stderr.contains("new server-7-n"), "{stderr}");
}

#[test]
fn absent_place_appearing_empty_fails_until_accepted() {
    // An empty base folder is what a herdr-pathed server leaves behind
    // (decision 44e): absent in the baseline, present now, fails.
    let rig = Rig::new("appeared");
    passes(&rig.run(&[]));
    std::fs::create_dir_all(rig.pane_graphics()).unwrap();
    std::fs::create_dir_all(rig.config()).unwrap();

    let (stdout, stderr) = fails(&rig.run(&[]));
    for part in ["pane-graphics", "config"] {
        assert!(
            line(&stdout, part).contains("FAIL appeared, absent in the baseline"),
            "{stdout}"
        );
    }
    assert!(line(&stdout, "state").contains("pass"), "{stdout}");
    assert!(
        stderr.contains(&format!(
            "pane-graphics: {} appeared (born 20",
            rig.pane_graphics().display()
        )),
        "{stderr}"
    );
    assert!(stderr.contains("accept pane-graphics ."), "{stderr}");

    // The folder itself is accepted as `.`, with who and why; only a folder
    // that is on disk now and was absent in the baseline.
    let accept_dot = |place: &str| {
        rig.run(&[
            "accept",
            place,
            ".",
            "--by",
            "liege",
            "--reason",
            "live herdr images",
        ])
    };
    assert_eq!(accept_dot("state").status.code(), Some(1));
    passes(&accept_dot("pane-graphics"));
    assert_eq!(accept_dot("pane-graphics").status.code(), Some(1));
    let b = rig.baseline();
    assert_eq!(b["places"]["pane-graphics"]["exists"], true);
    assert!(entries(&b, "pane-graphics").is_empty());
    let accepted = b["accepted"].as_array().unwrap();
    assert_eq!(accepted.len(), 1);
    assert_eq!(accepted[0]["place"], "pane-graphics");
    assert_eq!(accepted[0]["entry"], ".");
    assert_eq!(accepted[0]["by"], "liege");

    let (stdout, _) = fails(&rig.run(&[]));
    assert!(line(&stdout, "pane-graphics").contains("pass"), "{stdout}");
    assert!(line(&stdout, "config").contains("FAIL"), "{stdout}");

    // Entries inside an accepted folder are still judged on their own.
    passes(&accept_dot("config"));
    passes(&rig.run(&[]));
    std::fs::create_dir_all(rig.pane_graphics().join("server-7-n")).unwrap();
    let (stdout, _) = fails(&rig.run(&[]));
    assert!(
        line(&stdout, "pane-graphics").contains("FAIL 1 new entries"),
        "{stdout}"
    );
}

#[test]
fn place_that_is_a_symlink_is_followed() {
    // As on the liege's Mac: ~/.config/herdr links into a dotfiles folder.
    let rig = Rig::new("symlink-root");
    let dotfiles = rig.user.join("dotfiles/herdr/.config/herdr");
    touch(&dotfiles.join("config.toml"));
    std::fs::create_dir_all(rig.user.join(".config")).unwrap();
    std::os::unix::fs::symlink("../dotfiles/herdr/.config/herdr", rig.config()).unwrap();
    // A link inside a place is listed, not followed.
    std::os::unix::fs::symlink(&dotfiles, dotfiles.join("loop")).unwrap();
    passes(&rig.run(&[]));
    assert_eq!(rig.baseline()["places"]["config"]["exists"], true);
    assert_eq!(entries(&rig.baseline(), "config"), ["config.toml", "loop"]);

    // A socket named through the real path is still the live herdr's.
    rig.lsof_out(
        "herdr",
        &format!(
            "herdr    1186 yong   17u  unix 0x6680e81d2b274bc8      0t0      {}/herdr.sock\n",
            dotfiles.display()
        ),
    );
    rig.lsof_out(
        "hall",
        "hall     4242 yong    3u  unix 0xbf80ef7d4cee9c28      0t0      ->0x6680e81d2b274bc8\n",
    );
    let (stdout, _) = fails(&rig.run(&[]));
    assert!(line(&stdout, "peers").contains("FAIL"), "{stdout}");
}

#[test]
fn new_entry_fails_until_accepted() {
    let rig = Rig::populated("accept");
    passes(&rig.run(&[]));
    touch(&rig.state().join("client-shell/state.json"));
    std::fs::remove_file(rig.config().join("config.toml")).unwrap();

    let (stdout, stderr) = fails(&rig.run(&[]));
    assert!(
        line(&stdout, "state").contains("FAIL 2 new entries"),
        "{stdout}"
    );
    assert!(
        line(&stdout, "config").contains("pass"),
        "a gone entry does not fail:\n{stdout}"
    );
    assert!(
        stderr.contains("gone since baseline config.toml"),
        "{stderr}"
    );
    assert!(
        stderr.contains("state: new client-shell/state.json (born 20"),
        "{stderr}"
    );

    // Only an entry on disk and not yet in the baseline can be accepted, and
    // only with who and why.
    let missing = rig.run(&[
        "accept",
        "state",
        "nothing-here",
        "--by",
        "t",
        "--reason",
        "r",
    ]);
    assert_eq!(missing.status.code(), Some(1));
    let known = rig.run(&[
        "accept",
        "state",
        "agent-detection",
        "--by",
        "t",
        "--reason",
        "r",
    ]);
    assert_eq!(known.status.code(), Some(1));
    assert_eq!(
        rig.run(&["accept", "state", "client-shell"]).status.code(),
        Some(2)
    );
    assert_eq!(
        rig.run(&["accept", "nowhere", "x", "--by", "t", "--reason", "r"])
            .status
            .code(),
        Some(2)
    );

    for e in ["client-shell", "client-shell/state.json"] {
        let out = rig.run(&[
            "accept",
            "state",
            e,
            "--by",
            "liege",
            "--reason",
            "herdr client state",
        ]);
        passes(&out);
    }
    let b = rig.baseline();
    let accepted = b["accepted"].as_array().unwrap();
    assert_eq!(accepted.len(), 2);
    assert_eq!(accepted[0]["place"], "state");
    assert_eq!(accepted[0]["entry"], "client-shell");
    assert_eq!(accepted[0]["by"], "liege");
    assert_eq!(accepted[0]["reason"], "herdr client state");
    assert!(accepted[0]["at"].as_str().unwrap().ends_with('Z'));
    assert!(entries(&b, "state").contains(&"client-shell/state.json".to_string()));
    passes(&rig.run(&[]));
}

#[test]
fn depth_per_place() {
    let rig = Rig::populated("depth");
    passes(&rig.run(&[]));
    let b = rig.baseline();
    assert_eq!(entries(&b, "worktrees"), ["portfolio", "portfolio/main"]);
    assert_eq!(entries(&b, "pane-graphics"), ["server-1-abc"]);

    // Inside a worktree or a server's folder is not listed.
    touch(&rig.worktrees().join("portfolio/main/src/lib.rs"));
    touch(&rig.pane_graphics().join("server-1-abc/image-1"));
    passes(&rig.run(&[]));

    // Config and state are listed at every level.
    touch(&rig.config().join("sessions/work/a/b/c/deep.json"));
    let (stdout, stderr) = fails(&rig.run(&[]));
    assert!(
        line(&stdout, "config").contains("FAIL 4 new entries"),
        "{stdout}"
    );
    assert!(
        stderr.contains("new sessions/work/a/b/c/deep.json"),
        "{stderr}"
    );
    std::fs::remove_dir_all(rig.config().join("sessions/work/a")).unwrap();

    // A new branch names the repo its .git file points to.
    let wt = rig.worktrees().join("portfolio/feat-x");
    std::fs::create_dir_all(&wt).unwrap();
    std::fs::write(
        wt.join(".git"),
        "gitdir: /src/portfolio/.git/worktrees/feat-x\n",
    )
    .unwrap();
    // A new server folder names its pid's command while it runs.
    let me = std::process::id();
    std::fs::create_dir_all(rig.pane_graphics().join(format!("server-{me}-n1"))).unwrap();
    std::fs::create_dir_all(rig.pane_graphics().join("server-999999999-n2")).unwrap();
    let (stdout, stderr) = fails(&rig.run(&[]));
    assert!(
        line(&stdout, "worktrees").contains("FAIL 1 new entries"),
        "{stdout}"
    );
    assert!(stderr.contains("new portfolio/feat-x (born"), "{stderr}");
    assert!(stderr.contains("repo /src/portfolio"), "{stderr}");
    assert!(
        line(&stdout, "pane-graphics").contains("FAIL 2 new entries"),
        "{stdout}"
    );
    assert!(stderr.contains(&format!("pid {me} alive: ")), "{stderr}");
    assert!(stderr.contains("pid 999999999 not running"), "{stderr}");
}

#[test]
fn peer_on_a_live_herdr_socket_fails() {
    let rig = Rig::new("peer-hit");
    let config = rig.config();
    rig.lsof_out(
        "herdr",
        &format!(
            "herdr    1186 yong    4u  unix 0x8ab16074ea3a2569      0t0      {c}/herdr.sock\n\
             herdr    1186 yong   17u  unix 0x6680e81d2b274bc8      0t0      {c}/herdr.sock\n\
             herdr    1182 yong    4u  unix 0x3d8d60d548660ca6      0t0      ->0xca23e543a8c4b4e6\n",
            c = config.display()
        ),
    );
    rig.lsof_out(
        "hall",
        "hall     4242 yong    3u  unix 0xbf80ef7d4cee9c28      0t0      ->0x6680e81d2b274bc8\n",
    );
    let (stdout, stderr) = fails(&rig.run(&[]));
    assert!(
        line(&stdout, "peers").contains("FAIL 1 hall fds"),
        "{stdout}"
    );
    assert!(stderr.contains("pid 4242"), "{stderr}");
    assert!(
        stderr.contains(&format!(
            "herdr pid 1186 fd 17u {}/herdr.sock",
            config.display()
        )),
        "{stderr}"
    );

    // Both sides are asked for unix fds of that command only.
    let log = rig.lsof_log();
    assert!(log.contains(&"-nP -a -U -c hall".to_string()), "{log:?}");
    assert!(log.contains(&"-nP -a -U -c herdr".to_string()), "{log:?}");
}

#[test]
fn peer_check_ignores_lines_lsof_without_a_would_add() {
    let rig = Rig::new("peer-no-a");
    let c = rig.config();
    // As `lsof -U -c ...` without -a prints: every process's unix fds, and every
    // fd of the named command.
    rig.lsof_out(
        "herdr",
        &format!(
            "Python   69948 yong    5u  unix 0xf712f070ab9808fc      0t0      {c}/herdr.sock\n\
             herdr    1186 yong    4u  unix 0x8ab16074ea3a2569      0t0      {c}/herdr.sock\n\
             herdr    1186 yong   17u  unix 0x6680e81d2b274bc8      0t0      {c}/herdr.sock\n\
             herdr    1186 yong   12u  IPv4 0x1111111111111111      0t0      TCP 127.0.0.1:1\n\
             herdr    5555 yong    4u  unix 0x2222222222222222      0t0      /tmp/throwaway/herdr.sock\n",
            c = c.display()
        ),
    );
    rig.lsof_out(
        "hall",
        "Python   69948 yong    4u  unix 0x6144d70e1e13a59a      0t0      ->0x6680e81d2b274bc8\n\
         hall     4242 yong    5u   REG 0x3333333333333333      0t0      ->0x6680e81d2b274bc8\n\
         hall     4242 yong    6u  unix 0x4444444444444444      0t0      ->0xf712f070ab9808fc\n\
         hall     4242 yong    7u  unix 0x5555555555555555      0t0      ->0x2222222222222222\n\
         hall     4242 yong    8u  unix 0x6666666666666666      0t0      /Users/x/.config/hall-dev/herdr.sock\n",
    );
    let stdout = passes(&rig.run(&[]));
    assert!(
        line(&stdout, "peers").contains("pass (3 hall unix fds, 2 live herdr socket fds"),
        "{stdout}"
    );

    // The same lines plus a real hit still fail.
    let mut hall = std::fs::read_to_string(rig.bin.join("lsof-hall.out")).unwrap();
    hall.push_str(
        "hall-stan 4343 yong    3u  unix 0x7777777777777777      0t0      ->0x8AB16074EA3A2569\n",
    );
    std::fs::write(rig.bin.join("lsof-hall.out"), hall).unwrap();
    let (stdout, stderr) = fails(&rig.run(&[]));
    assert!(
        line(&stdout, "peers").contains("FAIL 1 hall fds"),
        "{stdout}"
    );
    assert!(stderr.contains("hall-stan pid 4343"), "{stderr}");
}

#[test]
fn unparsed_lsof_line_of_the_target_command_fails() {
    // Decision 44e: an fd of hall's or herdr's the check cannot read fails the
    // part rather than being skipped as harmless.
    let rig = Rig::new("lsof-unparsed");
    rig.lsof_out(
        "hall",
        "hall     4242 yong    3u  unix 0xbf80ef7d4cee9c28      0t0      ->0x6680e81d2b274bc8\n\
         hall     4242 yong    4u  unix\n",
    );
    let (stdout, stderr) = fails(&rig.run(&[]));
    assert!(
        line(&stdout, "peers").contains("FAIL an lsof line of hall or herdr did not parse"),
        "{stdout}"
    );
    assert!(
        stderr.contains("1 hall lines not parsed: hall     4242 yong    4u  unix"),
        "{stderr}"
    );

    // The same on herdr's side.
    rig.lsof_out(
        "hall",
        "hall     4242 yong    3u  unix 0xbf80ef7d4cee9c28      0t0      ->0x6680e81d2b274bc8\n",
    );
    rig.lsof_out("herdr", "herdr 1186 yong 17u unix ?\n");
    let (stdout, _) = fails(&rig.run(&[]));
    assert!(line(&stdout, "peers").contains("did not parse"), "{stdout}");

    // Another command's odd line (lsof without -a) and a line with no NAME
    // are not failures.
    rig.lsof_out(
        "herdr",
        "Google   777 yong    3u  unix ???\n\
         herdr    1186 yong   18u  unix 0x1234567812345678      0t0\n",
    );
    let stdout = passes(&rig.run(&[]));
    assert!(
        line(&stdout, "peers").contains("pass (1 hall unix fds, 0 live herdr socket fds"),
        "{stdout}"
    );
}

#[test]
fn lsof_failing_or_missing_fails() {
    let rig = Rig::new("lsof-fail");
    std::fs::write(
        rig.bin.join("lsof-fail"),
        "lsof: can't read kernel name list\n",
    )
    .unwrap();
    let (stdout, stderr) = fails(&rig.run(&[]));
    assert!(
        line(&stdout, "peers").contains("FAIL lsof did not run cleanly"),
        "{stdout}"
    );
    assert!(stderr.contains("can't read kernel name list"), "{stderr}");

    // No lsof on PATH at all: only what the script needs besides it.
    let tools = rig.bin.join("tools");
    std::fs::create_dir_all(&tools).unwrap();
    for t in ["/usr/bin/python3", "/bin/bash", "/bin/ps"] {
        std::os::unix::fs::symlink(t, tools.join(Path::new(t).file_name().unwrap())).unwrap();
    }
    let out = rig.run_with_path(&[], tools.to_str().unwrap());
    let (stdout, _) = fails(&out);
    assert!(line(&stdout, "peers").contains("FAIL"), "{stdout}");
}

#[test]
fn reeve_state_naming_the_test_setup_fails() {
    let rig = Rig::new("grep");
    let state = rig.user.join(".reeve/state");
    // Near misses: a longer name, a name inside a longer one, a similar home.
    std::fs::create_dir_all(state.join("sessions/s1")).unwrap();
    std::fs::write(
        state.join("sessions/s1/meta"),
        "reeve anselm\nerrand ansel-fixed\nhome /u/.reeve-hall-devx\n",
    )
    .unwrap();
    std::fs::write(state.join("binary.bin"), [0u8, 159, 146, 150, b'a']).unwrap();
    let stdout = passes(&rig.run(&[]));
    assert!(line(&stdout, "reeve-state").contains("pass"), "{stdout}");

    for (file, body) in [
        (
            "sessions/s1/home",
            "REEVE_HOME=/Users/x/.reeve-hall-dev/work\n".to_string(),
        ),
        ("sessions/s1/tilde", "cd ~/.reeve-hall-dev\n".to_string()),
        ("errands.log", "dispatched ansel-fix\n".to_string()),
        ("reeves", "name=bertil;\n".to_string()),
    ] {
        let p = state.join(file);
        std::fs::write(&p, body).unwrap();
        let (stdout, stderr) = fails(&rig.run(&[]));
        assert!(
            line(&stdout, "reeve-state").contains("FAIL 1 files"),
            "{file}: {stdout}"
        );
        assert!(stderr.contains(p.to_str().unwrap()), "{stderr}");
        std::fs::remove_file(&p).unwrap();
    }

    // A file whose own path names a test errand.
    touch(&state.join("errands/bertil-probe/meta"));
    let (stdout, _) = fails(&rig.run(&[]));
    assert!(line(&stdout, "reeve-state").contains("FAIL"), "{stdout}");
}

#[test]
fn reeve_state_naming_hall_dev_fails_until_accepted() {
    let rig = Rig::new("hall-dev");
    let state = rig.user.join(".reeve/state");
    // Near misses: the live herdr's pane key, hall-dev inside longer words, and
    // the test home, which is reeve-state's to judge.
    std::fs::create_dir_all(state.join("sessions/s1")).unwrap();
    std::fs::write(
        state.join("sessions/s1/pane"),
        "wS:p1@/u/.config/herdr/herdr.sock\nfix-hall-dev-docs hall-devx\n",
    )
    .unwrap();
    let stdout = passes(&rig.run(&[]));
    assert!(line(&stdout, "reeve-hall-dev").contains("pass"), "{stdout}");
    std::fs::write(state.join("sessions/s1/home"), "/u/.reeve-hall-dev\n").unwrap();
    let (stdout, _) = fails(&rig.run(&[]));
    assert!(line(&stdout, "reeve-state").contains("FAIL"), "{stdout}");
    assert!(line(&stdout, "reeve-hall-dev").contains("pass"), "{stdout}");
    std::fs::remove_file(state.join("sessions/s1/home")).unwrap();

    // A test hand's pane key, written by a reeve hook that ran without the
    // test home (finding F1 of step5-shown-working).
    let pane = state.join("sessions/s2/pane");
    std::fs::create_dir_all(pane.parent().unwrap()).unwrap();
    std::fs::write(
        &pane,
        "w1:p2@/Users/x/.config/hall-dev/sessions/hall-dev/herdr.sock\n",
    )
    .unwrap();
    let (stdout, stderr) = fails(&rig.run(&[]));
    assert!(
        line(&stdout, "reeve-hall-dev").contains(
            "FAIL 1 new files name /.config/hall-dev/, /hall-dev/sessions/, /sessions/hall-dev/"
        ),
        "{stdout}"
    );
    assert!(line(&stdout, "reeve-state").contains("pass"), "{stdout}");
    assert!(
        stderr.contains("reeve-hall-dev: new sessions/s2/pane names /.config/hall-dev/ (born 20"),
        "{stderr}"
    );
    assert!(stderr.contains("accept reeve-hall-dev <file>"), "{stderr}");

    // Only a file that names hall-dev now, not yet in the baseline, with who
    // and why.
    let accept = |file: &str| {
        rig.run(&[
            "accept",
            "reeve-hall-dev",
            file,
            "--by",
            "liege",
            "--reason",
            "seen",
        ])
    };
    assert_eq!(accept("sessions/s1/pane").status.code(), Some(1));
    assert_eq!(accept("sessions/none").status.code(), Some(1));
    assert_eq!(
        rig.run(&["accept", "reeve-hall-dev", "sessions/s2/pane"])
            .status
            .code(),
        Some(2)
    );
    passes(&accept("sessions/s2/pane"));
    assert_eq!(accept("sessions/s2/pane").status.code(), Some(1));
    let b = rig.baseline();
    assert_eq!(
        b["reeve-hall-dev"]["entries"],
        serde_json::json!(["sessions/s2/pane"])
    );
    let accepted = b["accepted"].as_array().unwrap();
    assert_eq!(accepted.len(), 1);
    assert_eq!(accepted[0]["place"], "reeve-hall-dev");
    assert_eq!(accepted[0]["entry"], "sessions/s2/pane");
    passes(&rig.run(&[]));

    // A live errand whose id is exactly hall-dev names neither the debug
    // hall's folder nor a session folder: its meta file, branch, errand folder
    // and worktree pass (review-mend-step5-test-setup, finding 2).
    std::fs::write(
        state.join("hall-dev.meta"),
        "branch=docs/hall-dev\nworktree=/u/yongshq/hall.worktrees/scout-hall-dev\nbrief=/u/.reeve/errands/hall-dev/brief.md\n",
    )
    .unwrap();
    touch(&state.join("groups/hall-dev/w1"));
    let stdout = passes(&rig.run(&[]));
    assert!(line(&stdout, "reeve-hall-dev").contains("pass"), "{stdout}");

    // The other forms are new too: a pane key of the debug hall's default
    // session, of hall-dev's folder under another XDG_CONFIG_HOME, and of the
    // test session in another hall's folder.
    for (file, key) in [
        ("s3", "w1:p1@/u/.config/hall-dev/herdr.sock"),
        ("s4", "w1:p1@/x/xdg/hall-dev/sessions/other/herdr.sock"),
        ("s5", "w1:p1@/u/.config/hall/sessions/hall-dev/herdr.sock"),
    ] {
        let pane = state.join("sessions").join(file).join("pane");
        std::fs::create_dir_all(pane.parent().unwrap()).unwrap();
        std::fs::write(&pane, format!("{key}\n")).unwrap();
        let (stdout, stderr) = fails(&rig.run(&[]));
        assert!(
            line(&stdout, "reeve-hall-dev").contains("FAIL 1 new files"),
            "{key}: {stdout}"
        );
        assert!(
            stderr.contains(&format!("new sessions/{file}/pane")),
            "{key}: {stderr}"
        );
        std::fs::remove_file(&pane).unwrap();
    }
}

#[test]
fn reeve_hall_dev_baseline() {
    // The first run records the files that name hall-dev then, as it records
    // the places.
    let rig = Rig::new("hall-dev-first");
    let state = rig.user.join(".reeve/state");
    std::fs::create_dir_all(&state).unwrap();
    std::fs::write(state.join("old"), "@/u/.config/hall-dev/x.sock\n").unwrap();
    passes(&rig.run(&[]));
    assert_eq!(
        rig.baseline()["reeve-hall-dev"]["entries"],
        serde_json::json!(["old"])
    );

    // A baseline recorded before the part existed has no entries for it: every
    // file naming hall-dev is new, and the run does not write the baseline.
    let mut b = rig.baseline();
    b.as_object_mut().unwrap().remove("reeve-hall-dev");
    std::fs::write(rig.baseline_path(), serde_json::to_vec_pretty(&b).unwrap()).unwrap();
    let before = std::fs::read(rig.baseline_path()).unwrap();
    let (stdout, _) = fails(&rig.run(&[]));
    assert!(
        line(&stdout, "reeve-hall-dev").contains("FAIL 1 new files"),
        "{stdout}"
    );
    assert_eq!(std::fs::read(rig.baseline_path()).unwrap(), before);
    passes(&rig.run(&[
        "accept",
        "reeve-hall-dev",
        "old",
        "--by",
        "liege",
        "--reason",
        "before the part",
    ]));
    passes(&rig.run(&[]));
}

#[test]
fn reeve_state_skips_the_shared_browsers_folder() {
    // Reeve's shared browser keeps its profile under ~/.reeve/state/browser/:
    // third-party files that held `ansel` and `bertil` by chance (decision 77).
    let rig = Rig::new("browser");
    let state = rig.user.join(".reeve/state");
    let profile = state.join("browser/profile");
    std::fs::create_dir_all(profile.join("ZxcvbnData/3.2.0.0")).unwrap();
    std::fs::write(
        profile.join("ZxcvbnData/3.2.0.0/us_tv_and_film.txt"),
        "ansel\nbertil\n",
    )
    .unwrap();
    std::fs::write(
        profile.join("pane"),
        "w1:p2@/Users/x/.config/hall-dev/sessions/hall-dev/herdr.sock\n",
    )
    .unwrap();
    touch(&profile.join("ansel-fix/meta"));
    let stdout = passes(&rig.run(&[]));
    assert!(line(&stdout, "reeve-state").contains("pass"), "{stdout}");
    assert!(line(&stdout, "reeve-hall-dev").contains("pass"), "{stdout}");
    assert_eq!(
        rig.baseline()["reeve-hall-dev"]["entries"],
        serde_json::json!([])
    );

    // Only that folder: a `browser` elsewhere, or a file of that name, is read.
    for file in ["sessions/browser/meta", "browser.log", "browserx/meta"] {
        let p = state.join(file);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, "reeve bertil\n").unwrap();
        let (stdout, stderr) = fails(&rig.run(&[]));
        assert!(
            line(&stdout, "reeve-state").contains("FAIL 1 files"),
            "{file}: {stdout}"
        );
        assert!(stderr.contains(p.to_str().unwrap()), "{stderr}");
        std::fs::remove_file(&p).unwrap();
    }
    let p = state.join("sessions/s1/pane");
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(&p, "w1:p1@/u/.config/hall-dev/herdr.sock\n").unwrap();
    let (stdout, _) = fails(&rig.run(&[]));
    assert!(
        line(&stdout, "reeve-hall-dev").contains("FAIL 1 new files"),
        "{stdout}"
    );
}
