//! hall: alerts on the reeve tree (plan step 4, decisions 19, 22, 35 and 62 to 65).
//!
//! Herdr's own notifications ring for a reeve's pane at a dialog (request) or ending its turn
//! (done), and for a hand's pane at a dialog (request); herdr's done for a hand's pane is
//! dropped, since a hand's turn end is not one of decision 19's triggers. Hall adds a request
//! sound when a read of the listing shows a hand newly blocked, asking or stuck. Every alert
//! also raises a highlight on the reeve's row, and on the hand's row for a hand, apart from the
//! sound, so it shows with the sound muted too. A highlight clears when the liege shows that
//! reeve's or hand's pane, or when what raised it no longer holds.

use super::*;
use crate::client::reeve_listing::{HandState, ListedErrand, ReeveListing};

/// What a pane the listing names is, to the alerts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum AlertRole {
    Reeve,
    Hand(String),
}

/// An alert raised for a reeve's or a hand's pane from herdr's notifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PaneAlert {
    /// The reeve's turn ended (herdr's Finished): holds while its pane reads idle or done.
    Reply,
    /// The pane stands at a dialog (herdr's NeedsAttention): holds while it reads blocked.
    Dialog,
}

/// What a hand's listing entry says that can raise an alert (decision 19).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct HandFacts {
    blocked: bool,
    needs_decision: bool,
    waiting: bool,
    idle: bool,
    wedged: bool,
    question: Option<String>,
}

impl HandFacts {
    fn of(errand: &ListedErrand) -> Self {
        Self {
            blocked: errand.hand_state == HandState::Blocked,
            needs_decision: errand.hand_state == HandState::NeedsDecision,
            waiting: errand.waiting,
            idle: errand.idle,
            wedged: errand.wedged,
            question: errand
                .question
                .clone()
                .filter(|question| !question.is_empty()),
        }
    }

    /// What rose from `before` to `self`, by name. A rise of `waiting` is left out when this
    /// client rang herdr's request for the dialog the hand's pane stands at, or holds it to
    /// ring (decisions 62, 64, 65).
    fn rises(&self, before: &Self, dialog_rung: bool) -> Vec<&'static str> {
        let mut rises = Vec::new();
        for (now, was, name) in [
            (self.blocked, before.blocked, "blocked"),
            (self.needs_decision, before.needs_decision, "needs-decision"),
            (self.waiting && !dialog_rung, before.waiting, "waiting"),
            (self.idle, before.idle, "idle"),
            (self.wedged, before.wedged, "wedged"),
        ] {
            if now && !was {
                rises.push(name);
            }
        }
        if self.question.is_some() && self.question != before.question {
            rises.push("question");
        }
        rises
    }

    /// What is kept of a hand that a read no longer lists: its question, so a hand coming
    /// back with the same question does not ring it again (decision 64).
    fn kept(&self) -> Option<Self> {
        self.question.as_ref().map(|question| Self {
            question: Some(question.clone()),
            ..Self::default()
        })
    }
}

#[derive(Debug, Default)]
pub(in crate::client) struct ReeveAlerts {
    /// The hands as the last good read listed them, and the question of each hand that has
    /// left it; None until the first good read seeds it.
    seen: Option<HashMap<String, HandFacts>>,
    /// Reeve and hand panes alerting from herdr's notifications, by pane id.
    panes: HashMap<String, PaneAlert>,
    /// Panes whose dialog this client rang for, listed or not, while they read blocked, apart
    /// from whether the liege has since shown them. One rung before this client's snapshot
    /// reads it blocked (herdr's rule rings an unlisted pane's request at once with a toast
    /// delay of 0) is kept until herdr's grace for that snapshot ends, the deadline here, and
    /// from the first read of it blocked on only while it reads blocked.
    dialogs: HashMap<String, Option<std::time::Instant>>,
    /// Hands alerting from the listing, by errand id.
    hands: HashSet<String>,
    /// The focused pane as last composed, to see the liege move to an alerting pane; None
    /// before the first frame.
    focused_pane: Option<Option<String>>,
}

impl ClientShellState {
    /// The listing that the alerts go by: the last good one, when the tree is on for one
    /// machine. Its reeves and errands in this hall session.
    fn alert_listing(&self) -> Option<(&ReeveListing, &str)> {
        let session = self.config.reeve_tree_session.as_deref()?;
        if self.endpoints.len() > 1 {
            return None;
        }
        Some((self.reeve_listing.as_ref()?, session))
    }

    /// What the notification's pane is to the alerts: a listed reeve's pane or a listed hand's
    /// pane in this hall session. None for any other pane, or when the tree is off.
    pub(super) fn reeve_alert_role(
        &self,
        endpoint_id: &ClientEndpointId,
        event: &SemanticNotification,
    ) -> Option<AlertRole> {
        if endpoint_id != &self.active_endpoint_id {
            return None;
        }
        self.pane_alert_role(event.pane_id.as_deref()?)
    }

