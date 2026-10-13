//! hall: alerts on the reeve tree (plan step 4, decisions 19, 22, 35, 62 and 63).
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

    /// What rose from `before` to `self`, by name. A rise of `waiting` is left out when hall
    /// already reads the hand's pane at a dialog: herdr's request rang for it (decision 62).
    fn rises(&self, before: &Self, pane_blocked: bool) -> Vec<&'static str> {
        let mut rises = Vec::new();
        for (now, was, name) in [
            (self.blocked, before.blocked, "blocked"),
            (self.needs_decision, before.needs_decision, "needs-decision"),
            (self.waiting && !pane_blocked, before.waiting, "waiting"),
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
}

#[derive(Debug, Default)]
pub(in crate::client) struct ReeveAlerts {
    /// The hands as the last good read listed them; None until the first good read seeds it.
    seen: Option<HashMap<String, HandFacts>>,
    /// Reeve and hand panes alerting from herdr's notifications, by pane id.
    panes: HashMap<String, PaneAlert>,
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
            located.herdr_session == session && located.pane_id.as_deref() == Some(pane_id)
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

    /// Raises the highlight for a reeve's or a hand's pane from a notification that rings.
    pub(super) fn raise_pane_alert(&mut self, pane_id: &str, role: &AlertRole, alert: PaneAlert) {
        tracing::info!(
            event = "reeve.alert",
            source = "herdr",
            pane_id,
            role = ?role,
            alert = ?alert,
            "hall alert raised"
        );
        self.reeve_alerts.panes.insert(pane_id.to_owned(), alert);
    }

    /// Takes a good read for the alerts: the first seeds them and raises nothing; after that,
    /// each hand whose listing entry rose (decision 62's edge rules) raises its highlight, and
    /// the read rings one request sound. A hand alert clears when the hand leaves the listing
    /// or no longer waits on the liege. Failed reads never come here.
    pub(super) fn read_listing_alerts(&mut self) -> Vec<ClientShellNotificationEffect> {
        let Some((listing, session)) = self.alert_listing() else {
            return Vec::new();
        };
        let pane_blocked = |pane_id: Option<&String>| {
            pane_id.is_some_and(|pane_id| {
                self.snapshot.as_deref().is_some_and(|snapshot| {
                    snapshot.agents.iter().any(|agent| {
                        &agent.pane_id == pane_id
                            && agent.agent_status == crate::api::schema::AgentStatus::Blocked
                    })
                })
            })
        };
        let errands = listing
            .errands
            .iter()
            .filter(|errand| errand.session.herdr_session == session)
            .collect::<Vec<_>>();
        let facts = errands
            .iter()
            .map(|errand| (errand.id.clone(), HandFacts::of(errand)))
            .collect::<HashMap<_, _>>();
        let mut risen = Vec::new();
        if let Some(seen) = self.reeve_alerts.seen.as_ref() {
            for errand in &errands {
                let before = seen.get(&errand.id).cloned().unwrap_or_default();
                let rises =
                    facts[&errand.id].rises(&before, pane_blocked(errand.session.pane_id.as_ref()));
                if !rises.is_empty() {
                    risen.push((errand.id.clone(), rises));
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
        self.reeve_alerts.panes.retain(|pane_id, alert| {
            use crate::api::schema::AgentStatus;
            listed.contains(pane_id)
                && match alert {
                    PaneAlert::Reply => {
                        matches!(status(pane_id), Some(AgentStatus::Idle | AgentStatus::Done))
                    }
                    PaneAlert::Dialog => status(pane_id) == Some(AgentStatus::Blocked),
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
