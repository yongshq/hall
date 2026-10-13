//! hall: alerts on the reeve tree (plan step 4, decisions 62 and 63).

use super::reeve_tree::{
    compose, errand_mut, hall_dev_snapshot, listing, picked_method, reeve_state, tree_rows,
};
use super::*;
use crate::client::reeve_listing::{HandState, ReeveListing};

/// The test layout with an agent in each reeve's and hand's pane, every one reading `status`
/// but those named in `statuses`.
fn agents_snapshot(statuses: &[(&str, AgentStatus)]) -> ClientShellSnapshot {
    let mut snapshot = hall_dev_snapshot();
    snapshot.agents = snapshot
        .panes
        .iter()
        .filter(|pane| pane.pane_id != "p_p0")
        .enumerate()
        .map(|(index, pane)| ClientShellAgent {
            pane_id: pane.pane_id.clone(),
            workspace_id: pane.workspace_id.clone(),
            tab_id: pane.tab_id.clone(),
            name: None,
            display_agent: Some("claude".into()),
            agent: Some("claude".into()),
            title: None,
            terminal_title: None,
            terminal_title_stripped: None,
            agent_status: statuses
                .iter()
                .find(|(pane_id, _)| *pane_id == pane.pane_id)
                .map_or(AgentStatus::Working, |(_, status)| *status),
            state_change_seq: index as u64 + 1,
            state_labels: Vec::new(),
            tokens: Vec::new(),
            focused: pane.focused,
        })
        .collect();
    snapshot
}

fn alert_state(listing: ReeveListing, statuses: &[(&str, AgentStatus)]) -> ClientShellState {
    let mut state = reeve_state(listing);
    state.config.toast_delay_seconds = 0;
    state.set_snapshot(Box::new(agents_snapshot(statuses)));
    state
}

fn set_statuses(state: &mut ClientShellState, statuses: &[(&str, AgentStatus)]) {
    state.set_snapshot(Box::new(agents_snapshot(statuses)));
}

fn notify(
    state: &mut ClientShellState,
    kind: SemanticNotificationKind,
    pane: &str,
    tab: &str,
) -> Vec<ClientShellNotificationEffect> {
    let sound = match kind {
        SemanticNotificationKind::Finished => Some(SemanticNotificationSound::Done),
        _ => Some(SemanticNotificationSound::Request),
    };
    let workspace = tab.rsplit_once('_').map_or(tab, |(workspace, _)| workspace);
    state
        .receive_notification(
            &ClientEndpointId::Local,
            SemanticNotification {
                kind,
                title: "claude".into(),
                body: None,
                sound,
                agent: Some("claude".into()),
                workspace_id: Some(workspace.into()),
                tab_id: Some(tab.into()),
                pane_id: Some(pane.into()),
                position: None,
            },
            std::time::Instant::now(),
        )
        .0
}

fn sounds(effects: &[ClientShellNotificationEffect]) -> Vec<crate::sound::Sound> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            ClientShellNotificationEffect::Sound { sound, .. } => Some(*sound),
            _ => None,
        })
        .collect()
}

/// The tree's rows that carry the alert bar, by their label.
fn alerting(state: &mut ClientShellState) -> Vec<String> {
    tree_rows(&compose(state))
        .into_iter()
        .filter(|row| row.starts_with('▌'))
        .map(|row| {
            row.split_whitespace()
                .find(|word| word.chars().any(char::is_alphabetic))
                .unwrap_or_default()
                .to_owned()
        })
        .collect()
}

#[test]
fn a_reeve_reply_rings_done_and_highlights_its_row_until_its_next_turn() {
    let mut state = alert_state(listing(), &[("p_b0", AgentStatus::Done)]);
    let effects = notify(
        &mut state,
        SemanticNotificationKind::Finished,
        "p_b0",
        "ws_b_t0",
    );
    assert_eq!(sounds(&effects), [crate::sound::Sound::Done]);
    let frame = compose(&mut state);
    assert_eq!(tree_rows(&frame)[4], "▌● bertil");
    assert_eq!(alerting(&mut state), ["bertil"]);
    // The reeve starts its next turn: what raised the highlight no longer holds.
    set_statuses(&mut state, &[("p_b0", AgentStatus::Working)]);
    assert!(alerting(&mut state).is_empty());
}

