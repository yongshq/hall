//! hall: reeve's listing, read by the client for the reeve tree (plan step 3).
//!
//! The listing is the draft in docs/listing-draft.md: every reeve and every errand in flight,
//! with where each one sits. Hall never reads status files (ADR 0002); it runs the command the
//! config names (`[reeve] listing_command`, decision 49) and reads its JSON, killing a run that
//! outlasts `[reeve] listing_timeout_ms` (decision 56), and keeps a record of each run so the
//! next client kills a run that a killed client left behind (decision 58).

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
#[cfg(not(unix))]
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::ClientLoopEvent;

/// What one read of the listing gave: the listing, or why there is none.
pub(super) type ReeveListingRead = Result<ReeveListing, String>;

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub(super) struct ReeveListing {
    #[serde(default)]
    pub(super) reeves: Vec<ListedReeve>,
    #[serde(default)]
    pub(super) errands: Vec<ListedErrand>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(super) struct ListedReeve {
    pub(super) name: String,
    pub(super) session: ListedSession,
    #[serde(default = "listed_live_default")]
    pub(super) live: bool,
}

fn listed_live_default() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(super) struct ListedErrand {
    pub(super) id: String,
    #[serde(default)]
    pub(super) office: String,
    pub(super) owner_reeve: String,
    pub(super) hand_state: HandState,
    #[serde(default)]
    pub(super) question: Option<String>,
    #[serde(default)]
    pub(super) waiting: bool,
    #[serde(default)]
    pub(super) idle: bool,
    #[serde(default)]
    pub(super) wedged: bool,
    pub(super) session: ListedSession,
}

/// A session location: the hall session (the draft still names the field `herdr_session`) and
/// the pane in it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(super) struct ListedSession {
    pub(super) herdr_session: String,
    #[serde(default)]
    pub(super) pane_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum HandState {
    Working,
    NeedsDecision,
    Blocked,
    Done,
    Failed,
    /// A state this hall does not know yet; shown, never counted.
    #[serde(other)]
    Unknown,
}

impl ListedErrand {
    /// Whether the hand counts toward its reeve's badge: what is on disk only (decision 17), a
    /// hand counted once however many of these hold (decision 51).
    pub(super) fn waits_on_liege(&self) -> bool {
        self.question
            .as_deref()
            .is_some_and(|question| !question.is_empty())
            || matches!(
                self.hand_state,
                HandState::NeedsDecision | HandState::Blocked
            )
            || self.waiting
            || self.idle
            || self.wedged
    }
}

/// The listing command, how often to run it and how long one run may take (decisions 49, 56).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ReeveListingSpec {
    pub(super) command: Vec<String>,
    pub(super) interval: Duration,
    pub(super) timeout: Duration,
}

impl ReeveListingSpec {
    /// None when no listing command is configured, which leaves herdr's sidebar.
    pub(super) fn from_config(config: &crate::config::ReeveConfig) -> Option<Self> {
        (!config.listing_command.is_empty()).then(|| Self {
            command: config.listing_command.clone(),
            interval: Duration::from_millis(config.listing_interval_ms.max(100)),
            timeout: Duration::from_millis(config.listing_timeout_ms.max(100)),
        })
    }
}

/// The most a listing may print; a larger one is refused rather than read without end.
const MAX_LISTING_BYTES: usize = 8 << 20;
/// How often a running listing command is checked for its end, its deadline and a stop.
const POLL: Duration = Duration::from_millis(10);
/// How long stopping the reader waits for it to kill and reap a running command.
const STOP_WAIT: Duration = Duration::from_secs(2);
/// How long the output may stay open once the command has exited: then whatever it left behind
/// in its group is killed, the output is closed and what the command printed is read
/// (decisions 58 and 60).
const EXIT_GRACE: Duration = Duration::from_millis(100);

/// Runs the listing command once and parses what it prints, killing it after `timeout`.
#[cfg(test)]
pub(super) fn read_listing(command: &[String], timeout: Duration) -> ReeveListingRead {
    let (_stop_tx, stop_rx) = mpsc::channel();
    read_listing_until(command, timeout, &stop_rx, None).unwrap_or_else(|| Err("stopped".into()))
}

