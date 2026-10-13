//! hall: the reeve tree (plan step 3).

use super::*;
use crate::client::reeve_listing::ReeveListing;
use crate::client::shell::reeve_tree::ReeveTreeKey;

const SIDEBAR: usize = 25;

fn workspace(id: &str, number: usize, label: &str, focused: bool) -> ClientShellWorkspace {
    ClientShellWorkspace {
        workspace_id: id.into(),
        active_tab_id: format!("{id}_t0"),
        new_workspace_cwd: "/work".into(),
        number,
        label: label.into(),
        custom_label: true,
        branch: None,
        git_ahead_behind: None,
        tokens: Vec::new(),
        worktree: None,
        focused,
        agent_status: AgentStatus::Idle,
    }
}

/// Three workspaces as the test setup lays them out: two test reeves, each a workspace with its
/// hands as tabs, and a plain workspace beside them. The reeve ansel's own pane has the focus.
fn hall_dev_snapshot() -> ClientShellSnapshot {
    let mut snapshot = snapshot();
    snapshot.focused_workspace_id = Some("ws_a".into());
    snapshot.focused_tab_id = Some("ws_a_t0".into());
    snapshot.focused_pane_id = Some("p_a0".into());
    snapshot.workspaces = vec![
        workspace("ws_a", 1, "ansel", true),
        workspace("ws_b", 2, "bertil", false),
        workspace("ws_p", 3, "scratch", false),
    ];
    let layout = [
        ("ws_a", "t0", "p_a0", "ansel"),
        ("ws_a", "t1", "p_a1", "ansel-fix"),
        ("ws_a", "t2", "p_a2", "ansel-review"),
        ("ws_b", "t0", "p_b0", "bertil"),
        ("ws_b", "t1", "p_b1", "bertil-probe"),
        ("ws_p", "t0", "p_p0", "zsh"),
    ];
    snapshot.tabs = layout
        .iter()
        .enumerate()
        .map(|(index, (ws, tab, _, label))| ClientShellTab {
            tab_id: format!("{ws}_{tab}"),
            workspace_id: (*ws).into(),
            number: index + 1,
            label: (*label).into(),
            custom_label: true,
            zoomed: false,
            focused: *ws == "ws_a" && *tab == "t0",
            agent_status: AgentStatus::Idle,
        })
        .collect();
    snapshot.panes = layout
        .iter()
        .map(|(ws, tab, pane, _)| ClientShellPane {
            pane_id: (*pane).into(),
            workspace_id: (*ws).into(),
            tab_id: format!("{ws}_{tab}"),
            label: None,
            cwd: None,
            foreground_cwd: None,
            focused: *pane == "p_a0",
            right_click_passthrough: false,
        })
        .collect();
    snapshot
}

/// The stand-in's listing for that layout, with every hand working and nothing waiting.
fn listing() -> ReeveListing {
    serde_json::from_value(serde_json::json!({
        "draft": true,
        "reeves": [
            { "name": "ansel", "session": { "herdr_session": "hall-dev", "pane_id": "p_a0" }, "live": true },
            { "name": "bertil", "session": { "herdr_session": "hall-dev", "pane_id": "p_b0" }, "live": true },
            { "name": "elsewhere", "session": { "herdr_session": "hall-dev-other", "pane_id": "p_p0" }, "live": true }
        ],
        "errands": [
            errand("ansel-fix", "ansel", "p_a1"),
            errand("ansel-review", "ansel", "p_a2"),
            errand("bertil-probe", "bertil", "p_b1")
        ]
    }))
    .expect("listing")
}

fn errand(id: &str, owner: &str, pane: &str) -> serde_json::Value {
    serde_json::json!({
        "id": id, "office": "artificer", "owner_reeve": owner,
        "hand_state": "working", "question": null,
        "waiting": false, "idle": false, "wedged": false,
        "session": { "herdr_session": "hall-dev", "pane_id": pane }
    })
}

fn errand_mut<'a>(
    listing: &'a mut ReeveListing,
    id: &str,
) -> &'a mut crate::client::reeve_listing::ListedErrand {
    listing
        .errands
        .iter_mut()
        .find(|errand| errand.id == id)
        .expect("errand")
}