#[test]
fn a_reeve_dialog_rings_request_and_clears_when_answered() {
    let mut state = alert_state(listing(), &[("p_b0", AgentStatus::Blocked)]);
    let effects = notify(
        &mut state,
        SemanticNotificationKind::NeedsAttention,
        "p_b0",
        "ws_b_t0",
    );
    assert_eq!(sounds(&effects), [crate::sound::Sound::Request]);
    assert_eq!(alerting(&mut state), ["bertil"]);
    set_statuses(&mut state, &[("p_b0", AgentStatus::Done)]);
    assert!(alerting(&mut state).is_empty());
}

#[test]
fn a_hand_at_a_dialog_rings_and_highlights_the_hand_and_its_reeve() {
    let mut state = alert_state(listing(), &[("p_b1", AgentStatus::Blocked)]);
    let effects = notify(
        &mut state,
        SemanticNotificationKind::NeedsAttention,
        "p_b1",
        "ws_b_t1",
    );
    assert_eq!(sounds(&effects), [crate::sound::Sound::Request]);
    assert_eq!(alerting(&mut state), ["bertil", "bertil-probe"]);
    assert_eq!(tree_rows(&compose(&mut state))[5], "▌ └ ● bertil-probe");
}

#[test]
fn a_hand_ending_its_turn_is_not_an_alert() {
    let mut state = alert_state(listing(), &[("p_b1", AgentStatus::Done)]);
    let effects = notify(
        &mut state,
        SemanticNotificationKind::Finished,
        "p_b1",
        "ws_b_t1",
    );
    assert!(effects.is_empty(), "herdr's done is dropped for a hand");
    assert!(alerting(&mut state).is_empty());
    assert!(
        state.visible_notification.is_none(),
        "and its toast with it"
    );
}

#[test]
fn a_pane_the_listing_does_not_name_keeps_herdrs_notifications_and_no_highlight() {
    let mut listing = listing();
    listing.errands.retain(|errand| errand.id != "bertil-probe");
    let mut state = alert_state(listing, &[("p_b1", AgentStatus::Done)]);
    let effects = notify(
        &mut state,
        SemanticNotificationKind::Finished,
        "p_b1",
        "ws_b_t1",
    );
    assert_eq!(sounds(&effects), [crate::sound::Sound::Done]);
    assert!(alerting(&mut state).is_empty());
}

#[test]
fn the_highlight_rises_without_a_sound_in_the_notification() {
    let mut state = alert_state(listing(), &[("p_b0", AgentStatus::Blocked)]);
    state.receive_notification(
        &ClientEndpointId::Local,
        SemanticNotification {
            kind: SemanticNotificationKind::NeedsAttention,
            title: "claude".into(),
            body: None,
            sound: None,
            agent: Some("claude".into()),
            workspace_id: Some("ws_b".into()),
            tab_id: Some("ws_b_t0".into()),
            pane_id: Some("p_b0".into()),
            position: None,
        },
        std::time::Instant::now(),
    );
    assert_eq!(alerting(&mut state), ["bertil"]);
}

#[test]
fn a_focused_reeve_ending_its_turn_alerts_with_the_setting_on() {
    // Ansel's tab is the focused active one, so herdr marks the pane seen: idle, not done.
    let mut state = alert_state(listing(), &[("p_a0", AgentStatus::Idle)]);
    assert!(state.config.reeve_alert_focused_reply, "on by default");
    let effects = notify(
        &mut state,
        SemanticNotificationKind::Finished,
        "p_a0",
        "ws_a_t0",
    );
    assert_eq!(sounds(&effects), [crate::sound::Sound::Done]);
    assert_eq!(alerting(&mut state), ["ansel"]);

    let mut state = alert_state(listing(), &[("p_a0", AgentStatus::Idle)]);
    state.config.reeve_alert_focused_reply = false;
    let effects = notify(
        &mut state,
        SemanticNotificationKind::Finished,
        "p_a0",
        "ws_a_t0",
    );
    assert!(effects.is_empty(), "herdr's suppression, as herdr has it");
    assert!(alerting(&mut state).is_empty());

    // Off, a reeve's done that still reads done in the focused tab rings nothing either.
    let mut state = alert_state(listing(), &[("p_a0", AgentStatus::Done)]);
    state.config.reeve_alert_focused_reply = false;
    let effects = notify(
        &mut state,
        SemanticNotificationKind::Finished,
        "p_a0",
        "ws_a_t0",
    );
    assert!(sounds(&effects).is_empty());
}

#[test]
fn the_setting_lifts_the_suppression_for_reeves_only() {
    let mut listing = listing();
    // A plain agent in the focused tab: not a reeve, so herdr's suppression stands.
    listing.reeves.retain(|reeve| reeve.name != "ansel");
    listing
        .errands
        .retain(|errand| errand.owner_reeve != "ansel");
    let mut state = alert_state(listing, &[("p_a0", AgentStatus::Idle)]);
    let effects = notify(
        &mut state,
        SemanticNotificationKind::Finished,
        "p_a0",
        "ws_a_t0",
    );
    assert!(effects.is_empty());
}

