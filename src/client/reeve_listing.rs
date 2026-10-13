//! hall: reeve's listing, read by the client for the reeve tree (plan step 3).
//!
//! The listing is the draft in docs/listing-draft.md: every reeve and every errand in flight,
//! with where each one sits. Hall never reads status files (ADR 0002); it runs the command the
//! config names (`[reeve] listing_command`, decision 49) and reads its JSON.

use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde::Deserialize;

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

/// Runs the listing command once and parses what it prints.
pub(super) fn read_listing(command: &[String]) -> ReeveListingRead {
    let Some((program, args)) = command.split_first() else {
        return Err("no listing command".into());
    };
    let output = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("{program}: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let reason = stderr
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map_or_else(|| output.status.to_string(), str::to_owned);
        return Err(reason);
    }
    serde_json::from_slice(&output.stdout).map_err(|error| format!("listing unreadable: {error}"))
}

/// Reads the listing every `interval` on a thread of its own and sends each change to the
/// client loop, as `catalog_reload::watch_profiles` does for saved machines.
pub(super) fn watch(
    command: Vec<String>,
    interval: Duration,
    event_tx: tokio::sync::mpsc::Sender<ClientLoopEvent>,
    should_quit: Arc<AtomicBool>,
) {
    std::thread::spawn(move || {
        let mut previous = None;
        while !should_quit.load(Ordering::Acquire) {
            let current = read_listing(&command);
            if previous.as_ref() != Some(&current) {
                previous = Some(current.clone());
                if event_tx
                    .blocking_send(ClientLoopEvent::ReeveListing(current))
                    .is_err()
                {
                    break;
                }
            }
            std::thread::sleep(interval);
        }
    });
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

    #[test]
    fn a_failing_command_reads_as_its_first_stderr_line() {
        let read = read_listing(&[
            "sh".into(),
            "-c".into(),
            "echo; echo 'refusing home' >&2; exit 1".into(),
        ]);
        assert_eq!(read, Err("refusing home".into()));
    }

    #[test]
    fn a_missing_program_and_bad_json_are_errors_not_panics() {
        assert!(read_listing(&["/nonexistent/hall-listing".into()]).is_err());
        let read = read_listing(&["sh".into(), "-c".into(), "echo not json".into()]);
        assert!(read.unwrap_err().starts_with("listing unreadable"));
        assert_eq!(read_listing(&[]), Err("no listing command".into()));
    }

    #[test]
    fn a_command_printing_the_draft_reads_it() {
        let read = read_listing(&["printf".into(), "%s".into(), DRAFT.into()]).unwrap();
        assert_eq!(read.errands.len(), 1);
    }
}
