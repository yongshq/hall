//! scripts/test-setup and scripts/test-drive, run in --dry-run only: every
//! command they would run is printed, and checked here against the test setup's
//! rules (ADR 0003, decisions 26, 33, 38, 47 and 48). Nothing is started, no hall
//! session is touched, and no herdr call is printed at all.

use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const SESSION: &str = "hall-dev";
const MODEL: &str = "claude-haiku-4-5-20251001";
const CLEAN_ENV: &str = "+ env -u HALL_SOCKET_PATH -u HALL_CLIENT_SOCKET_PATH -u HALL_SESSION -u HALL_CONFIG_PATH -u HALL_ENV -u HALL_PANE_ID -u HALL_BIN_PATH -u HALL_TAB_ID -u HALL_WORKSPACE_ID -u REEVE_BACKEND ";
const REEVES: [&str; 2] = ["ansel", "bertil"];
const ERRANDS: usize = 3;
const HANDS: [&str; ERRANDS] = ["ansel-fix", "ansel-review", "bertil-probe"];
const ROSTER: usize = 5;

fn scripts() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts")
}

/// The hall binary the dry runs name. Never run: a dry run runs nothing.
fn hall_bin() -> PathBuf {
    Path::new(env!("CARGO_TARGET_TMPDIR")).join("scripts-dry-run/no-such-hall")
}