#[test]
fn showing_the_reeve_clears_its_highlight() {
    let mut state = alert_state(listing(), &[("p_b0", AgentStatus::Done)]);
    notify(
        &mut state,
        SemanticNotificationKind::Finished,
        "p_b0",
        "ws_b_t0",
    );
    assert_eq!(alerting(&mut state), ["bertil"]);
    // Walk to bertil and show it.
    state.handle_input_bytes(b"\x02w\x1b[B\x1b[B\x1b[B");
    let pick = state.handle_input_bytes(b"\r");
    assert!(matches!(
        picked_method(&pick),
        Some(crate::api::schema::Method::PaneFocus(target)) if target.pane_id == "p_b0"
    ));
    assert!(alerting(&mut state).is_empty());
}

#[test]
fn moving_to_the_hands_pane_clears_its_highlight() {
    let mut state = alert_state(listing(), &[("p_b1", AgentStatus::Blocked)]);
    notify(
        &mut state,
        SemanticNotificationKind::NeedsAttention,
        "p_b1",
        "ws_b_t1",
    );
    assert_eq!(alerting(&mut state), ["bertil", "bertil-probe"]);
    let mut snapshot = agents_snapshot(&[("p_b1", AgentStatus::Blocked)]);
    snapshot.focused_workspace_id = Some("ws_b".into());
    snapshot.focused_tab_id = Some("ws_b_t1".into());
    snapshot.focused_pane_id = Some("p_b1".into());
    state.set_snapshot(Box::new(snapshot));
    assert!(alerting(&mut state).is_empty());
}

#[test]
fn the_first_read_seeds_the_listing_alerts_without_ringing() {
    let mut listing = listing();
    errand_mut(&mut listing, "ansel-fix").hand_state = HandState::Blocked;
    let mut state = alert_state(ReeveListing::default(), &[]);
    // reeve_state's own read was the first: an empty listing seeded the hands as none.
    state.reset_reeve_listing();
    assert!(state.set_reeve_listing(Ok(listing.clone())).is_empty());
    assert!(alerting(&mut state).is_empty());
    // The same read again changes nothing.
    assert!(state.set_reeve_listing(Ok(listing)).is_empty());
}

#[test]
fn each_listing_trigger_rises_once_and_clears_with_its_cause() {
    let mut listing = listing();
    let mut state = alert_state(listing.clone(), &[]);
    let triggers: [fn(&mut crate::client::reeve_listing::ListedErrand); 6] = [
        |errand| errand.hand_state = HandState::Blocked,
        |errand| errand.hand_state = HandState::NeedsDecision,
        |errand| errand.idle = true,
        |errand| errand.wedged = true,
        |errand| errand.waiting = true,
        |errand| errand.question = Some("Which base?".into()),
    ];
    for (index, set) in triggers.into_iter().enumerate() {
        set(errand_mut(&mut listing, "bertil-probe"));
        let effects = state.set_reeve_listing(Ok(listing.clone()));
        assert_eq!(
            sounds(&effects),
            [crate::sound::Sound::Request],
            "trigger {index} rings"
        );
        assert_eq!(alerting(&mut state), ["bertil", "bertil-probe"], "{index}");
        // Read again unchanged: no second ring.
        assert!(state.set_reeve_listing(Ok(listing.clone())).is_empty());
        // Back to a working hand with nothing waiting: the highlight goes.
        *errand_mut(&mut listing, "bertil-probe") = super::reeve_tree::listing().errands[2].clone();
        assert!(state.set_reeve_listing(Ok(listing.clone())).is_empty());
        assert!(alerting(&mut state).is_empty(), "{index} cleared");
    }
}

#[test]
fn a_new_question_rings_when_its_text_changes_to_another() {
    let mut listing = listing();
    let mut state = alert_state(listing.clone(), &[]);
    errand_mut(&mut listing, "ansel-fix").question = Some("Which base?".into());
    assert_eq!(state.set_reeve_listing(Ok(listing.clone())).len(), 1);
    errand_mut(&mut listing, "ansel-fix").question = Some("And the name?".into());
    assert_eq!(state.set_reeve_listing(Ok(listing.clone())).len(), 1);
    errand_mut(&mut listing, "ansel-fix").question = Some(String::new());
    assert!(state.set_reeve_listing(Ok(listing)).is_empty());
    assert!(
        alerting(&mut state).is_empty(),
        "an answered question clears"
    );
}

