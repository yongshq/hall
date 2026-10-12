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
fn run_watching<'a>(
    mut child: Child,
    listeners: &[&'a Listener],
    window: Duration,
) -> Vec<&'a Path> {
    let mut hit = Vec::new();
    let deadline = Instant::now() + window;
    let mut exited_at = None;
    loop {
        for l in listeners {
            if l.accepted() && !hit.contains(&l.path.as_path()) {
                hit.push(l.path.as_path());
            }
        }
        if exited_at.is_none() && child.try_wait().unwrap().is_some() {
            exited_at = Some(Instant::now());
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
    hit
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