/// Runs the listing command once, in a process group of its own, and parses what it prints.
/// When the command outlasts `timeout`, or `stop` says to stop (None then), the command and
/// everything it started in its group are killed and the command is reaped: nothing is left.
/// When the command exits but something it started holds its output past `EXIT_GRACE`, that
/// group is killed, the output closed and what the command printed is read (on Unix; on
/// Windows the run times out). While it runs, a record of it sits in `runs`, for the next
/// client should this one be killed (decision 58).
fn read_listing_until(
    command: &[String],
    timeout: Duration,
    stop: &mpsc::Receiver<()>,
    runs: Option<&Path>,
) -> Option<ReeveListingRead> {
    let Some((program, args)) = command.split_first() else {
        return Some(Err("no listing command".into()));
    };
    let mut process = Command::new(program);
    process
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // A process group of its own, as herdr's tab bar status commands get (`tab_bar_status.rs`).
    // Not on Windows, where that spawns the command suspended until herdr's job guard, which
    // this reader does not use, resumes it: there the command alone is killed.
    #[cfg(unix)]
    crate::platform::configure_status_command(&mut process);
    let mut child = match process.spawn() {
        Ok(child) => child,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Some(Err(format!("not found: {program}")));
        }
        Err(error) => return Some(Err(format!("{program}: {error}"))),
    };
    let _record = runs.and_then(|runs| RunRecord::write(runs, &child, command));
    let mut stdout = Pipe::read(child.stdout.take());
    let mut stderr = Pipe::read(child.stderr.take());
    let deadline = Instant::now() + timeout;
    let mut status = None;
    let mut exited = None;
    loop {
        if status.is_none() {
            status = child.try_wait().ok().flatten();
            if status.is_some() {
                exited = Some(Instant::now());
            }
        }
        #[cfg(unix)]
        if exited.is_some_and(|exited| exited.elapsed() >= EXIT_GRACE) {
            // The command is gone and something it started still holds its output. A child it
            // left in its group is killed with the group (the group id stays reserved while
            // that member exists); a daemon that left the group (setsid) is out of reach, so
            // the output is closed on it. Either way what the command printed is read.
            kill_group(&child);
            stdout.close();
            stderr.close();
        }
        stdout.collect();
        stderr.collect();
        if stdout
            .output()
            .is_some_and(|out| out.len() > MAX_LISTING_BYTES)
        {
            kill_and_reap(&mut child);
            return Some(Err("listing larger than 8 MiB".into()));
        }
        if let (Some(status), Some(stdout), Some(stderr)) =
            (status, stdout.output(), stderr.output())
        {
            return Some(listing_from_output(status, stdout, stderr));
        }
        let stopped = wait(&stdout, &stderr, stop);
        if stopped || Instant::now() >= deadline {
            kill_and_reap(&mut child);
            return (!stopped).then(|| Err(format!("timed out after {}", seconds(timeout))));
        }
    }
}

fn listing_from_output(
    status: std::process::ExitStatus,
    stdout: &[u8],
    stderr: &[u8],
) -> ReeveListingRead {
    if !status.success() {
        let stderr = String::from_utf8_lossy(stderr);
        let reason = stderr
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map_or_else(|| status.to_string(), str::to_owned);
        return Err(reason);
    }
    serde_json::from_slice(stdout).map_err(|error| format!("unreadable JSON: {error}"))
}

/// One of the command's output pipes, read without blocking as it fills, on the reader's own
/// thread: no thread of its own waits on a pipe that something past the command's reach (a
/// daemon) holds open. Read to its end, past the size cap or until closed.
#[cfg(unix)]
struct Pipe {
    pipe: Option<std::fs::File>,
    output: Vec<u8>,
}

#[cfg(unix)]
impl Pipe {
    fn read(pipe: Option<impl Into<std::os::fd::OwnedFd>>) -> Self {
        use std::os::fd::AsRawFd;
        let pipe = pipe.map(Into::into).map(std::fs::File::from);
        // A pipe that cannot be read without blocking is not read at all: a blocking read here
        // would hold the reader past its deadline.
        Self {
            pipe: pipe.filter(|pipe| crate::pty::fd::set_nonblocking(pipe.as_raw_fd()).is_ok()),
            output: Vec::new(),
        }
    }