fn reeve_state(listing: ReeveListing) -> ClientShellState {
    let mut config = Config::default();
    config.reeve.listing_command = vec!["true".into()];
    let mut config = ClientShellConfig::from_config(&config);
    config.reeve_tree_session = Some("hall-dev".into());
    let mut state = ClientShellState::new(config);
    state.set_snapshot(Box::new(hall_dev_snapshot()));
    state.set_pane_surface(surface());
    state.set_reeve_listing(Ok(listing));
    state
}

fn rows(frame: &FrameData) -> Vec<String> {
    frame
        .cells
        .chunks(frame.width as usize)
        .map(|row| {
            row.iter()
                .map(|cell| cell.symbol.as_str())
                .collect::<String>()
        })
        .collect()
}

/// The reeve section of the sidebar: its rows up to the first blank row after `other`.
fn tree_rows(frame: &FrameData) -> Vec<String> {
    rows(frame)
        .iter()
        .map(|row| {
            row.chars()
                .take(SIDEBAR)
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .take(10)
        .collect()
}

fn compose(state: &mut ClientShellState) -> FrameData {
    state.compose(100, 30).expect("composed frame")
}

#[test]
fn reeves_sit_at_the_top_with_their_hands_beneath_and_plain_workspaces_apart() {
    let mut state = reeve_state(listing());
    let frame = compose(&mut state);
    assert_eq!(
        tree_rows(&frame),
        [
            " reeves",
            " · ansel",
            "  ├ ● ansel-fix",
            "  └ ● ansel-review",
            " · bertil",
            "  └ ● bertil-probe",
            "",
            " other",
            " ○ scratch",
            "",
        ]
    );
    // The reeve in another hall session is not this session's to show.
    assert!(!rows(&frame).iter().any(|row| row.contains("elsewhere")));
}

#[test]
fn the_key_help_line_is_the_bottom_row() {
    let mut state = reeve_state(listing());
    let frame = compose(&mut state);
    let rows = rows(&frame);
    let bottom = rows.last().expect("bottom row");
    assert_eq!(
        bottom.trim_end(),
        " ctrl+b w reeve tree  ctrl+b ? keys  ctrl+b q detach"
    );
    assert_eq!(state.layout(100, 30).key_help, Rect::new(0, 29, 100, 1));
    assert_eq!(state.layout(100, 30).sidebar.height, 29);
    assert_eq!(state.layout(100, 30).pane_surface.height, 28);

    state.handle_input_bytes(b"\x02w");
    let frame = compose(&mut state);
    assert_eq!(
        self::rows(&frame).last().expect("bottom row").trim_end(),
        " ↑/↓ move  enter show  esc back"
    );
}

#[test]
fn a_hand_state_sets_its_mark_and_what_waits_sets_the_reeve_badge() {
    let mut listing = listing();
    errand_mut(&mut listing, "ansel-fix").hand_state =
        crate::client::reeve_listing::HandState::Blocked;
    errand_mut(&mut listing, "ansel-review").question = Some("Which base?".into());
    errand_mut(&mut listing, "ansel-review").idle = true;
    errand_mut(&mut listing, "bertil-probe").hand_state =
        crate::client::reeve_listing::HandState::Done;
    let mut state = reeve_state(listing.clone());
    let frame = compose(&mut state);
    let tree = tree_rows(&frame);
    assert_eq!(tree[1], " · ansel               2");
    assert_eq!(tree[2], "  ├ ! ansel-fix   blocked");
    assert_eq!(tree[3], "  └ ● ansel-re… asks idle");
    assert_eq!(tree[4], " · bertil");
    assert_eq!(tree[5], "  └ ✓ bertil-probe   done");

    // Back to working: the mark and the badge follow the next read.
    errand_mut(&mut listing, "ansel-fix").hand_state =
        crate::client::reeve_listing::HandState::Working;
    state.set_reeve_listing(Ok(listing));
    let tree = tree_rows(&compose(&mut state));
    assert_eq!(tree[1], " · ansel               1");
    assert_eq!(tree[2], "  ├ ● ansel-fix");
}

#[test]
fn each_flag_and_state_has_its_own_word_and_counts_once() {
    let mut listing = listing();
    {
        let errand = errand_mut(&mut listing, "bertil-probe");
        errand.hand_state = crate::client::reeve_listing::HandState::NeedsDecision;
        errand.question = Some("Ship?".into());
        errand.waiting = true;
        errand.wedged = true;
    }
    let mut state = reeve_state(listing);
    let tree = tree_rows(&compose(&mut state));
    assert_eq!(tree[4], " · bertil              1");
    // A note too long for half the row ends in an ellipsis, as a long label does.
    assert_eq!(tree[5], "  └ ? bertil-p… decide a…");
    let tree_model = state.current_reeve_tree().expect("tree");
    let probe = &tree_model.items[tree_model
        .position(&ReeveTreeKey::Hand("bertil-probe".into()))
        .expect("probe")];
    assert!(matches!(
        &probe.kind,
        crate::client::shell::reeve_tree::ReeveTreeItemKind::Hand { notes, .. }
            if notes == &["decide", "asks", "waiting", "wedged"]
    ));
}

#[test]
fn a_finished_and_cleaned_up_errand_drops_from_the_tree() {
    let mut listing = listing();
    let mut state = reeve_state(listing.clone());
    assert!(rows(&compose(&mut state))
        .iter()
        .any(|row| row.contains("ansel-review")));
    listing.errands.retain(|errand| errand.id != "ansel-review");
    state.set_reeve_listing(Ok(listing));
    let tree = tree_rows(&compose(&mut state));
    assert_eq!(tree[2], "  └ ● ansel-fix");
    assert_eq!(tree[3], " · bertil");
    assert!(!tree.iter().any(|row| row.contains("ansel-review")));
}

#[test]
fn a_hand_without_a_pane_and_a_gone_reeve_show_dimmed_notes() {
    let mut listing = listing();
    errand_mut(&mut listing, "ansel-fix").session.pane_id = None;
    listing.reeves[1].live = false;
    let mut state = reeve_state(listing);
    let tree = tree_rows(&compose(&mut state));
    assert_eq!(tree[2], "  ├ ● ansel-fix   no pane");
    assert_eq!(tree[4], " · bertil            gone");
}

#[test]
fn a_listing_error_shows_and_every_workspace_stays_reachable() {
    let mut state = reeve_state(listing());
    state.set_reeve_listing(Err("refusing home".into()));
    let tree = tree_rows(&compose(&mut state));
    assert_eq!(tree[1], " listing: refusing home");
    assert_eq!(tree[3], " other");
    assert_eq!(tree[4], " ○ ansel");
    assert_eq!(tree[6], " ○ scratch");
}

#[test]
fn keys_walk_the_tree_and_picking_a_hand_shows_its_pane() {
    let mut state = reeve_state(listing());
    compose(&mut state);
    state.handle_input_bytes(b"\x02w");
    assert_eq!(state.mode, ClientShellMode::Navigate);
    assert_eq!(
        state.navigate_reeve_key,
        Some(ReeveTreeKey::Reeve("ansel".into()))
    );

    state.handle_input_bytes(b"\x1b[B");
    assert_eq!(
        state.navigate_reeve_key,
        Some(ReeveTreeKey::Hand("ansel-fix".into()))
    );
    let frame = compose(&mut state);
    let selected = state.hits.agents.iter().find(|(_, pane)| pane == "p_a1");
    let (rect, _) = selected.expect("hand row hit");
    let cell = usize::from(rect.y) * usize::from(frame.width) + 2;
    assert_eq!(
        frame.cells[cell].bg,
        crate::protocol::color_to_u32(state.config.palette.selection_bg),
        "the selected hand row is highlighted"
    );

    for _ in 0..4 {
        state.handle_input_bytes(b"\x1b[B");
    }
    assert_eq!(
        state.navigate_reeve_key,
        Some(ReeveTreeKey::Workspace("ws_p".into()))
    );
    state.handle_input_bytes(b"\x1b[B");
    assert_eq!(
        state.navigate_reeve_key,
        Some(ReeveTreeKey::Reeve("ansel".into())),
        "the walk wraps"
    );
    state.handle_input_bytes(b"\x1b[A\x1b[A");
    assert_eq!(
        state.navigate_reeve_key,
        Some(ReeveTreeKey::Hand("bertil-probe".into()))
    );

    let pick = state.handle_input_bytes(b"\r");
    assert_eq!(state.mode, ClientShellMode::Terminal);
    assert!(matches!(
        &pick.actions[..],
        [ClientShellAction::Endpoint { request, .. }]
            if matches!(
                &request.method,
                crate::api::schema::Method::PaneFocus(target) if target.pane_id == "p_b1"
            )
    ));
}

#[test]
fn picking_a_reeve_shows_its_own_pane_and_a_plain_workspace_focuses_it() {
    let mut state = reeve_state(listing());
    compose(&mut state);
    state.handle_input_bytes(b"\x02w\x1b[B\x1b[B\x1b[B");
    let pick = state.handle_input_bytes(b"\r");
    assert!(matches!(
        &pick.actions[..],
        [ClientShellAction::Endpoint { request, .. }]
            if matches!(
                &request.method,
                crate::api::schema::Method::PaneFocus(target) if target.pane_id == "p_b0"
            )
    ));

    state.handle_input_bytes(b"\x02w\x1b[A");
    let pick = state.handle_input_bytes(b"\r");
    assert!(matches!(
        &pick.actions[..],
        [ClientShellAction::Endpoint { request, .. }]
            if matches!(
                &request.method,
                crate::api::schema::Method::WorkspaceFocus(target) if target.workspace_id == "ws_p"
            )
    ));
}

#[test]
fn the_focused_hand_tab_marks_the_hand_not_its_reeve() {
    let mut snapshot = hall_dev_snapshot();
    snapshot.focused_tab_id = Some("ws_a_t2".into());
    snapshot.focused_pane_id = Some("p_a2".into());
    let mut state = reeve_state(listing());
    state.set_snapshot(Box::new(snapshot));
    let tree = state.current_reeve_tree().expect("tree");
    let focused = tree
        .items
        .iter()
        .filter(|item| item.focused)
        .map(|item| item.key.clone())
        .collect::<Vec<_>>();
    assert_eq!(focused, [ReeveTreeKey::Hand("ansel-review".into())]);
    state.handle_input_bytes(b"\x02w");
    assert_eq!(
        state.navigate_reeve_key,
        Some(ReeveTreeKey::Hand("ansel-review".into()))
    );
}

#[test]
fn clicking_a_hand_row_shows_its_pane() {
    let mut state = reeve_state(listing());
    compose(&mut state);
    let (rect, _) = state
        .hits
        .agents
        .iter()
        .find(|(_, pane)| pane == "p_a2")
        .cloned()
        .expect("hand row hit");
    let click = state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: rect.x + 4,
        row: rect.y,
        modifiers: KeyModifiers::empty(),
    })]);
    assert!(matches!(
        &click.actions[..],
        [ClientShellAction::Endpoint { request, .. }]
            if matches!(
                &request.method,
                crate::api::schema::Method::PaneFocus(target) if target.pane_id == "p_a2"
            )
    ));
}