#[test]
fn a_failed_read_never_rings_and_keeps_the_highlight() {
    let mut listing = listing();
    let mut state = alert_state(listing.clone(), &[]);
    errand_mut(&mut listing, "ansel-fix").hand_state = HandState::Blocked;
    assert_eq!(state.set_reeve_listing(Ok(listing.clone())).len(), 1);
    assert!(state
        .set_reeve_listing(Err("timed out after 5s".into()))
        .is_empty());
    assert_eq!(alerting(&mut state), ["ansel", "ansel-fix"]);
    // A good read like the last one rings nothing new.
    assert!(state.set_reeve_listing(Ok(listing)).is_empty());
}

#[test]
fn a_hand_leaving_the_listing_never_rings_and_its_highlight_goes() {
    let mut listing = listing();
    let mut state = alert_state(listing.clone(), &[]);
    errand_mut(&mut listing, "ansel-fix").wedged = true;
    assert_eq!(state.set_reeve_listing(Ok(listing.clone())).len(), 1);
    listing.errands.retain(|errand| errand.id != "ansel-fix");
    assert!(state.set_reeve_listing(Ok(listing)).is_empty());
    assert!(alerting(&mut state).is_empty());
}

#[test]
fn waiting_on_a_pane_already_read_at_a_dialog_is_not_rung_twice() {
    let mut listing = listing();
    let mut state = alert_state(listing.clone(), &[("p_b1", AgentStatus::Blocked)]);
    assert_eq!(
        sounds(&notify(
            &mut state,
            SemanticNotificationKind::NeedsAttention,
            "p_b1",
            "ws_b_t1"
        )),
        [crate::sound::Sound::Request]
    );
    errand_mut(&mut listing, "bertil-probe").waiting = true;
    assert!(state.set_reeve_listing(Ok(listing.clone())).is_empty());
    assert_eq!(alerting(&mut state), ["bertil", "bertil-probe"]);
    // On a pane hall does not read blocked, waiting rings.
    errand_mut(&mut listing, "ansel-fix").waiting = true;
    assert_eq!(state.set_reeve_listing(Ok(listing)).len(), 1);
}

#[test]
fn several_hands_rising_in_one_read_ring_once() {
    let mut listing = listing();
    let mut state = alert_state(listing.clone(), &[]);
    errand_mut(&mut listing, "ansel-fix").idle = true;
    errand_mut(&mut listing, "bertil-probe").hand_state = HandState::Blocked;
    let effects = state.set_reeve_listing(Ok(listing));
    assert_eq!(sounds(&effects), [crate::sound::Sound::Request]);
    assert_eq!(
        alerting(&mut state),
        ["ansel", "ansel-fix", "bertil", "bertil-probe"]
    );
}

#[test]
fn showing_a_hand_clears_its_listing_alert_until_the_next_rise() {
    let mut listing = listing();
    let mut state = alert_state(listing.clone(), &[]);
    errand_mut(&mut listing, "bertil-probe").hand_state = HandState::Blocked;
    state.set_reeve_listing(Ok(listing.clone()));
    // Walk to the hand and show it.
    state.handle_input_bytes(b"\x02w\x1b[B\x1b[B\x1b[B\x1b[B");
    let pick = state.handle_input_bytes(b"\r");
    assert!(matches!(
        picked_method(&pick),
        Some(crate::api::schema::Method::PaneFocus(target)) if target.pane_id == "p_b1"
    ));
    assert!(alerting(&mut state).is_empty());
    // Still blocked, read again: nothing new rose, nothing rings.
    assert!(state.set_reeve_listing(Ok(listing.clone())).is_empty());
    errand_mut(&mut listing, "bertil-probe").wedged = true;
    assert_eq!(state.set_reeve_listing(Ok(listing)).len(), 1);
    assert_eq!(alerting(&mut state), ["bertil", "bertil-probe"]);
}

#[test]
fn a_muted_sound_config_plays_nothing_while_the_highlight_stands() {
    let muted = crate::config::SoundConfig {
        enabled: false,
        ..Default::default()
    };
    assert!(!muted.allows(None));
    assert!(!muted.allows(Some(crate::detect::Agent::Claude)));
    // The shell raises the highlight whatever the client then does with the sound.
    let mut listing = listing();
    let mut state = alert_state(listing.clone(), &[]);
    errand_mut(&mut listing, "ansel-fix").idle = true;
    assert_eq!(state.set_reeve_listing(Ok(listing)).len(), 1);
    assert_eq!(alerting(&mut state), ["ansel", "ansel-fix"]);
}
