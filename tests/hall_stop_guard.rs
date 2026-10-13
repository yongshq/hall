//! hall: hall never stops or hands off a server that is not hall's, and never
//! tells the operator to (decision 45). A fake server answers ping as herdr
//! v0.9.0 does (protocol 22, endpoint generation 1) and records every request;
//! none of hall's stop, handoff, CLI or client paths may send it `server.stop`
//! or `server.live_handoff`, and nothing hall prints may give a stop command.
//! The control starts a real hall server and stops it with `hall server stop`.
//! Everything runs under a temporary HOME, XDG_CONFIG_HOME and XDG_STATE_HOME,
//! with the environment cleared.

#![cfg(unix)]

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Short, since a unix socket path is limited to 104 bytes on macOS.
fn unique_test_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = PathBuf::from(format!("/tmp/hstop-{tag}-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A server that answers ping as herdr v0.9.0 and records every request line.
/// It also listens on the client socket hall derives from the API socket, so
/// a client launch finds a server running instead of spawning one.
struct FakeHerdr {
    requests: Arc<Mutex<Vec<String>>>,
    running: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl FakeHerdr {
    fn bind(api: &Path) -> FakeHerdr {
        std::fs::create_dir_all(api.parent().unwrap()).unwrap();
        let api_listener = UnixListener::bind(api).unwrap();
        let client_listener = UnixListener::bind(api.with_file_name("herdr-client.sock")).unwrap();
        api_listener.set_nonblocking(true).unwrap();
        client_listener.set_nonblocking(true).unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let running = Arc::new(AtomicBool::new(true));
        let handle = {
            let requests = requests.clone();
            let running = running.clone();
            std::thread::spawn(move || {
                while running.load(Ordering::Relaxed) {
                    if let Ok((stream, _)) = api_listener.accept() {
                        let requests = requests.clone();
                        std::thread::spawn(move || answer(stream, &requests));
                    }
                    // A client connection is only held open and dropped.
                    let _ = client_listener.accept();
                    std::thread::sleep(Duration::from_millis(5));
                }
            })
        };
        FakeHerdr {
            requests,
            running,
            handle: Some(handle),
        }
    }

    fn requests(&self) -> Vec<String> {
        self.requests.lock().unwrap().clone()
    }
}

impl Drop for FakeHerdr {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn answer(stream: UnixStream, requests: &Mutex<Vec<String>>) {
    stream.set_nonblocking(false).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut writer = stream.try_clone().unwrap();
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    while matches!(reader.read_line(&mut line), Ok(n) if n > 0) {
        requests.lock().unwrap().push(line.trim().to_string());
        let request: serde_json::Value = serde_json::from_str(&line).unwrap_or_default();
        let id = request["id"].clone();
        let response = if request["method"] == "ping" {
            serde_json::json!({"id": id, "result": {
                "type": "pong",
                "version": "0.9.0",
                "protocol": 22,
                "capabilities": {
                    "live_handoff": true,
                    "detached_server_daemon": true,
                    "endpoint_protocol_generation": 1,
                    "surface_interest": true,
                    "health_check": true
                }
            }})
        } else {
            // herdr honours server.stop before any protocol check.
            serde_json::json!({"id": id, "result": {"type": "ok"}})
        };
        let _ = writeln!(writer, "{response}");
        let _ = writer.flush();
        line.clear();
    }
}

struct Rig {
    base: PathBuf,
}

const SESSION: &str = "leak";

impl Rig {
    fn new(tag: &str) -> Rig {
        Rig {
            base: unique_test_dir(tag),
        }
    }

    /// Where `HALL_SOCKET_PATH` points in the tests that use it.
    fn socket(&self) -> PathBuf {
        self.base.join("h/herdr.sock")
    }

    /// The API socket of hall session `name` under the temporary config.
    fn session_socket(&self, name: &str) -> PathBuf {
        self.base
            .join("c/hall-dev/sessions")
            .join(name)
            .join("herdr.sock")
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_hall"));
        cmd.args(args)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("SHELL", "/bin/sh")
            .env("HOME", self.base.join("home"))
            .env("XDG_CONFIG_HOME", self.base.join("c"))
            .env("XDG_STATE_HOME", self.base.join("s"))
            .env("XDG_RUNTIME_DIR", self.base.join("r"))
            .env("HERDR_DISABLE_SOUND", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        cmd
    }
}

impl Drop for Rig {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.base);
    }
}

/// Runs the command to completion, killing it after `window`.
fn run(mut cmd: Command, window: Duration) -> Output {
    let mut child = cmd.spawn().unwrap();
    let deadline = Instant::now() + window;
    while child.try_wait().unwrap().is_none() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    let _ = child.kill();
    child.wait_with_output().unwrap()
}

/// The fake saw a ping and nothing that stops or hands off a server; hall
/// failed and printed that the socket is not hall's, with no stop command.
fn assert_refused(args: &[&str], fake: &FakeHerdr, output: &Output) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let requests = fake.requests();
    assert!(
        requests.iter().any(|r| r.contains("\"ping\"")),
        "hall {args:?} never pinged: {requests:?}\nstderr: {stderr}"
    );
    for request in &requests {
        assert!(
            !request.contains("server.stop") && !request.contains("live_handoff"),
            "hall {args:?} sent {request}"
        );
    }
    assert_ne!(output.status.code(), Some(0), "hall {args:?} succeeded");
    assert!(
        stderr.contains("this socket is not a hall server"),
        "hall {args:?} stderr: {stderr}"
    );
    for text in [&stdout, &stderr] {
        for stop in ["server stop", "session stop", "Stop the old server"] {
            assert!(
                !text.contains(stop),
                "hall {args:?} printed a stop command: {text}"
            );
        }
    }
}

#[test]
fn server_stop_refuses_a_server_that_is_not_hall() {
    let rig = Rig::new("srv");
    let fake = FakeHerdr::bind(&rig.socket());
    let args = &["server", "stop"];
    let mut cmd = rig.command(args);
    cmd.env("HALL_SOCKET_PATH", rig.socket());
    let output = run(cmd, Duration::from_secs(10));
    assert_refused(args, &fake, &output);
}

#[test]
fn session_stop_refuses_a_server_that_is_not_hall() {
    let rig = Rig::new("ses");
    let fake = FakeHerdr::bind(&rig.session_socket(SESSION));
    let args = &["session", "stop", SESSION];
    let output = run(rig.command(args), Duration::from_secs(10));
    assert_refused(args, &fake, &output);
}

#[test]
fn live_handoff_refuses_a_server_that_is_not_hall() {
    let rig = Rig::new("lho");
    let fake = FakeHerdr::bind(&rig.socket());
    let args = &["server", "live-handoff"];
    let mut cmd = rig.command(args);
    cmd.env("HALL_SOCKET_PATH", rig.socket());
    let output = run(cmd, Duration::from_secs(10));
    assert_refused(args, &fake, &output);
}

#[test]
fn cli_request_gives_no_stop_command_for_a_server_that_is_not_hall() {
    let rig = Rig::new("pane");
    let fake = FakeHerdr::bind(&rig.socket());
    let args = &["pane", "list"];
    let mut cmd = rig.command(args);
    cmd.env("HALL_SOCKET_PATH", rig.socket());
    let output = run(cmd, Duration::from_secs(10));
    assert_refused(args, &fake, &output);
}

#[test]
fn client_launch_gives_no_stop_command_for_a_server_that_is_not_hall() {
    let rig = Rig::new("cli");
    let fake = FakeHerdr::bind(&rig.socket());
    let mut cmd = rig.command(&[]);
    cmd.env("HALL_SOCKET_PATH", rig.socket());
    let output = run(cmd, Duration::from_secs(10));
    assert_refused(&[], &fake, &output);
}

#[test]
fn status_names_a_server_that_is_not_hall() {
    let rig = Rig::new("st");
    let fake = FakeHerdr::bind(&rig.socket());
    for args in [
        &["status"][..],
        &["status", "server"],
        &["status", "--json"],
    ] {
        let mut cmd = rig.command(args);
        cmd.env("HALL_SOCKET_PATH", rig.socket());
        let output = run(cmd, Duration::from_secs(10));
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(0), "hall {args:?}: {stderr}");
        if args.contains(&"--json") {
            let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
            assert_eq!(json["server"]["hall_server"], false, "{stdout}");
            assert_eq!(json["server"]["restart_needed"], serde_json::Value::Null);
            assert_eq!(json["update"]["restart_needed"], serde_json::Value::Null);
        } else {
            assert!(
                stdout.contains("hall_server: no"),
                "hall {args:?}: {stdout}"
            );
            assert!(
                stdout.contains("this socket is not a hall server"),
                "hall {args:?}: {stdout}"
            );
        }
        assert!(!stdout.contains("restart_needed: yes"), "{stdout}");
        for stop in ["server stop", "session stop", "Stop the old server"] {
            assert!(!stdout.contains(stop), "hall {args:?}: {stdout}");
        }
    }
    let requests = fake.requests();
    assert!(!requests.is_empty());
    for request in &requests {
        assert!(request.contains("\"ping\""), "hall status sent {request}");
    }
}

/// A listener that accepts on both sockets and never answers.
struct SilentListener {
    running: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl SilentListener {
    fn bind(api: &Path) -> SilentListener {
        std::fs::create_dir_all(api.parent().unwrap()).unwrap();
        let listeners = [
            UnixListener::bind(api).unwrap(),
            UnixListener::bind(api.with_file_name("herdr-client.sock")).unwrap(),
        ];
        let running = Arc::new(AtomicBool::new(true));
        let handle = {
            let running = running.clone();
            std::thread::spawn(move || {
                let mut held = Vec::new();
                for listener in &listeners {
                    listener.set_nonblocking(true).unwrap();
                }
                while running.load(Ordering::Relaxed) {
                    for listener in &listeners {
                        if let Ok((stream, _)) = listener.accept() {
                            held.push(stream);
                        }
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
            })
        };
        SilentListener {
            running,
            handle: Some(handle),
        }
    }
}

impl Drop for SilentListener {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

#[test]
fn client_launch_says_hall_cannot_confirm_a_silent_listener() {
    let rig = Rig::new("sil");
    let _silent = SilentListener::bind(&rig.socket());
    let mut cmd = rig.command(&[]);
    cmd.env("HALL_SOCKET_PATH", rig.socket());
    let output = run(cmd, Duration::from_secs(15));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "stderr: {stderr}");
    assert!(
        stderr.contains("hall cannot confirm it is a hall server"),
        "stderr: {stderr}"
    );
    assert!(!stderr.contains("os error"), "stderr: {stderr}");
    assert!(!stderr.contains("server stop"), "stderr: {stderr}");
}

/// Kills the real hall server if the test fails before it stops.
struct ServerGuard(Child);

impl Drop for ServerGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn server_stop_still_stops_a_hall_server() {
    let rig = Rig::new("ctl");
    let session = "ctl";
    std::fs::create_dir_all(rig.base.join("c/hall-dev")).unwrap();
    std::fs::write(
        rig.base.join("c/hall-dev/config.toml"),
        "onboarding = false\n",
    )
    .unwrap();
    let mut server = rig.command(&["server"]);
    server
        .env("HALL_SESSION", session)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut server = ServerGuard(server.spawn().unwrap());

    let socket = rig.session_socket(session);
    let deadline = Instant::now() + Duration::from_secs(15);
    while UnixStream::connect(&socket).is_err() {
        assert!(Instant::now() < deadline, "hall server never listened");
        assert!(
            server.0.try_wait().unwrap().is_none(),
            "hall server exited early"
        );
        std::thread::sleep(Duration::from_millis(25));
    }

    let mut stop = rig.command(&["server", "stop"]);
    stop.env("HALL_SESSION", session);
    let output = run(stop, Duration::from_secs(20));
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let deadline = Instant::now() + Duration::from_secs(10);
    while server.0.try_wait().unwrap().is_none() {
        assert!(Instant::now() < deadline, "hall server did not exit");
        std::thread::sleep(Duration::from_millis(25));
    }
}