    fn collect(&mut self) {
        let mut buffer = [0; 64 << 10];
        while let Some(pipe) = self.pipe.as_mut() {
            match pipe.read(&mut buffer) {
                Ok(0) => self.pipe = None,
                Ok(read) => {
                    self.output.extend_from_slice(&buffer[..read]);
                    if self.output.len() > MAX_LISTING_BYTES {
                        self.pipe = None;
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => return,
                Err(_) => self.pipe = None,
            }
        }
    }

    /// Reads what the pipe holds now and closes it, whoever else still holds it open.
    fn close(&mut self) {
        self.collect();
        self.pipe = None;
    }

    fn output(&self) -> Option<&[u8]> {
        self.pipe.is_none().then_some(self.output.as_slice())
    }

    fn poll_fd(&self) -> libc::pollfd {
        use std::os::fd::AsRawFd;
        libc::pollfd {
            // A negative descriptor is skipped by poll.
            fd: self.pipe.as_ref().map_or(-1, AsRawFd::as_raw_fd),
            events: libc::POLLIN,
            revents: 0,
        }
    }
}

/// Waits up to `POLL` for output on either pipe or for a stop. True when told to stop.
#[cfg(unix)]
fn wait(stdout: &Pipe, stderr: &Pipe, stop: &mpsc::Receiver<()>) -> bool {
    let mut pipes = [stdout.poll_fd(), stderr.poll_fd()];
    // SAFETY: `pipes` is an array of two `pollfd`s, as its length says.
    unsafe {
        libc::poll(
            pipes.as_mut_ptr(),
            pipes.len() as libc::nfds_t,
            POLL.as_millis() as libc::c_int,
        );
    }
    !matches!(stop.try_recv(), Err(mpsc::TryRecvError::Empty))
}

/// Waits up to `POLL` for a stop. True when told to stop.
#[cfg(not(unix))]
fn wait(_stdout: &Pipe, _stderr: &Pipe, stop: &mpsc::Receiver<()>) -> bool {
    !matches!(stop.recv_timeout(POLL), Err(RecvTimeoutError::Timeout))
}

/// One of the command's output pipes, read to its end (or past the size cap) on a thread. On
/// Windows, where nothing kills what the command left behind, a child holding the output keeps
/// this thread until that child exits (decision 60).
#[cfg(not(unix))]
enum Pipe {
    Reading(JoinHandle<Vec<u8>>),
    Read(Vec<u8>),
}

#[cfg(not(unix))]
impl Pipe {
    fn read(pipe: Option<impl Read + Send + 'static>) -> Self {
        let Some(pipe) = pipe else {
            return Self::Read(Vec::new());
        };
        Self::Reading(std::thread::spawn(move || {
            let mut output = Vec::new();
            let _ = pipe
                .take(MAX_LISTING_BYTES as u64 + 1)
                .read_to_end(&mut output);
            output
        }))
    }

    fn collect(&mut self) {
        if matches!(self, Self::Reading(reader) if reader.is_finished()) {
            let Self::Reading(reader) = std::mem::replace(self, Self::Read(Vec::new())) else {
                unreachable!()
            };
            *self = Self::Read(reader.join().unwrap_or_default());
        }
    }

    fn output(&self) -> Option<&[u8]> {
        match self {
            Self::Reading(_) => None,
            Self::Read(output) => Some(output),
        }
    }
}

/// Kills the command's whole process group, so a child it left behind holding the output pipe
/// goes too, then reaps the command. On Windows the command alone is killed.
fn kill_and_reap(child: &mut Child) {
    kill_group(child);
    let _ = child.kill();
    let _ = child.wait();
}

fn kill_group(child: &Child) {
    #[cfg(unix)]
    if let Ok(group) = libc::pid_t::try_from(child.id()) {
        // The command leads its own group, as in herdr's `StatusCommandGuard::terminate`; the
        // group id stays reserved while the unreaped command or any member of it exists.
        unsafe {
            libc::kill(-group, libc::SIGKILL);
        }
    }
    // Windows: nothing is killed here (decision 60).
    #[cfg(not(unix))]
    let _ = child;
}

/// Where each client keeps the record of its running listing command: the client's state
/// folder.
fn runs_dir() -> PathBuf {
    crate::config::state_dir().join("reeve-listing")
}

/// A process told apart from any later one given the same pid: its pid and its start time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct ProcessIdentity {
    pid: i32,
    start: u64,
}

impl ProcessIdentity {
    fn of(pid: i32) -> Option<Self> {
        process_start(pid).map(|(start, _)| Self { pid, start })
    }

    /// Whether this very process still runs.
    fn alive(self) -> bool {
        process_start(self.pid).is_some_and(|(start, _)| start == self.start)
    }