/// How every hall call starts: the HALL_* names hall reads dropped, then hall.
fn hall_prefix() -> String {
    format!("{CLEAN_ENV}{} ", hall_bin().display())
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

/// What the scripts' starter carries when a claude runs them from a live herdr
/// pane (decision 48): who that claude is, and which herdr it talks to.
const STARTER: [(&str, &str); 9] = [
    ("REEVE_HAND", "starter-hand"),
    ("REEVE_HALL_SOCKET", "/live/hall.sock"),
    ("CLAUDECODE", "1"),
    ("CLAUDE_PID", "4242"),
    ("CLAUDE_EFFORT", "high"),
    ("CLAUDE_CODE_SESSION_ID", "starter-session"),
    ("CLAUDE_CODE_MESSAGING_TOKEN", "starter-token"),
    ("HERDR_SESSION", "live"),
    ("HERDR_PANE_ID", "w9:p9"),
];

/// The `env -u` list the hall server starts under: every STARTER name, the
/// CLAUDE_CODE_* and HERDR_* ones sorted.
const SERVER_DROP: &str = "-u REEVE_HAND -u REEVE_HALL_SOCKET -u REEVE_HALL_SESSION -u REEVE_HALL_BIN -u CLAUDECODE -u CLAUDE_PID -u CLAUDE_EFFORT -u CLAUDE_CODE_MESSAGING_TOKEN -u CLAUDE_CODE_SESSION_ID -u HERDR_PANE_ID -u HERDR_SESSION";
/// The `env -u` list of a claude line, which the pane's shell works out when
/// the line runs (decision 48's second amendment): the same whatever the
/// runner of up carries. It keeps the HERDR_* names hall sets into every pane
/// with that session's own values.
const CLAUDE_DROP: &str = r#"-u REEVE_HAND -u REEVE_HALL_SOCKET -u REEVE_HALL_SESSION -u REEVE_HALL_BIN -u CLAUDECODE -u CLAUDE_PID -u CLAUDE_EFFORT $(command env | command sed -nE "/^HERDR_(ENV|SOCKET_PATH|BIN_PATH|WORKSPACE_ID|TAB_ID|PANE_ID)=/d;s/^((CLAUDE_CODE|HERDR)_[A-Za-z0-9_]*)=.*/-u \1/p")"#;

/// The scripts with this process's own identity names replaced by STARTER, so
/// what they drop does not depend on who runs the tests.
fn command(name: &str, args: &[&str]) -> Command {
    let mut c = Command::new("bash");
    for (k, _) in std::env::vars() {
        if k.starts_with("CLAUDE")
            || k.starts_with("HERDR_")
            || k.starts_with("REEVE_HALL_")
            || k == "REEVE_HAND"
        {
            c.env_remove(k);
        }
    }
    c.arg(scripts().join(name))
        .args(args)
        .envs(STARTER)
        .env("HALL_STANDIN_BIN", env!("CARGO_BIN_EXE_hall-standin"))
        .env("HALL_DEBUG_BIN", hall_bin());
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
    assert_eq!(
        first[2],
        format!("  hall session:    {SESSION}"),
        "{stdout}"
    );

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
    // No herdr call at all (decision 33).
    assert!(
        !c.split_whitespace()
            .any(|w| w == "herdr" || w.ends_with("/herdr")),
        "a herdr call: {c}"
    );
    let hall = hall_bin().display().to_string();
    if c.ends_with(" server &") {
        // The hall server starts without the starter's identity names.
        assert_eq!(
            c,
            format!("{CLEAN_ENV}{SERVER_DROP} {hall} --session {SESSION} server &")
        );
    } else if c.split_whitespace().any(|w| w == hall) {
        let prefix = hall_prefix();
        assert!(c.starts_with(&prefix), "hall without a clean env: {c}");
        let rest = &c[prefix.len()..];
        let named = rest.ends_with(&format!(" --session {SESSION}"));
        let session_cmd = rest == "session list --json"
            || rest == format!("session stop {SESSION} --json")
            || rest == format!("session delete {SESSION} --json");
        assert!(named || session_cmd, "hall call names no test session: {c}");
    }
    if c.contains("claude ") {
        let h = home.to_str().unwrap();
        let cd = format!("cd \"{h}/work/");
        let at = c.find(&cd).unwrap_or_else(|| panic!("{c}"));
        let name = c[at + cd.len()..].split('"').next().unwrap();
        // A test hand gets REEVE_HAND back, set on purpose; a test reeve has none.
        let hand = if HANDS.contains(&name) {
            format!("REEVE_HAND=\"{name}\" ")
        } else {
            String::new()
        };
        assert!(
            c.contains(&format!(
                "{cd}{name}\" && env {CLAUDE_DROP} {hand}REEVE_HOME=\"{h}\" XDG_CONFIG_HOME=\"{h}/xdg-config\" XDG_STATE_HOME=\"{h}/xdg-state\" HALL_SOCKET_PATH=\"<socket-of-{SESSION}>\" claude --model {MODEL}'"
            )),
            "{c}"
        );
        assert_eq!(
            c.matches("REEVE_HAND=").count(),
            usize::from(!hand.is_empty()),
            "{c}"
        );
    }
}

fn count(cmds: &[String], needle: &str) -> usize {
    cmds.iter().filter(|c| c.contains(needle)).count()
}

/// The claude lines a dry run of up prints, with `starter` in place of STARTER
/// and the home written `<home>`.
fn claude_lines(test: &str, starter: &[(&str, &str)]) -> Vec<String> {
    let h = home(test);
    let mut c = command(
        "test-setup",
        &["--dry-run", "--home", h.to_str().unwrap(), "up"],
    );
    for (k, _) in STARTER {
        c.env_remove(k);
    }
    let out = c.envs(starter.iter().copied()).output().unwrap();
    assert!(out.status.success(), "{out:?}");
    assert!(!h.exists(), "{test} created {}", h.display());
    let lines: Vec<String> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .filter(|c| c.starts_with("+ ") && c.contains("claude --model"))
        .map(|c| c.replace(h.to_str().unwrap(), "<home>"))
        .collect();
    assert_eq!(lines.len(), ROSTER);
    lines
}

#[test]
fn the_claude_line_drops_what_the_pane_holds_not_what_the_runner_holds() {
    // A runner of up with none of the names, and one with names the other lacks
    // (a hall-dev server started by someone else holds such names): the claude
    // lines are the same, since each pane's shell works out its drop.
    let bare = claude_lines("claude-line-bare", &[]);
    let full = claude_lines(
        "claude-line-full",
        &[
            ("REEVE_HAND", "starter-hand"),
            ("CLAUDE_CODE_CHILD_SESSION", "1"),
            ("CLAUDE_CODE_MESSAGING_SOCKET", "/live/cc.sock"),
            ("HERDR_SESSION", "live"),
            ("HERDR_TAB_ID", "w9:t9"),
        ],
    );
    assert_eq!(bare, full);
    for line in &full {
        assert!(!line.contains("-u CLAUDE_CODE_"), "{line}");
        assert!(!line.contains("-u HERDR_"), "{line}");
    }
}

#[test]
fn up() {
    let h = home("up");
    let cmds = dry("test-setup", &h, &["up"]);
    assert_eq!(count(&cmds, "claude --model"), ROSTER);
    assert_eq!(count(&cmds, " workspace create "), REEVES.len());
    assert_eq!(
        count(&cmds, " tab create --workspace '<workspace-of-"),
        ERRANDS
    );
    assert_eq!(count(&cmds, " place "), ROSTER);
    assert_eq!(count(&cmds, &format!("--session {SESSION} server &")), 1);
    // The world file, and the hall client config that turns the reeve tree on.
    assert_eq!(count(&cmds, "+ write "), 2);
    assert_eq!(count(&cmds, "/standin/hall.toml"), 1);
    assert_eq!(count(&cmds, "/xdg-config"), 1 + ROSTER);
    assert_eq!(count(&cmds, "/xdg-state"), 1 + ROSTER);
    assert_eq!(count(&cmds, "REEVE_HAND=\""), ERRANDS);
    assert_eq!(count(&cmds, "cargo build"), 0);
    // Each test hand opens in its own reeve's workspace.
    for (hand, owner) in [
        ("ansel-fix", "ansel"),
        ("ansel-review", "ansel"),
        ("bertil-probe", "bertil"),
    ] {
        assert_eq!(
            count(
                &cmds,
                &format!(
                    " tab create --workspace '<workspace-of-{owner}>' --cwd {}/work/{hand} --label {hand} ",
                    h.display()
                ),
            ),
            1,
            "{hand}: {cmds:?}"
        );
    }

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
        assert_eq!(count(&cmds, " cargo build "), 1);
        assert_eq!(count(&cmds, " -p hall-standin"), 1);
        assert_eq!(count(&cmds, " -p hall "), 0);
        assert_eq!(count(&cmds, "claude --model"), ROSTER);
    }

    // Without HALL_DEBUG_BIN, up builds the debug hall, with the HALL_* names
    // dropped, and names target/debug/hall in every call.
    let out = command(
        "test-setup",
        &["--dry-run", "--home", h.to_str().unwrap(), "up"],
    )
    .env_remove("HALL_DEBUG_BIN")
    .output()
    .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(out.status.success(), "{stdout}");
    let cmds: Vec<&str> = stdout.lines().filter(|l| l.starts_with("+ ")).collect();
    let built: Vec<&&str> = cmds
        .iter()
        .filter(|c| c.contains(" cargo build "))
        .collect();
    assert_eq!(built.len(), 1, "{stdout}");
    assert!(
        built[0].starts_with(CLEAN_ENV) && built[0].ends_with(" -p hall"),
        "{stdout}"
    );
    let server = format!("/target/debug/hall --session {SESSION} server &");
    assert!(
        cmds.iter()
            .any(|c| c.starts_with(&format!("{CLEAN_ENV}{SERVER_DROP} ")) && c.ends_with(&server)),
        "{stdout}"
    );
    assert!(!h.exists());
}