    fn pane_alert_role(&self, pane_id: &str) -> Option<AlertRole> {
        let (listing, session) = self.alert_listing()?;
        let names = |located: &crate::client::reeve_listing::ListedSession| {
            located.in_session(session) && located.pane_id.as_deref() == Some(pane_id)
        };
        if listing.reeves.iter().any(|reeve| names(&reeve.session)) {
            return Some(AlertRole::Reeve);
        }
        listing
            .errands
            .iter()
            .find(|errand| names(&errand.session))
            .map(|errand| AlertRole::Hand(errand.id.clone()))
    }

    /// Raises the highlight for a reeve's or a hand's pane from a notification that rings. A
    /// reply on the pane the liege is looking at rings with no highlight left (decision 64).
    pub(super) fn raise_pane_alert(&mut self, pane_id: &str, role: &AlertRole, alert: PaneAlert) {
        let in_view = self.pane_in_view(pane_id);
        tracing::info!(
            event = "reeve.alert",
            source = "herdr",
            pane_id,
            role = ?role,
            alert = ?alert,
            in_view,
            "hall alert raised"
        );
        if alert == PaneAlert::Reply && in_view {
            return;
        }
        self.reeve_alerts.panes.insert(pane_id.to_owned(), alert);
    }

    /// Herdr's request for a dialog on `pane_id` rang in this client: remembered while the
    /// pane reads blocked, whether the listing names the pane yet or not, so a listing
    /// `waiting` on that dialog does not ring it again (decision 68). A hand's first seconds go
    /// unlisted, since reeve records its pane only once its launch is proven, and its request
    /// then rings on herdr's plain rule (decision 66's window B).
    pub(super) fn note_rung_dialog(&mut self, pane_id: &str, now: std::time::Instant) {
        let grace = super::notification_policy::COMPLETION_EVIDENCE_GRACE;
        let until =
            (!self.pane_reads_blocked(pane_id)).then(|| now.checked_add(grace).unwrap_or(now));
        self.reeve_alerts.dialogs.insert(pane_id.to_owned(), until);
    }

    /// Whether this client's snapshot reads the pane blocked.
    fn pane_reads_blocked(&self, pane_id: &str) -> bool {
        self.snapshot.as_deref().is_some_and(|snapshot| {
            snapshot.agents.iter().any(|agent| {
                agent.pane_id == pane_id
                    && agent.agent_status == crate::api::schema::AgentStatus::Blocked
            })
        })
    }

    /// The hall window has focus again: a reply on the pane in view is seen, as herdr counts it
    /// seen, and its highlight clears; a dialog's stands (decision 65). Whether one cleared.
    pub(super) fn see_reply_in_view(&mut self) -> bool {
        let Some(pane_id) = self
            .snapshot
            .as_deref()
            .and_then(|snapshot| snapshot.focused_pane_id.clone())
        else {
            return false;
        };
        if !self.pane_in_view(&pane_id)
            || self.reeve_alerts.panes.get(&pane_id) != Some(&PaneAlert::Reply)
        {
            return false;
        }
        self.reeve_alerts.panes.remove(&pane_id);
        true
    }

    /// Whether the liege is looking at the pane: the focused pane of this client's active
    /// endpoint, in a window that has focus or cannot say (herdr's rule for its own
    /// focused-tab suppression).
    fn pane_in_view(&self, pane_id: &str) -> bool {
        self.outer_focused != Some(false)
            && self
                .snapshot
                .as_deref()
                .and_then(|snapshot| snapshot.focused_pane_id.as_deref())
                == Some(pane_id)
    }