    /// Whether this very process still runs and still leads its own process group.
    #[cfg_attr(not(unix), allow(dead_code))]
    fn leads_its_group(self) -> bool {
        process_start(self.pid)
            .is_some_and(|(start, group)| start == self.start && group == self.pid)
    }
}

/// A process's start time and process group, or None when it is gone (or on a system where
/// hall cannot tell).
#[cfg(target_os = "macos")]
fn process_start(pid: i32) -> Option<(u64, i32)> {
    let mut info: libc::proc_bsdinfo = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of::<libc::proc_bsdinfo>() as libc::c_int;
    // SAFETY: `info` is a `proc_bsdinfo` of `size` bytes, as PROC_PIDTBSDINFO fills.
    let ret = unsafe {
        libc::proc_pidinfo(
            pid,
            libc::PROC_PIDTBSDINFO,
            0,
            &mut info as *mut _ as *mut libc::c_void,
            size,
        )
    };
    (ret == size).then(|| {
        (
            info.pbi_start_tvsec * 1_000_000 + info.pbi_start_tvusec,
            info.pbi_pgid as i32,
        )
    })
}

#[cfg(target_os = "linux")]
fn process_start(pid: i32) -> Option<(u64, i32)> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // After the command name: state, ppid, pgrp, ..., starttime (the 22nd field).
    let fields = stat
        .rsplit_once(')')?
        .1
        .split_whitespace()
        .collect::<Vec<_>>();
    Some((fields.get(19)?.parse().ok()?, fields.get(2)?.parse().ok()?))
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn process_start(_pid: i32) -> Option<(u64, i32)> {
    None
}

/// The record of one running listing command, in the client's state folder while it runs:
/// which client started it, the process group it leads and the command. A client killed with
/// SIGKILL cannot kill its run; the next client's reader finds the record and does
/// (`kill_leftover_runs`). Removed when the run ends.
#[derive(Debug, Serialize, Deserialize)]
struct ListingRun {
    client: ProcessIdentity,
    group: ProcessIdentity,
    command: Vec<String>,
}

struct RunRecord {
    path: PathBuf,
}

impl RunRecord {
    fn write(runs: &Path, child: &Child, command: &[String]) -> Option<Self> {
        let client = ProcessIdentity::of(i32::try_from(std::process::id()).ok()?)?;
        let group = ProcessIdentity::of(i32::try_from(child.id()).ok()?)?;
        let run = ListingRun {
            client,
            group,
            command: command.to_vec(),
        };
        let path = runs.join(format!("{}-{}.json", client.pid, group.pid));
        // Written whole, then moved into place, so a record is never read half written.
        let partial = path.with_extension("partial");
        std::fs::create_dir_all(runs).ok()?;
        std::fs::write(&partial, serde_json::to_vec(&run).ok()?).ok()?;
        std::fs::rename(&partial, &path).ok()?;
        Some(Self { path })
    }
}

impl Drop for RunRecord {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Kills every run in `runs` that a client no longer running left behind, when its command
/// still runs: the very process the run started (same pid, same start time), still leading its
/// group. Its record goes either way. A run of a client still running is that client's own. A
/// record a client was killed while writing (`.partial`) is read as a record when it is whole,
/// and removed when it is not and no process has its client's pid.
fn kill_leftover_runs(runs: &Path) {
    let Ok(entries) = std::fs::read_dir(runs) else {
        return;
    };
    for path in entries.flatten().map(|entry| entry.path()) {
        let partial = match path.extension().and_then(|extension| extension.to_str()) {
            Some("json") => false,
            Some("partial") => true,
            _ => continue,
        };
        let Some(run) = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<ListingRun>(&bytes).ok())
        else {
            let client = path
                .file_stem()
                .and_then(|stem| stem.to_str()?.split_once('-')?.0.parse().ok());
            if partial && client.is_some_and(|client| process_start(client).is_none()) {
                let _ = std::fs::remove_file(&path);
            }
            continue;
        };
        if run.client.alive() {
            continue;
        }
        #[cfg(unix)]
        if run.group.leads_its_group() {
            unsafe {
                libc::kill(-run.group.pid, libc::SIGKILL);
            }
        }
        let _ = std::fs::remove_file(&path);
    }
}

/// Seconds as herdr's status commands word a timeout (`timed out after 2s`).
fn seconds(duration: Duration) -> String {
    let millis = duration.as_millis();
    if millis.is_multiple_of(1000) {
        format!("{}s", millis / 1000)
    } else {
        format!("{:.1}s", duration.as_secs_f64())
    }
}

/// The listing reader: a thread that runs the listing command every interval and sends each
/// changed read, tagged with this reader's generation, to the client loop, as
/// `catalog_reload::watch_profiles` does for saved machines. Dropping it stops the thread and
/// waits (briefly) for it to kill and reap a running command, so a client that detaches or
/// reloads its config leaves no listing command behind.
pub(super) struct ReeveListingWatch {
    spec: ReeveListingSpec,
    generation: u64,
    stop: Option<mpsc::Sender<()>>,
    stopped: mpsc::Receiver<()>,
}

impl ReeveListingWatch {
    pub(super) fn start(
        spec: ReeveListingSpec,
        generation: u64,
        event_tx: tokio::sync::mpsc::Sender<ClientLoopEvent>,
    ) -> Self {
        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let (stopped_tx, stopped_rx) = mpsc::channel::<()>();
        let thread_spec = spec.clone();
        std::thread::spawn(move || {
            let runs = runs_dir();
            kill_leftover_runs(&runs);
            let mut previous = None;
            while let Some(current) = read_listing_until(
                &thread_spec.command,
                thread_spec.timeout,
                &stop_rx,
                Some(&runs),
            ) {
                if previous.as_ref() != Some(&current) {
                    match event_tx
                        .try_send(ClientLoopEvent::ReeveListing(generation, current.clone()))
                    {
                        Ok(()) => previous = Some(current),
                        // A busy loop gets this read on the next round instead.
                        Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {}
                        Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => break,
                    }
                }
                if !matches!(
                    stop_rx.recv_timeout(thread_spec.interval),
                    Err(RecvTimeoutError::Timeout)
                ) {
                    break;
                }
            }
            let _ = stopped_tx.send(());
        });
        Self {
            spec,
            generation,
            stop: Some(stop_tx),
            stopped: stopped_rx,
        }
    }

