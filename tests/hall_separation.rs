//! hall: hall's CLI and client never follow herdr's inbound environment names
//! (decision 33, finding 5 of review-mend-fork-plan-3). Throwaway listener
//! sockets stand where `HERDR_SOCKET_PATH` and `HERDR_SESSION` point; none of
//! them may see a connection. The control cases point `HALL_SOCKET_PATH` and
//! `HALL_SESSION` at the same kind of listener and must connect, so a silent
//! listener means hall ignored the name, not that the listener was unreachable.
//! Everything runs under a temporary HOME, XDG_CONFIG_HOME and XDG_STATE_HOME.

#![cfg(unix)]

use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Short, since a unix socket path is limited to 104 bytes on macOS.
fn unique_test_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = PathBuf::from(format!("/tmp/hsep-{tag}-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

struct Listener {
    path: PathBuf,
    socket: UnixListener,
}

impl Listener {
    fn bind(path: PathBuf) -> Listener {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let socket = UnixListener::bind(&path).unwrap();
        socket.set_nonblocking(true).unwrap();
        Listener { path, socket }
    }

    fn accepted(&self) -> bool {
        self.socket.accept().is_ok()
    }
}

struct Rig {
    base: PathBuf,
    /// Where `HERDR_SOCKET_PATH` points, and the client socket hall would derive from it.
    herdr_api: Listener,
    herdr_client: Listener,
    /// The session `HERDR_SESSION` names, as hall's own folders would lay it out.
    session_api: Listener,
    session_client: Listener,
}

const SESSION: &str = "leak";

impl Rig {
    fn new(tag: &str) -> Rig {
        let base = unique_test_dir(tag);
        let sessions = base.join("c/hall-dev/sessions").join(SESSION);
        Rig {
            herdr_api: Listener::bind(base.join("h/herdr.sock")),
            herdr_client: Listener::bind(base.join("h/herdr-client.sock")),
            session_api: Listener::bind(sessions.join("herdr.sock")),
            session_client: Listener::bind(sessions.join("herdr-client.sock")),
            base,
        }
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_hall"));
        cmd.args(args)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", self.base.join("home"))
            .env("XDG_CONFIG_HOME", self.base.join("c"))
            .env("XDG_STATE_HOME", self.base.join("s"))
            .env("HERDR_DISABLE_SOUND", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        cmd
    }

    fn listeners(&self) -> [&Listener; 4] {
        [
            &self.herdr_api,
            &self.herdr_client,
            &self.session_api,
            &self.session_client,
        ]
    }
}

impl Drop for Rig {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.base);
    }
}

/// Runs the child until it exits or `window` passes, then kills it, watching
/// every listener the whole time; returns the listeners that saw a connection.
fn run_watching<'a>(child: Child, listeners: &[&'a Listener], window: Duration) -> Vec<&'a Path> {
    run_collecting(child, listeners, window).hit
}

struct Run<'a> {
    hit: Vec<&'a Path>,
    /// None when the child had to be killed.
    code: Option<i32>,
    stderr: String,
}