#[test]
fn without_a_listing_command_the_sidebar_and_keys_stay_herdrs() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    assert_eq!(state.config.reeve_tree_session, None);
    state.set_snapshot(Box::new(hall_dev_snapshot()));
    state.set_pane_surface(surface());
    let frame = compose(&mut state);
    assert_eq!(tree_rows(&frame)[0], " spaces");
    assert!(state.layout(100, 30).key_help.is_empty());
    assert_eq!(state.layout(100, 30).sidebar.height, 30);
    assert!(!rows(&frame).iter().any(|row| row.contains("reeve tree")));
    state.handle_input_bytes(b"\x02w\x1b[B");
    assert_eq!(state.navigate_reeve_key, None);
    assert_eq!(state.navigate_workspace_id.as_deref(), Some("ws_b"));
}

#[test]
fn a_long_tree_scrolls_to_keep_the_selection_in_view() {
    let mut listing = listing();
    for index in 0..20 {
        listing.errands.push(
            serde_json::from_value(errand(&format!("extra-{index}"), "bertil", "p_b1"))
                .expect("errand"),
        );
    }
    let mut state = reeve_state(listing);
    compose(&mut state);
    assert!(state.hits.workspace_max_scroll > 0);
    state.handle_input_bytes(b"\x02w");
    for _ in 0..20 {
        state.handle_input_bytes(b"\x1b[B");
    }
    let frame = compose(&mut state);
    let selected = match state.navigate_reeve_key.clone() {
        Some(ReeveTreeKey::Hand(id)) => id,
        other => panic!("expected a hand, got {other:?}"),
    };
    assert!(state.workspace_scroll > 0);
    assert!(rows(&frame).iter().any(|row| row.contains(&selected)));
}