    pub(super) fn spec(&self) -> &ReeveListingSpec {
        &self.spec
    }

    pub(super) fn generation(&self) -> u64 {
        self.generation
    }
}

impl Drop for ReeveListingWatch {
    fn drop(&mut self) {
        drop(self.stop.take());
        let _ = self.stopped.recv_timeout(STOP_WAIT);
    }
}

/// Keeps the reader in step with the config: started, restarted with a new command, interval or
/// timeout, or stopped when the listing command is removed. True when it changed.
pub(super) fn follow(
    watch: &mut Option<ReeveListingWatch>,
    spec: Option<&ReeveListingSpec>,
    event_tx: &tokio::sync::mpsc::Sender<ClientLoopEvent>,
) -> bool {
    if watch.as_ref().map(ReeveListingWatch::spec) == spec {
        return false;
    }
    // Every reader gets a generation of its own, so a read the last one queued is never taken
    // for this one's.
    static GENERATION: AtomicU64 = AtomicU64::new(0);
    *watch = None;
    *watch = spec.map(|spec| {
        ReeveListingWatch::start(
            spec.clone(),
            GENERATION.fetch_add(1, Ordering::Relaxed),
            event_tx.clone(),
        )
    });
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    const DRAFT: &str = r#"{
      "draft": true,
      "reeves": [
        { "name": "ansel", "session": { "herdr_session": "hall-dev", "pane_id": "w1:p1" }, "live": true }
      ],
      "errands": [
        {
          "id": "ansel-fix", "office": "artificer", "owner_reeve": "ansel",
          "hand_state": "needs-decision", "question": "Which base branch?",
          "waiting": false, "idle": false, "wedged": false,
          "session": { "herdr_session": "hall-dev", "pane_id": null }
        }
      ]
    }"#;

    #[test]
    fn parses_the_draft_listing() {
        let listing: ReeveListing = serde_json::from_str(DRAFT).unwrap();
        assert_eq!(listing.reeves[0].name, "ansel");
        assert_eq!(listing.reeves[0].session.pane_id.as_deref(), Some("w1:p1"));
        let errand = &listing.errands[0];
        assert_eq!(errand.hand_state, HandState::NeedsDecision);
        assert_eq!(errand.question.as_deref(), Some("Which base branch?"));
        assert_eq!(errand.session.pane_id, None);
        assert!(errand.waits_on_liege());
    }

    #[test]
    fn an_unknown_hand_state_reads_as_unknown() {
        let listing: ReeveListing =
            serde_json::from_str(&DRAFT.replace("\"needs-decision\"", "\"pondering\"")).unwrap();
        assert_eq!(listing.errands[0].hand_state, HandState::Unknown);
    }

