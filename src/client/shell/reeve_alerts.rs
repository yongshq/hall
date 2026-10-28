//! hall: alerts on the reeve tree (plan step 4, decisions 19, 22, 35 and 62 to 65).
//!
//! Herdr's own notifications ring for a reeve's pane at a dialog (request) or ending its turn
//! (done), and for a hand's pane at a dialog (request); herdr's done for a hand's pane is
//! dropped, since a hand's turn end is not one of decision 19's triggers. Hall adds a request
//! sound when a read of the listing shows a hand newly blocked, asking or stuck. Every alert
//! also raises a highlight on the reeve's row, and on the hand's row for a hand, apart from the
//! sound, so it shows with the sound muted too. A highlight clears when the liege shows that
//! reeve's or hand's pane, or when what raised it no longer holds; showing a reeve also clears
//! its done and failed hands' highlights, which no row of theirs could clear. A hand's
//! `waiting`, `idle` or `wedged` that rises again on a spell already rung (the reeve's watch
//! exited and came back), or is up at a client's first read, raises its highlight again
//! without a sound, unless the liege has shown the hand since that spell rang, or is looking
//! at its pane at that first read.

use super::*;
use crate::client::reeve_listing::{FlagsSince, HandState, ListedErrand, ReeveListing};

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
    /// When each flag's present spell began, as listed.
    since: FlagsSince,
    /// The spell of each flag this client last rang, or seeded on its first read: a flag that
    /// rises again with the same `since` (the reeve's watch exited and came back, which drops
    /// the flag between) is the same spell and rings nothing.
    rung: FlagsSince,
    /// The spell of each flag rung when the liege last showed the hand, or rung by a dialog the
    /// liege had shown: a flag rising again on that spell raises no highlight either.
    shown: FlagsSince,
}

/// What this client did with herdr's request for the dialog a hand's pane stands at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HandDialog {
    /// None, or none that still counts.
    None,
    /// Holds it for the toast delay or the snapshot: it may yet ring, or be dropped as stale.
    Held,
    /// Rang it.
    Rang,
    /// Rang it, and the liege has shown the pane since.
    Shown,
}

/// What a read raised for a hand: the rises that ring, by name, and the flags back up on a
/// spell already rung, which raise the highlight without a sound.
#[derive(Debug, Default)]
struct Raised {
    rang: Vec<&'static str>,
    again: Vec<&'static str>,
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
            since: errand.since,
            rung: FlagsSince::default(),
            shown: FlagsSince::default(),
        }
    }

    /// On the first read, which rings nothing: each flag up then counts as rung for its spell.
    /// Whether one was up, which raises the hand's highlight without a sound.
    fn seed(&mut self) -> bool {
        let mut up_any = false;
        for (up, since, rung) in [
            (self.waiting, self.since.waiting, &mut self.rung.waiting),
            (self.idle, self.since.idle, &mut self.rung.idle),
            (self.wedged, self.since.wedged, &mut self.rung.wedged),
        ] {
            if up {
                *rung = since;
                up_any = true;
            }
        }
        up_any
    }

    /// What rose from `before` to `self`, keeping `before`'s rung and shown spells and noting
    /// each spell rung now. A flag rising with the `since` it last rang with rings nothing, and
    /// raises the highlight again unless the liege has shown the hand since; one with no
    /// `since` rises on every rise. A rise of `waiting` rings nothing either when this client
    /// rang herdr's request for the dialog the hand's pane stands at (decisions 62, 64, 65),
    /// and its spell counts as rung, and as shown when the liege has shown that dialog. While
    /// this client only holds that request, the rise waits: it is taken on a later read, once
    /// the request has rung or been dropped.
    fn rises(&mut self, before: &Self, dialog: HandDialog) -> Raised {
        self.rung = before.rung;
        self.shown = before.shown;
        let mut raised = Raised::default();
        for (now, was, name) in [
            (self.blocked, before.blocked, "blocked"),
            (self.needs_decision, before.needs_decision, "needs-decision"),
        ] {
            if now && !was {
                raised.rang.push(name);
            }
        }
        let mut waiting_held = false;
        for (now, was, since, rung, shown, dialog, name) in [
            (
                self.waiting,
                before.waiting,
                self.since.waiting,
                &mut self.rung.waiting,
                &mut self.shown.waiting,
                dialog,
                "waiting",
            ),
            (
                self.idle,
                before.idle,
                self.since.idle,
                &mut self.rung.idle,
                &mut self.shown.idle,
                HandDialog::None,
                "idle",
            ),
            (
                self.wedged,
                before.wedged,
                self.since.wedged,
                &mut self.rung.wedged,
                &mut self.shown.wedged,
                HandDialog::None,
                "wedged",
            ),
        ] {
            if !now || was {
                continue;
            }
            if since.is_some() && since == *rung {
                if since != *shown {
                    raised.again.push(name);
                }
                continue;
            }
            if dialog == HandDialog::Held {
                waiting_held = true;
                continue;
            }
            if since.is_some() {
                *rung = since;
                if dialog == HandDialog::Shown {
                    *shown = since;
                }
            }
            if dialog == HandDialog::None {
                raised.rang.push(name);
            }
        }
        // Not taken yet: kept as down, so the next read sees the rise again.
        if waiting_held {
            self.waiting = false;
        }
        if self.question.is_some() && self.question != before.question {
            raised.rang.push("question");
        }
        raised
    }

    /// What is kept of a hand that a read no longer lists: its question, so a hand coming
    /// back with the same question does not ring it again (decision 64). Its rung spells go.
    fn kept(&self) -> Option<Self> {
        self.question.as_ref().map(|question| Self {
            question: Some(question.clone()),
            ..Self::default()
        })
    }
}