/// As `run_watching`, and also keeps the exit code and stderr.
fn run_collecting<'a>(mut child: Child, listeners: &[&'a Listener], window: Duration) -> Run<'a> {
    let mut hit = Vec::new();
    let deadline = Instant::now() + window;
    let mut exited_at = None;
    let mut code = None;
    loop {
        for l in listeners {
            if l.accepted() && !hit.contains(&l.path.as_path()) {
                hit.push(l.path.as_path());
            }
        }
        if exited_at.is_none() {
            if let Some(status) = child.try_wait().unwrap() {
                exited_at = Some(Instant::now());
                code = status.code();
            }
        }
        // Keep watching a little after the child is gone, for a late connect.
        let done = exited_at.is_some_and(|t| t.elapsed() > Duration::from_millis(300));
        if done || Instant::now() > deadline {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let _ = child.kill();
    let _ = child.wait();
    let mut stderr = String::new();
    if let Some(mut pipe) = child.stderr.take() {
        let _ = std::io::Read::read_to_string(&mut pipe, &mut stderr);
    }
    Run { hit, code, stderr }
}

const CLI: &[&str] = &["status", "server", "--json"];
const CLIENT: &[&str] = &["client"];

#[test]
fn cli_and_client_ignore_herdr_socket_path() {
    let rig = Rig::new("sock");
    for args in [CLI, CLIENT] {
        let child = rig
            .command(args)
            .env("HERDR_SOCKET_PATH", &rig.herdr_api.path)
            .env("HERDR_CLIENT_SOCKET_PATH", &rig.herdr_client.path)
            .env("HERDR_ENV", "1")
            .spawn()
            .unwrap();
        let hit = run_watching(child, &rig.listeners(), Duration::from_secs(5));
        assert!(hit.is_empty(), "hall {args:?} connected to {hit:?}");
    }
}

#[test]
fn cli_and_client_ignore_herdr_session() {
    let rig = Rig::new("sess");
    for args in [CLI, CLIENT] {
        let child = rig
            .command(args)
            .env("HERDR_SESSION", SESSION)
            .spawn()
            .unwrap();
        let hit = run_watching(child, &rig.listeners(), Duration::from_secs(5));
        assert!(hit.is_empty(), "hall {args:?} connected to {hit:?}");
    }
}

#[test]
fn hall_socket_path_does_connect() {
    let rig = Rig::new("ctl");
    let child = rig
        .command(CLI)
        .env("HALL_SOCKET_PATH", &rig.herdr_api.path)
        .spawn()
        .unwrap();
    let hit = run_watching(child, &[&rig.herdr_api], Duration::from_secs(5));
    assert_eq!(
        hit,
        [rig.herdr_api.path.as_path()],
        "the CLI did not reach HALL_SOCKET_PATH"
    );

    // The client derives its socket from HALL_SOCKET_PATH: herdr.sock -> herdr-client.sock.
    let child = rig
        .command(CLIENT)
        .env("HALL_SOCKET_PATH", &rig.herdr_api.path)
        .spawn()
        .unwrap();
    let hit = run_watching(child, &[&rig.herdr_client], Duration::from_secs(5));
    assert_eq!(
        hit,
        [rig.herdr_client.path.as_path()],
        "the client did not reach HALL_SOCKET_PATH's client socket"
    );
}

#[test]
fn hall_session_does_connect() {
    let rig = Rig::new("ctls");
    let child = rig
        .command(CLI)
        .env("HALL_SESSION", SESSION)
        .spawn()
        .unwrap();
    let hit = run_watching(child, &[&rig.session_api], Duration::from_secs(5));
    assert_eq!(
        hit,
        [rig.session_api.path.as_path()],
        "the CLI did not reach HALL_SESSION's socket"
    );

    let child = rig
        .command(CLIENT)
        .env("HALL_SESSION", SESSION)
        .spawn()
        .unwrap();
    let hit = run_watching(child, &[&rig.session_client], Duration::from_secs(5));
    assert_eq!(
        hit,
        [rig.session_client.path.as_path()],
        "the client did not reach HALL_SESSION's client socket"
    );
}

/// Decision 44b: hall sets HALL_ENV=1 into its own panes, and the nested guard
/// reads it, so hall started inside a hall pane is refused before it connects.
/// HERDR_ENV=1 alone (a live herdr pane) is covered above: there hall runs.
#[test]
fn nested_guard_refuses_inside_a_hall_pane() {
    let rig = Rig::new("nest");
    let child = rig
        .command(CLIENT)
        .env("HALL_ENV", "1")
        .env("HERDR_ENV", "1")
        .env("HALL_SOCKET_PATH", &rig.herdr_api.path)
        .spawn()
        .unwrap();
    let run = run_collecting(child, &rig.listeners(), Duration::from_secs(5));
    assert!(
        run.hit.is_empty(),
        "nested hall client connected to {:?}",
        run.hit
    );
    assert_eq!(run.code, Some(1), "stderr: {}", run.stderr);
    assert!(
        run.stderr.contains("nested hall is disabled"),
        "stderr: {}",
        run.stderr
    );
}

/// Decision 44b: a pane command's default target comes from HALL_PANE_ID only.
/// A live herdr pane's HERDR_PANE_ID is ignored, so `--current` there has no
/// pane and is refused before any request; HALL_PANE_ID is the control.
#[test]
fn pane_target_default_reads_hall_pane_id_only() {
    let rig = Rig::new("pane");
    let split = &["pane", "split", "--direction", "down", "--current"];
    let child = rig
        .command(split)
        .env("HERDR_PANE_ID", "w1:p1")
        .env("HALL_SOCKET_PATH", &rig.herdr_api.path)
        .spawn()
        .unwrap();
    let run = run_collecting(child, &[&rig.herdr_api], Duration::from_secs(5));
    assert!(
        run.hit.is_empty(),
        "split with only HERDR_PANE_ID connected"
    );
    assert_eq!(run.code, Some(2), "stderr: {}", run.stderr);
    assert!(
        run.stderr.contains("--current requires HALL_PANE_ID"),
        "stderr: {}",
        run.stderr
    );

    let child = rig
        .command(split)
        .env("HALL_PANE_ID", "w1:p1")
        .env("HALL_SOCKET_PATH", &rig.herdr_api.path)
        .spawn()
        .unwrap();
    let hit = run_watching(child, &[&rig.herdr_api], Duration::from_secs(5));
    assert_eq!(
        hit,
        [rig.herdr_api.path.as_path()],
        "split with HALL_PANE_ID did not reach HALL_SOCKET_PATH"
    );
}

/// HERDR_CONFIG_PATH is never followed: a config write (`hall channel set`,
/// through the same config path as the onboarding write) lands in hall's own
/// XDG config and leaves the HERDR_CONFIG_PATH file as it was.
/// HALL_CONFIG_PATH is the control. The onboarding write itself is covered by
/// `onboarding_write_ignores_herdr_config_path` in the client shell tests.
#[test]
fn config_writes_ignore_herdr_config_path() {
    let rig = Rig::new("conf");
    let herdr_config = rig.base.join("herdr-config.toml");
    let herdr_content = "[terminal]\ndefault_shell = \"zsh\"\n";
    std::fs::write(&herdr_config, herdr_content).unwrap();
    let hall_xdg_config = rig.base.join("c/hall-dev/config.toml");

    let help = rig
        .command(&["--help"])
        .env("HERDR_CONFIG_PATH", &herdr_config)
        .stdout(Stdio::piped())
        .output()
        .unwrap();
    let help = String::from_utf8_lossy(&help.stdout);
    assert!(
        help.contains(&format!("Config: {}", hall_xdg_config.display())),
        "--help: {help}"
    );

    // The write happens before the (refused) self-update, which exits 1.
    let child = rig
        .command(&["channel", "set", "stable"])
        .env("HERDR_CONFIG_PATH", &herdr_config)
        .spawn()
        .unwrap();
    run_collecting(child, &[], Duration::from_secs(5));
    let written = std::fs::read_to_string(&hall_xdg_config).unwrap();
    assert!(written.contains("channel = \"stable\""), "{written}");
    assert_eq!(
        std::fs::read_to_string(&herdr_config).unwrap(),
        herdr_content
    );

    // Control: HALL_CONFIG_PATH is followed.
    let hall_config = rig.base.join("hall-config.toml");
    let child = rig
        .command(&["channel", "set", "stable"])
        .env("HERDR_CONFIG_PATH", &herdr_config)
        .env("HALL_CONFIG_PATH", &hall_config)
        .spawn()
        .unwrap();
    run_collecting(child, &[], Duration::from_secs(5));
    let written = std::fs::read_to_string(&hall_config).unwrap();
    assert!(written.contains("channel = \"stable\""), "{written}");
    assert_eq!(
        std::fs::read_to_string(&herdr_config).unwrap(),
        herdr_content
    );
}