    #[test]
    fn a_working_hand_with_no_flag_does_not_wait_on_the_liege() {
        let mut listing: ReeveListing = serde_json::from_str(DRAFT).unwrap();
        let errand = &mut listing.errands[0];
        errand.hand_state = HandState::Working;
        errand.question = None;
        assert!(!errand.waits_on_liege());
        errand.question = Some(String::new());
        assert!(!errand.waits_on_liege(), "an empty question is no question");
        for flag in 0..3 {
            let mut errand = errand.clone();
            match flag {
                0 => errand.waiting = true,
                1 => errand.idle = true,
                _ => errand.wedged = true,
            }
            assert!(errand.waits_on_liege());
        }
        errand.hand_state = HandState::Blocked;
        assert!(errand.waits_on_liege());
        errand.hand_state = HandState::Done;
        assert!(!errand.waits_on_liege());
    }

    const TIMEOUT: Duration = Duration::from_secs(5);

    fn sh(script: &str) -> Vec<String> {
        vec!["sh".into(), "-c".into(), script.into()]
    }

    #[test]
    fn a_failing_command_reads_as_its_first_stderr_line() {
        let read = read_listing(&sh("echo; echo 'refusing home' >&2; exit 1"), TIMEOUT);
        assert_eq!(read, Err("refusing home".into()));
    }

    #[test]
    fn a_missing_program_and_bad_json_are_short_errors_not_panics() {
        assert_eq!(
            read_listing(&["/nonexistent/hall-listing".into()], TIMEOUT),
            Err("not found: /nonexistent/hall-listing".into())
        );
        let read = read_listing(&sh("echo not json"), TIMEOUT);
        assert!(read.unwrap_err().starts_with("unreadable JSON: "));
        assert_eq!(read_listing(&[], TIMEOUT), Err("no listing command".into()));
    }

    #[test]
    fn a_command_printing_the_draft_reads_it() {
        let read = read_listing(&["printf".into(), "%s".into(), DRAFT.into()], TIMEOUT).unwrap();
        assert_eq!(read.errands.len(), 1);
    }

    /// Whether a process is still in the process table (a zombie counts: it is not reaped).
    #[cfg(unix)]
    fn in_process_table(pid: i32) -> bool {
        // SAFETY: signal 0 only checks that the process exists.
        unsafe { libc::kill(pid, 0) == 0 }
    }