/// The errands in the hall session `session` that the tree shows a row for: those whose owner
/// is a reeve listed there. One whose owner names no such reeve (a null `owner_reeve`, say)
/// nests under no reeve (docs/listing.md), so no row could show its highlight: it raises no
/// alert, and its pane keeps herdr's own notifications as any pane the listing does not name.
fn alert_errands<'a>(
    listing: &'a ReeveListing,
    session: &'a str,
) -> impl Iterator<Item = &'a ListedErrand> {
    listing.errands.iter().filter(move |errand| {
        errand.session.in_session(session)
            && listing
                .reeves
                .iter()
                .any(|reeve| reeve.name == errand.owner_reeve && reeve.session.in_session(session))
    })
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
    /// Panes in `dialogs` whose dialog has had its highlight, raised or shown: a dialog rung
    /// before the listing named its pane (decision 66's window B) raises one on the first read
    /// that names it, and only then.
    dialogs_highlighted: HashSet<String>,
    /// Panes in `dialogs` whose rung dialog the liege has shown: a `waiting` that dialog
    /// absorbs counts as shown for its spell too.
    dialogs_shown: HashSet<String>,
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
        alert_errands(listing, session)
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
        self.reeve_alerts.dialogs_shown.remove(pane_id);
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

    /// Takes a good read for the alerts: the first seeds them and raises no sound, and raises
    /// the highlight of each hand with a flag up, but the one whose pane is focused, which
    /// counts as shown; after that, each hand whose listing entry
    /// rose (decision 62's edge rules) raises its highlight, and the read rings one request
    /// sound, while a flag back up on a spell already rung raises it without a sound. Any read,
    /// the first too, gives a dialog this client already rang its highlight once it names the
    /// pane. A hand alert clears when the hand leaves the listing or no longer waits on the
    /// liege. Failed reads never come here.
    pub(super) fn read_listing_alerts(&mut self) -> Vec<ClientShellNotificationEffect> {
        let Some((listing, session)) = self.alert_listing() else {
            return Vec::new();
        };
        // A dialog this client rang, while its pane reads blocked; or one whose request it
        // holds for the toast delay or the snapshot and rings when that is done (decision 65),
        // unless herdr's rule drops it then.
        let now = std::time::Instant::now();
        let hand_dialog = |pane_id: Option<&String>| {
            let Some(pane_id) = pane_id else {
                return HandDialog::None;
            };
            let rang = self.reeve_alerts.dialogs.get(pane_id).is_some_and(|until| {
                self.pane_reads_blocked(pane_id) || until.is_some_and(|until| now < until)
            });
            let held = self.pending_notifications.iter().any(|pending| {
                pending.endpoint_id == self.active_endpoint_id
                    && pending.event.kind == SemanticNotificationKind::NeedsAttention
                    && pending.event.pane_id.as_ref() == Some(pane_id)
            });
            if rang && self.reeve_alerts.dialogs_shown.contains(pane_id) {
                HandDialog::Shown
            } else if rang {
                HandDialog::Rang
            } else if held {
                HandDialog::Held
            } else {
                HandDialog::None
            }
        };
        let errands = alert_errands(listing, session).collect::<Vec<_>>();
        let mut facts = errands
            .iter()
            .map(|errand| (errand.id.clone(), HandFacts::of(errand)))
            .collect::<HashMap<_, _>>();
        let mut risen = Vec::new();
        if let Some(seen) = self.reeve_alerts.seen.as_ref() {
            for errand in &errands {
                let before = seen.get(&errand.id).cloned().unwrap_or_default();
                let dialog = hand_dialog(errand.session.pane_id.as_ref());
                let raised = facts
                    .get_mut(&errand.id)
                    .map_or_else(Raised::default, |facts| facts.rises(&before, dialog));
                if !raised.rang.is_empty() || !raised.again.is_empty() {
                    risen.push((errand.id.clone(), raised));
                }
            }
            for (id, before) in seen {
                if !facts.contains_key(id) {
                    if let Some(kept) = before.kept() {
                        facts.insert(id.clone(), kept);
                    }
                }
            }
        } else {
            // The hand whose pane the liege is looking at counts as shown, not highlighted.
            let focused = self
                .snapshot
                .as_deref()
                .and_then(|snapshot| snapshot.focused_pane_id.as_ref());
            for errand in &errands {
                let Some(facts) = facts.get_mut(&errand.id) else {
                    continue;
                };
                if !facts.seed() {
                    continue;
                }
                if focused.is_some() && errand.session.pane_id.as_ref() == focused {
                    facts.shown = facts.rung;
                } else {
                    risen.push((errand.id.clone(), Raised::default()));
                }
            }
        }
        let waiting = errands
            .iter()
            .filter(|errand| errand.waits_on_liege())
            .map(|errand| errand.id.clone())
            .collect::<HashSet<_>>();
        // A reeve's or a hand's dialog this client rang while the listing did not name its
        // pane (a hand's first seconds, a reeve's while no good read had named it) rang with no
        // highlight; it gets one now, without a sound, while it reads blocked (decision 63).
        let unhighlighted = listing
            .reeves
            .iter()
            .filter(|reeve| reeve.session.in_session(session))
            .map(|reeve| &reeve.session)
            .chain(errands.iter().map(|errand| &errand.session))
            .filter_map(|located| located.pane_id.clone())
            .filter(|pane_id| {
                self.reeve_alerts.dialogs.contains_key(pane_id)
                    && !self.reeve_alerts.dialogs_highlighted.contains(pane_id)
                    && self.pane_reads_blocked(pane_id)
            })
            .collect::<Vec<_>>();
        let alerts = &mut self.reeve_alerts;
        alerts.seen = Some(facts);
        for pane_id in unhighlighted {
            if !alerts.panes.contains_key(&pane_id) {
                tracing::info!(
                    event = "reeve.alert",
                    source = "dialog",
                    pane_id = pane_id.as_str(),
                    "hall alert raised"
                );
                alerts.panes.insert(pane_id.clone(), PaneAlert::Dialog);
            }
            alerts.dialogs_highlighted.insert(pane_id);
        }
        alerts.hands.retain(|id| waiting.contains(id));
        for (id, raised) in &risen {
            tracing::info!(
                event = "reeve.alert",
                source = "listing",
                hand = id.as_str(),
                rises = ?raised.rang,
                again = ?raised.again,
                "hall alert raised"
            );
            alerts.hands.insert(id.clone());
        }
        if risen.iter().all(|(_, raised)| raised.rang.is_empty()) {
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
        let dialogs = &self.reeve_alerts.dialogs;
        self.reeve_alerts
            .dialogs_highlighted
            .retain(|pane_id| dialogs.contains_key(pane_id));
        self.reeve_alerts
            .dialogs_shown
            .retain(|pane_id| dialogs.contains_key(pane_id));
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

    /// The liege shows a pane: the alerts of the reeve or hand it belongs to clear, and a
    /// hand's flags rising again on the spells rung so far stay clear, as does a `waiting` the
    /// dialog this client rang on that pane absorbs later. Showing a reeve shows its done and
    /// failed hands with it, since they have no row of their own to show (decision 76).
    pub(super) fn show_reeve_alerts(&mut self, pane_id: &str) {
        let role = self.pane_alert_role(pane_id);
        let finished = match role {
            Some(AlertRole::Reeve) => self.finished_hands(pane_id),
            _ => Vec::new(),
        };
        let alerts = &mut self.reeve_alerts;
        alerts.show_pane(pane_id);
        if let Some(AlertRole::Hand(id)) = role {
            alerts.show_hand(&id);
        }
        for (id, hand_pane) in finished {
            if let Some(hand_pane) = hand_pane {
                alerts.show_pane(&hand_pane);
            }
            alerts.show_hand(&id);
        }
    }

    /// The listed hands reported done or failed of the reeve whose pane is `pane_id`, with
    /// their panes as listed.
    fn finished_hands(&self, pane_id: &str) -> Vec<(String, Option<String>)> {
        let Some((listing, session)) = self.alert_listing() else {
            return Vec::new();
        };
        let Some(reeve) = listing.reeves.iter().find(|reeve| {
            reeve.session.in_session(session) && reeve.session.pane_id.as_deref() == Some(pane_id)
        }) else {
            return Vec::new();
        };
        alert_errands(listing, session)
            .filter(|errand| {
                errand.owner_reeve == reeve.name
                    && matches!(errand.hand_state, HandState::Done | HandState::Failed)
            })
            .map(|errand| (errand.id.clone(), errand.session.pane_id.clone()))
            .collect()
    }

    /// Forgets every alert and the seed, when the listing command changes or goes.
    pub(super) fn reset_reeve_alerts(&mut self) {
        self.reeve_alerts = ReeveAlerts::default();
    }
}

impl ReeveAlerts {
    /// The liege shows the pane: its highlight clears, and a dialog rung on it counts as shown.
    fn show_pane(&mut self, pane_id: &str) {
        self.panes.remove(pane_id);
        if self.dialogs.contains_key(pane_id) {
            self.dialogs_highlighted.insert(pane_id.to_owned());
            self.dialogs_shown.insert(pane_id.to_owned());
        }
    }

    /// The liege shows the hand `id`: its listing alert clears, and its spells rung so far
    /// count as shown.
    fn show_hand(&mut self, id: &str) {
        self.hands.remove(id);
        if let Some(facts) = self.seen.as_mut().and_then(|seen| seen.get_mut(id)) {
            facts.shown = facts.rung;
        }
    }

    /// Whether the hand `id`, whose pane is `pane_id`, has a highlight raised.
    pub(super) fn hand(&self, id: &str, pane_id: Option<&str>) -> bool {
        self.hands.contains(id) || pane_id.is_some_and(|pane_id| self.panes.contains_key(pane_id))
    }

    /// Whether a reeve's own pane has a highlight raised.
    pub(super) fn pane(&self, pane_id: Option<&str>) -> bool {
        pane_id.is_some_and(|pane_id| self.panes.contains_key(pane_id))
    }
}