#[test]
fn down() {
    let h = home("down");
    let cmds = dry("test-setup", &h, &["down"]);
    assert_eq!(
        count(&cmds, &format!("hall session stop {SESSION} --json")),
        1
    );
    assert_eq!(count(&cmds, "session delete"), 0);
    let cmds = dry("test-setup", &h, &["down", "--delete"]);
    assert_eq!(count(&cmds, "session delete"), 1);
}

#[test]
fn status_is_read_only() {
    let h = home("status");
    let cmds = dry("test-setup", &h, &["status"]);
    for c in &cmds {
        let read = [
            "hall session list --json",
            " pane get ",
            " agent explain ",
            "+ jq ",
            " locate ",
            " listing",
            "/scripts/live-check",
        ]
        .iter()
        .any(|r| c.contains(r));
        assert!(read, "status runs something that is not a read: {c}");
    }
    // The live setup check is scripts/live-check, not one of the scripts' own.
    assert_eq!(count(&cmds, "/scripts/live-check"), 1);
    assert_eq!(count(&cmds, ".reeve/state"), 0);
}

#[test]
fn drive() {
    let h = home("drive");
    // A test hand is driven as a test reeve is, so a hand at a dialog can be shown (step 4).
    for reeve in REEVES.into_iter().chain(HANDS) {
        let cmds = dry("test-drive", &h, &[reeve, "dialog"]);
        assert_eq!(count(&cmds, " agent prompt "), 1);
        assert_eq!(count(&cmds, " agent wait "), 1);
        assert_eq!(count(&cmds, " agent explain "), 1);

        let cmds = dry("test-drive", &h, &[reeve, "turn-end"]);
        assert_eq!(count(&cmds, " agent prompt "), 1);
        assert_eq!(count(&cmds, " --wait "), 1);
        assert_eq!(count(&cmds, " agent explain "), 2);
        assert_eq!(count(&cmds, " agent wait "), 1);

        let cmds = dry("test-drive", &h, &[reeve, "deny"]);
        assert_eq!(count(&cmds, " agent send-keys "), 1);
        // Esc is followed by a wait for hall to read the dialog gone.
        assert_eq!(count(&cmds, " agent wait "), 1);
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

/// The subcommands that write or launch, which refuse a home failing the walk
/// inside it, and those that go on past it with a warning.
const BLOCKS: [&[&str]; 4] = [ALL[0], ALL[3], ALL[4], ALL[5]];
const WARNS: [&[&str]; 2] = [ALL[1], ALL[2]];

/// Checks a dry run that went on past a home failing the walk inside it: it
/// warned with `needle`, ran only commands the test setup's rules allow, and
/// none that reads or writes inside the home. Returns the commands.
fn warned(out: &Output, home: &Path, needle: &str) -> Vec<String> {
    let stdout = String::from_utf8_lossy(&out.stdout);
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{err}\n{stdout}");
    assert!(err.contains("WARNING") && err.contains(needle), "{err}");
    assert!(stdout.contains("  WARNING: "), "{stdout}");
    let cmds: Vec<String> = stdout
        .lines()
        .filter(|l| l.starts_with("+ "))
        .map(String::from)
        .collect();
    for c in &cmds {
        check_command(c, home);
        for inside in [
            "hall-standin",
            " place ",
            " locate ",
            " listing",
            "+ write ",
            "mkdir",
        ] {
            assert!(!c.contains(inside), "ran inside the home: {c}");
        }
    }
    cmds
}

/// Like [`refused_everywhere`] for a home that passes the root check and fails
/// the walk inside it: up and test-drive refuse it, down and status warn.
fn refused_inside(user: &Path, home: &Path, needle: &str) {
    let hs = home.to_str().unwrap();
    for cmd in BLOCKS {
        let mut args = vec!["--dry-run", "--home", hs];
        args.extend_from_slice(&cmd[1..]);
        refused(&script_as(user, cmd[0], &args), needle);
    }
    for cmd in WARNS {
        let mut args = vec!["--dry-run", "--home", hs];
        args.extend_from_slice(&cmd[1..]);
        warned(&script_as(user, cmd[0], &args), home, needle);
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
fn refuses_a_symlink_inside_the_home() {
    let user = fake_user("inner-symlink", true);
    let live = user.join(".reeve");
    let h = user.join(".reeve-hall-dev");
    for name in [
        "standin",
        "work",
        "work/ansel",
        "state",
        "errands",
        "work/ansel/.claude",
    ] {
        let _ = std::fs::remove_dir_all(&h);
        std::fs::create_dir_all(h.join("work")).unwrap();
        let _ = std::fs::remove_dir(h.join(name));
        std::fs::create_dir_all(h.join(name).parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(&live, h.join(name)).unwrap();
        refused_inside(&user, &h, "is a symlink");
    }
    let mut names: Vec<_> = std::fs::read_dir(&live)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    names.sort();
    assert_eq!(names, ["state"]);
}

#[test]
fn refuses_a_home_it_cannot_list() {
    let user = fake_user("unlisted", true);
    let live = user.join(".reeve");
    let h = user.join(".reeve-hall-dev");
    for dir in ["standin", "work"] {
        let _ = std::fs::set_permissions(h.join("standin"), Permissions::from_mode(0o755));
        let _ = std::fs::set_permissions(h.join("work"), Permissions::from_mode(0o755));
        let _ = std::fs::remove_dir_all(&h);
        std::fs::create_dir_all(h.join("standin")).unwrap();
        std::fs::create_dir_all(h.join("work")).unwrap();
        if dir == "standin" {
            std::os::unix::fs::symlink(live.join("world.toml"), h.join("standin/world.toml"))
                .unwrap();
        } else {
            std::os::unix::fs::symlink(&live, h.join("work/ansel")).unwrap();
        }
        // Search but no read: the link inside can be followed but not listed.
        std::fs::set_permissions(h.join(dir), Permissions::from_mode(0o300)).unwrap();
        refused_inside(&user, &h, "cannot be checked for symlinks");
        std::fs::set_permissions(h.join(dir), Permissions::from_mode(0o755)).unwrap();
    }
    assert!(!live.join("world.toml").exists());
}

/// What `npm install` leaves in a test claude's working directory: a relative
/// link that stays inside the home, but a link all the same.
fn plant_npm_link(dir: &Path) {
    std::fs::create_dir_all(dir.join("node_modules/.bin")).unwrap();
    std::fs::create_dir_all(dir.join("node_modules/pkg")).unwrap();
    std::fs::write(dir.join("node_modules/pkg/cli.js"), "").unwrap();
    std::os::unix::fs::symlink("../pkg/cli.js", dir.join("node_modules/.bin/pkg")).unwrap();
}

#[test]
fn down_and_status_go_on_past_a_dirtied_home() {
    let user = fake_user("dirtied", true);
    let h = user.join(".reeve-hall-dev");
    std::fs::create_dir_all(h.join("standin")).unwrap();
    std::fs::write(h.join("standin/world.toml"), "").unwrap();
    plant_npm_link(&h.join("work/ansel"));
    refused_inside(&user, &h, "is a symlink");

    // down still stops, and with --delete deletes, the test hall session by name.
    let hs = h.to_str().unwrap();
    for (args, verb) in [
        (&["--dry-run", "--home", hs, "down"][..], "stop"),
        (&["--dry-run", "--home", hs, "down", "--delete"], "delete"),
    ] {
        let cmds = warned(&script_as(&user, "test-setup", args), &h, "is a symlink");
        assert_eq!(
            count(&cmds, &format!("hall session {verb} {SESSION} --json")),
            1,
            "{cmds:?}"
        );
    }
    let out = script_as(&user, "test-setup", &["--dry-run", "--home", hs, "status"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("CHECK: recorded panes not read"),
        "{stdout}"
    );
}

#[test]
fn the_root_check_still_blocks_down_and_status() {
    // A live reeve home a claude dirtied the same way: its root check fails
    // before its walk would, so down and status refuse it outright.
    let user = fake_user("dirtied-live", true);
    let live = user.join(".reeve");
    plant_npm_link(&live.join("work/ansel"));
    std::os::unix::fs::symlink(&live, user.join(".reeve-hall-dev")).unwrap();
    for h in [live.clone(), user.join(".reeve-hall-dev")] {
        for cmd in WARNS {
            let mut args = vec!["--dry-run", "--home", h.to_str().unwrap()];
            args.extend_from_slice(&cmd[1..]);
            refused(&script_as(&user, cmd[0], &args), "live reeve home");
        }
    }
}

/// A stand-in whose check-root and check-home pass any home, so the scripts' own checks
/// behind it can be seen; every other command goes to the real stand-in.
fn lax_standin(dir: &Path) -> PathBuf {
    let p = dir.join("lax-standin");
    std::fs::write(
        &p,
        format!(
            "#!/bin/sh\ncase $1 in check-home | check-root) printf '%s\\n' \"$3\"; exit 0 ;; esac\nexec '{}' \"$@\"\n",
            env!("CARGO_BIN_EXE_hall-standin")
        ),
    )
    .unwrap();
    std::fs::set_permissions(&p, Permissions::from_mode(0o755)).unwrap();
    p
}

#[test]
fn up_refuses_to_write_through_a_symlink() {
    let user = fake_user("write-link", true);
    let live = user.join(".reeve");
    let h = user.join(".reeve-hall-dev");
    let lax = lax_standin(&tmp("write-link"));
    // Dangling links first: a plain -e test reads them as absent.
    for (name, target) in [
        ("standin/world.toml", live.join("world.toml")),
        ("xdg-config", live.join("xdg-config")),
        ("standin", live.join("standin")),
        ("work/ansel", live.join("ansel")),
        ("work", live.clone()),
    ] {
        let _ = std::fs::remove_dir_all(&h);
        std::fs::create_dir_all(h.join("standin")).unwrap();
        std::fs::create_dir_all(h.join("work")).unwrap();
        let _ = std::fs::remove_dir(h.join(name));
        std::os::unix::fs::symlink(&target, h.join(name)).unwrap();
        let out = command(
            "test-setup",
            &["--dry-run", "--home", h.to_str().unwrap(), "up"],
        )
        .env("HOME", &user)
        .env("HALL_STANDIN_BIN", &lax)
        .output()
        .unwrap();
        assert_eq!(out.status.code(), Some(1), "{name}: {out:?}");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(err.contains("refusing to write"), "{name}: {err}");
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            !stdout.contains(&format!("{}\n", h.join(name).display())),
            "{name} printed as written: {stdout}"
        );
    }
    let mut names: Vec<_> = std::fs::read_dir(&live)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    names.sort();
    assert_eq!(names, ["state"]);
}

#[test]
fn refuses_a_parent_dir_in_the_home() {
    let dir = tmp("parent-dir");
    let h = dir.join("link/../.reeve-hall-dev");
    refused_everywhere(None, h.to_str().unwrap(), "no . or .. in the path");
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
fn refuses_the_default_hall_session() {
    let h = home("refuse-default");
    for cmd in ALL {
        for name in ["default", "Default"] {
            let mut args = vec![
                "--dry-run",
                "--home",
                h.to_str().unwrap(),
                "--session",
                name,
            ];
            args.extend_from_slice(&cmd[1..]);
            refused(&script(cmd[0], &args), "never the test setup's");
        }
        let mut args = vec![
            "--dry-run",
            "--home",
            h.to_str().unwrap(),
            "--session",
            "work",
        ];
        args.extend_from_slice(&cmd[1..]);
        refused(
            &script(cmd[0], &args),
            "test hall sessions are named hall-dev*",
        );
        // The second session is gone (decision 38).
        let mut args = vec![
            "--dry-run",
            "--home",
            h.to_str().unwrap(),
            "--session-2",
            "hall-dev-2",
        ];
        args.extend_from_slice(&cmd[1..]);
        let out = script(cmd[0], &args);
        assert_eq!(out.status.code(), Some(2), "{out:?}");
        assert!(String::from_utf8_lossy(&out.stderr).contains("unknown option --session-2"));
    }
}

#[test]
fn inputs_are_required() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("scripts-dry-run/conf");
    std::fs::create_dir_all(&dir).unwrap();
    let conf = dir.join("no-home.conf");
    std::fs::write(
        &conf,
        "HALL_DEV_SESSION=hall-dev\nHALL_DEV_ROSTER=(\"reeve ansel\")\n",
    )
    .unwrap();
    let out = script(
        "test-setup",
        &["--dry-run", "--config", conf.to_str().unwrap(), "up"],
    );
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stderr).contains("no reeve home"));
}

/// Where the shipped config finds reeve's listing command, under the HOME the
/// dry runs see.
fn reeve_listing() -> String {
    format!(
        "{}/yongshq/reeve/bin/reeve-listing",
        std::env::var("HOME").unwrap()
    )
}

#[test]
fn up_with_the_reeve_listing_starts_one_test_reeve_and_places_no_hand() {
    let h = home("up-reeve-listing");
    let out = script(
        "test-setup",
        &[
            "--dry-run",
            "--home",
            h.to_str().unwrap(),
            "--listing",
            "reeve",
            "up",
        ],
    );
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let cmds = dry_output("test-setup", &h, &["--listing", "reeve", "up"], out);
    assert!(
        stdout.contains(&format!(
            "  listing:         reeve's, {} --home {}",
            reeve_listing(),
            h.display()
        )),
        "{stdout}"
    );
    assert_eq!(count(&cmds, "claude --model"), 1);
    assert_eq!(count(&cmds, " workspace create "), 1);
    assert_eq!(count(&cmds, " --label ansel "), 1);
    assert_eq!(count(&cmds, " tab create "), 0);
    assert_eq!(count(&cmds, "REEVE_HAND=\""), 0);
    assert_eq!(count(&cmds, " place "), 1);
    assert_eq!(count(&cmds, "+ write "), 2);
    // The listing up prints is reeve's, run on the test home; the stand-in's is not.
    assert_eq!(
        count(
            &cmds,
            &format!("+ {} --home {}", reeve_listing(), h.display())
        ),
        1,
        "{cmds:?}"
    );
    assert_eq!(count(&cmds, " listing"), 0, "{cmds:?}");
}

#[test]
fn drive_finds_a_dispatched_hand_in_the_reeve_listing() {
    let h = home("drive-reeve-listing");
    let cmds = dry("test-drive", &h, &["--listing", "reeve", "scout-1", "deny"]);
    assert_eq!(
        count(
            &cmds,
            &format!("+ {} --home {}", reeve_listing(), h.display())
        ),
        1,
        "{cmds:?}"
    );
    assert_eq!(
        count(&cmds, " agent send-keys '<pane-of-scout-1>' esc --session "),
        1,
        "{cmds:?}"
    );
    // With the stand-in's listing a name off the roster is refused.
    let out = script(
        "test-drive",
        &[
            "--dry-run",
            "--home",
            h.to_str().unwrap(),
            "scout-1",
            "deny",
        ],
    );
    refused(&out, "scout-1 is not in the roster");
}

#[test]
fn the_reeve_listing_takes_one_test_reeve_and_a_plain_command() {
    let dir = tmp("conf-reeve-listing");
    let conf = |name: &str, extra: &str| {
        let conf = dir.join(name);
        std::fs::write(
            &conf,
            format!(
                "HALL_DEV_HOME=\"{}\"\nHALL_DEV_SESSION=hall-dev\nHALL_DEV_LISTING=reeve\nHALL_DEV_ROSTER=(\"reeve ansel\")\nHALL_DEV_REEVE_LISTING=/opt/reeve/bin/reeve-listing\n{extra}",
                dir.join(".reeve-hall-dev").display()
            ),
        )
        .unwrap();
        script(
            "test-setup",
            &["--dry-run", "--config", conf.to_str().unwrap(), "up"],
        )
    };
    refused(
        &conf(
            "two.conf",
            "HALL_DEV_REEVE_ROSTER=(\"reeve ansel\" \"reeve bertil\")\n",
        ),
        "one 'reeve <name>' line",
    );
    refused(
        &conf(
            "hand.conf",
            "HALL_DEV_REEVE_ROSTER=(\"errand ansel-fix ansel artificer\")\n",
        ),
        "places no test hand",
    );
    refused(
        &conf(
            "odd.conf",
            "HALL_DEV_REEVE_ROSTER=(\"reeve ansel\")\nHALL_DEV_REEVE_LISTING='reeve-listing; rm'\n",
        ),
        "refusing reeve listing command",
    );
    let out = conf("ok.conf", "HALL_DEV_REEVE_ROSTER=(\"reeve ansel\")\n");
    assert!(out.status.success(), "{out:?}");
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("+ /opt/reeve/bin/reeve-listing --home ")
    );

    let out = script("test-setup", &["--dry-run", "--listing", "other", "up"]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stderr).contains("unknown listing other"));
}