    /// A fresh file path for a test command to write its pid to.
    #[cfg(unix)]
    fn pid_file() -> std::path::PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        std::env::temp_dir().join(format!(
            "hall-listing-pid-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[cfg(unix)]
    fn pid_in(path: &std::path::Path) -> i32 {
        let deadline = Instant::now() + TIMEOUT;
        loop {
            if let Some(pid) = std::fs::read_to_string(path)
                .ok()
                .and_then(|text| text.trim().parse().ok())
            {
                let _ = std::fs::remove_file(path);
                return pid;
            }
            assert!(Instant::now() < deadline, "no pid in {}", path.display());
            std::thread::sleep(POLL);
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_hung_command_times_out_and_is_killed_and_reaped() {
        let pid_file = pid_file();
        let started = Instant::now();
        let read = read_listing(
            &sh(&format!(
                "echo $$ > '{}'; exec sleep 1000",
                pid_file.display()
            )),
            Duration::from_millis(300),
        );
        assert_eq!(read, Err("timed out after 0.3s".into()));
        assert!(started.elapsed() < Duration::from_secs(5));
        assert!(
            !in_process_table(pid_in(&pid_file)),
            "the hung command is gone"
        );
    }

    /// Waits, up to `TIMEOUT`, for a process this test did not start to leave the process
    /// table. A killed orphan stays there as a zombie until init reaps it, which this process
    /// cannot wait on, so a check made at once can find it still there.
    #[cfg(unix)]
    fn leaves_process_table(pid: i32) -> bool {
        let deadline = Instant::now() + TIMEOUT;
        while in_process_table(pid) {
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(POLL);
        }
        true
    }

    #[cfg(unix)]
    #[test]
    fn a_child_left_holding_the_output_is_killed_and_the_listing_read() {
        let pid_file = pid_file();
        let read = read_listing(
            &sh(&format!(
                "echo '{{}}'; sleep 1000 & echo $! > '{}'",
                pid_file.display()
            )),
            TIMEOUT,
        );
        assert_eq!(read, Ok(ReeveListing::default()), "read after the grace");
        assert!(
            leaves_process_table(pid_in(&pid_file)),
            "the background child is gone"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_daemon_holding_the_output_outside_the_group_does_not_hold_the_read() {
        let pid_file = pid_file();
        let started = Instant::now();
        // The daemon leaves the group (setsid) and writes its pid; the command waits for that
        // pid before it exits, so the daemon is out of the group before the grace ends.
        let read = read_listing(
            &sh(&format!(
                "echo '{{}}'; perl -MPOSIX -e 'setsid(); open my $f, \">\", $ARGV[0]; \
                 print $f $$; close $f; exec \"sleep\", \"1000\"' '{file}' & \
                 while [ ! -s '{file}' ]; do sleep 0.01; done",
                file = pid_file.display()
            )),
            TIMEOUT,
        );
        assert_eq!(read, Ok(ReeveListing::default()), "read after the grace");
        assert!(
            started.elapsed() < TIMEOUT / 2,
            "not held to the timeout: {:?}",
            started.elapsed()
        );
        let daemon = pid_in(&pid_file);
        // Out of the reader's reach, so still running: the test ends it.
        assert!(in_process_table(daemon), "the daemon is not in the group");
        // SAFETY: the daemon is this test's own, and its pid is held while it runs.
        unsafe {
            libc::kill(daemon, libc::SIGKILL);
        }
        assert!(leaves_process_table(daemon));
    }

    #[cfg(windows)]
    #[test]
    fn a_listing_command_runs_on_windows() {
        // Spawned without the suspended start herdr's status commands get.
        let read = read_listing(&["cmd".into(), "/C".into(), "echo {}".into()], TIMEOUT);
        assert_eq!(read, Ok(ReeveListing::default()));
    }

    /// A fresh, empty folder for a test's run records.
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn runs() -> PathBuf {
        let runs = pid_file().with_extension("runs");
        std::fs::create_dir_all(&runs).expect("runs folder");
        runs
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn records(runs: &Path) -> Vec<String> {
        let mut names = std::fs::read_dir(runs)
            .expect("runs folder")
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        names.sort();
        names
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn a_run_is_recorded_while_it_runs_and_its_record_goes_after() {
        let runs = runs();
        let seen = pid_file();
        let (_stop_tx, stop_rx) = mpsc::channel();
        let read = read_listing_until(
            &sh(&format!(
                "ls '{}' > '{}'; echo '{{}}'",
                runs.display(),
                seen.display()
            )),
            TIMEOUT,
            &stop_rx,
            Some(&runs),
        );
        assert_eq!(read, Some(Ok(ReeveListing::default())));
        let seen_names = std::fs::read_to_string(&seen).expect("listing of runs");
        let _ = std::fs::remove_file(&seen);
        let name = seen_names.trim();
        assert!(
            name.starts_with(&format!("{}-", std::process::id())) && name.ends_with(".json"),
            "one record, named for this client: {seen_names:?}"
        );
        assert_eq!(records(&runs), Vec::<String>::new(), "gone after the run");
        let _ = std::fs::remove_dir_all(&runs);
    }

    /// Writes a run record as a client would, `client` and `group` as given.
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn record(runs: &Path, name: &str, client: ProcessIdentity, group: ProcessIdentity) {
        let run = ListingRun {
            client,
            group,
            command: vec!["sleep".into(), "1000".into()],
        };
        std::fs::write(runs.join(name), serde_json::to_vec(&run).unwrap()).unwrap();
    }

    /// A hung listing command as a killed client leaves it: leading a group of its own.
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn hung_run() -> (Child, ProcessIdentity) {
        use std::os::unix::process::CommandExt;
        let child = Command::new("sleep")
            .arg("1000")
            .process_group(0)
            .spawn()
            .expect("sleep");
        let group = ProcessIdentity::of(child.id() as i32).expect("identity");
        (child, group)
    }

    /// This process, as a client that is no longer running would be recorded: same pid,
    /// another start time.
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn gone_client() -> ProcessIdentity {
        let mut client = ProcessIdentity::of(std::process::id() as i32).expect("identity");
        client.start += 1;
        assert!(!client.alive());
        client
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn the_next_reader_kills_a_run_a_killed_client_left_behind() {
        use std::os::unix::process::ExitStatusExt;
        let runs = runs();
        let (mut child, group) = hung_run();
        record(&runs, "1-2.json", gone_client(), group);
        kill_leftover_runs(&runs);
        // The hung command is this test's child: waiting on it is deterministic.
        assert_eq!(child.wait().expect("wait").signal(), Some(libc::SIGKILL));
        assert_eq!(records(&runs), Vec::<String>::new());
        let _ = std::fs::remove_dir_all(&runs);
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn a_leftover_record_kills_only_the_very_command_it_started() {
        let runs = runs();
        let (mut child, group) = hung_run();
        // The pid now names a later process: the start time differs.
        let mut later = group;
        later.start += 1;
        record(&runs, "1-reused.json", gone_client(), later);
        // A client still running owns its run.
        let this_client = ProcessIdentity::of(std::process::id() as i32).expect("identity");
        record(&runs, "2-live.json", this_client, group);
        std::fs::write(runs.join("3-unreadable.json"), "{").unwrap();
        kill_leftover_runs(&runs);
        assert!(child.try_wait().expect("try_wait").is_none(), "not killed");
        assert_eq!(records(&runs), ["2-live.json", "3-unreadable.json"]);
        let _ = child.kill();
        let _ = child.wait();
        let _ = std::fs::remove_dir_all(&runs);
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn a_record_left_half_written_goes_with_its_client() {
        use std::os::unix::process::ExitStatusExt;
        let runs = runs();
        // Killed between writing and renaming: a whole record, read as one.
        let (mut child, group) = hung_run();
        record(&runs, "1-2.partial", gone_client(), group);
        // Killed while writing: a client no process has (pids stay far below this), and this
        // very client, still running and still writing.
        std::fs::write(runs.join("999999999-3.partial"), "{\"cli").unwrap();
        let live = format!("{}-4.partial", std::process::id());
        std::fs::write(runs.join(&live), "{\"cli").unwrap();
        kill_leftover_runs(&runs);
        assert_eq!(child.wait().expect("wait").signal(), Some(libc::SIGKILL));
        assert_eq!(records(&runs), [live]);
        let _ = std::fs::remove_dir_all(&runs);
    }

    #[test]
    fn a_listing_past_the_size_cap_is_refused() {
        let read = read_listing(&sh("head -c 9000000 /dev/zero"), TIMEOUT);
        assert_eq!(read, Err("listing larger than 8 MiB".into()));
    }

    #[test]
    fn the_spec_follows_the_config() {
        let mut config = crate::config::ReeveConfig::default();
        assert_eq!(ReeveListingSpec::from_config(&config), None);
        config.listing_command = vec!["true".into()];
        config.listing_interval_ms = 10;
        let spec = ReeveListingSpec::from_config(&config).unwrap();
        assert_eq!(spec.interval, Duration::from_millis(100), "at least 100 ms");
        assert_eq!(spec.timeout, Duration::from_secs(5), "5 s by default");
    }

    #[cfg(unix)]
    #[test]
    fn dropping_the_reader_kills_and_reaps_a_running_command() {
        let pid_file = pid_file();
        let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(8);
        let mut watch = None;
        let spec = ReeveListingSpec {
            command: sh(&format!(
                "echo $$ > '{}'; exec sleep 1000",
                pid_file.display()
            )),
            interval: Duration::from_millis(100),
            timeout: Duration::from_secs(600),
        };
        assert!(follow(&mut watch, Some(&spec), &event_tx));
        assert!(
            !follow(&mut watch, Some(&spec), &event_tx),
            "same spec, same reader"
        );
        let pid = pid_in(&pid_file);
        assert!(in_process_table(pid));
        // The client detaching drops the reader with the rest of its loop.
        drop(watch);
        assert!(
            !in_process_table(pid),
            "no listing command left after detach"
        );
        assert!(event_rx.try_recv().is_err(), "a killed run sends nothing");
    }

    #[test]
    fn a_new_command_restarts_the_reader_and_its_reads_carry_a_new_generation() {
        let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(8);
        let mut watch = None;
        let spec = |text: &str| ReeveListingSpec {
            command: vec!["printf".into(), "%s".into(), text.into()],
            interval: Duration::from_millis(100),
            timeout: TIMEOUT,
        };
        let recv = |event_rx: &mut tokio::sync::mpsc::Receiver<ClientLoopEvent>| {
            let deadline = Instant::now() + TIMEOUT;
            loop {
                if let Ok(ClientLoopEvent::ReeveListing(generation, read)) = event_rx.try_recv() {
                    return (generation, read);
                }
                assert!(Instant::now() < deadline, "no read");
                std::thread::sleep(POLL);
            }
        };
        follow(&mut watch, Some(&spec(DRAFT)), &event_tx);
        let (first, read) = recv(&mut event_rx);
        assert_eq!(read.unwrap().errands.len(), 1);
        assert!(follow(&mut watch, Some(&spec("{}")), &event_tx));
        let (second, read) = recv(&mut event_rx);
        assert_ne!(first, second);
        assert_eq!(watch.as_ref().unwrap().generation(), second);
        assert_eq!(read.unwrap().errands.len(), 0);
        assert!(
            follow(&mut watch, None, &event_tx),
            "a removed command stops the reader"
        );
        assert!(watch.is_none());
    }
}