    /// Takes a good read for the alerts: the first seeds them and raises nothing; after that,
    /// each hand whose listing entry rose (decision 62's edge rules) raises its highlight, and
    /// the read rings one request sound. A hand alert clears when the hand leaves the listing
    /// or no longer waits on the liege. Failed reads never come here.
    pub(super) fn read_listing_alerts(&mut self) -> Vec<ClientShellNotificationEffect> {
        let Some((listing, session)) = self.alert_listing() else {
            return Vec::new();
        };
        // A dialog this client rang, while its pane reads blocked, or one whose request it
        // holds for the toast delay or the snapshot and rings when that is done (decision 65).
        let now = std::time::Instant::now();
        let dialog_rung = |pane_id: Option<&String>| {
            pane_id.is_some_and(|pane_id| {
                let rung = self.reeve_alerts.dialogs.get(pane_id).is_some_and(|until| {
                    self.pane_reads_blocked(pane_id) || until.is_some_and(|until| now < until)
                });
                rung || self.pending_notifications.iter().any(|pending| {
                    pending.endpoint_id == self.active_endpoint_id
                        && pending.event.kind == SemanticNotificationKind::NeedsAttention
                        && pending.event.pane_id.as_ref() == Some(pane_id)
                })
            })
        };
        let errands = listing
            .errands
            .iter()
            .filter(|errand| errand.session.in_session(session))
            .collect::<Vec<_>>();
        let mut facts = errands
            .iter()
            .map(|errand| (errand.id.clone(), HandFacts::of(errand)))
            .collect::<HashMap<_, _>>();
        let mut risen = Vec::new();
        if let Some(seen) = self.reeve_alerts.seen.as_ref() {
            for errand in &errands {
                let before = seen.get(&errand.id).cloned().unwrap_or_default();
                let rises =
                    facts[&errand.id].rises(&before, dialog_rung(errand.session.pane_id.as_ref()));
                if !rises.is_empty() {
                    risen.push((errand.id.clone(), rises));
                }
            }
            for (id, before) in seen {
                if !facts.contains_key(id) {
                    if let Some(kept) = before.kept() {
                        facts.insert(id.clone(), kept);
                    }
                }
            }
        }
        let waiting = errands
            .iter()
            .filter(|errand| errand.waits_on_liege())
            .map(|errand| errand.id.clone())
            .collect::<HashSet<_>>();
        let alerts = &mut self.reeve_alerts;
        alerts.seen = Some(facts);
        alerts.hands.retain(|id| waiting.contains(id));
        for (id, rises) in &risen {
            tracing::info!(
                event = "reeve.alert",
                source = "listing",
                hand = id.as_str(),
                rises = ?rises,
                "hall alert raised"
            );
            alerts.hands.insert(id.clone());
        }
        if risen.is_empty() {
            Vec::new()
        } else {
            vec![ClientShellNotificationEffect::Sound {
                sound: crate::sound::Sound::Request,
                agent: None,
            }]
        }
    }

    /// Clears the highlights whose trigger no longer holds, and those of a pane the liege has
    /// just moved to. Run before each frame is built.
    pub(super) fn prune_reeve_alerts(&mut self) {
        let Some(snapshot) = self.snapshot.as_deref() else {
            return;
        };
        let focused = snapshot.focused_pane_id.clone();
        let status = |pane_id: &str| {
            snapshot
                .agents
                .iter()
                .find(|agent| agent.pane_id == pane_id)
                .map(|agent| agent.agent_status)
        };
        let listed = self
            .reeve_alerts
            .panes
            .keys()
            .filter(|pane_id| self.pane_alert_role(pane_id).is_some())
            .cloned()
            .collect::<HashSet<_>>();
        use crate::api::schema::AgentStatus;
        self.reeve_alerts.panes.retain(|pane_id, alert| {
            listed.contains(pane_id)
                && match alert {
                    PaneAlert::Reply => {
                        matches!(status(pane_id), Some(AgentStatus::Idle | AgentStatus::Done))
                    }
                    PaneAlert::Dialog => status(pane_id) == Some(AgentStatus::Blocked),
                }
        });
        let now = std::time::Instant::now();
        self.reeve_alerts.dialogs.retain(|pane_id, until| {
            if status(pane_id) == Some(AgentStatus::Blocked) {
                *until = None;
                true
            } else {
                until.is_some_and(|until| now < until)
            }
        });
        let moved = self
            .reeve_alerts
            .focused_pane
            .as_ref()
            .is_some_and(|last| last != &focused);
        self.reeve_alerts.focused_pane = Some(focused.clone());
        if let Some(pane_id) = focused.filter(|_| moved) {
            self.show_reeve_alerts(&pane_id);
        }
    }

    /// The liege shows a pane: the alerts of the reeve or hand it belongs to clear.
    pub(super) fn show_reeve_alerts(&mut self, pane_id: &str) {
        let role = self.pane_alert_role(pane_id);
        let alerts = &mut self.reeve_alerts;
        alerts.panes.remove(pane_id);
        if let Some(AlertRole::Hand(id)) = role {
            alerts.hands.remove(&id);
        }
    }

    /// Forgets every alert and the seed, when the listing command changes or goes.
    pub(super) fn reset_reeve_alerts(&mut self) {
        self.reeve_alerts = ReeveAlerts::default();
    }
}

impl ReeveAlerts {
    /// Whether the hand `id`, whose pane is `pane_id`, has a highlight raised.
    pub(super) fn hand(&self, id: &str, pane_id: Option<&str>) -> bool {
        self.hands.contains(id) || pane_id.is_some_and(|pane_id| self.panes.contains_key(pane_id))
    }

    /// Whether a reeve's own pane has a highlight raised.
    pub(super) fn pane(&self, pane_id: Option<&str>) -> bool {
        pane_id.is_some_and(|pane_id| self.panes.contains_key(pane_id))
    }
}
