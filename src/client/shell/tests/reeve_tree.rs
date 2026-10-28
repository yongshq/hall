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
pub(super) fn hall_dev_snapshot() -> ClientShellSnapshot {
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

/// The stand-in's listing for that layout, with every hand working and nothing waiting. Its
/// errands name no office, so each hand's row reads as its bare errand id.
pub(super) fn listing() -> ReeveListing {
    serde_json::from_value(serde_json::json!({
        "version": 1,
        "reeves": [
            { "name": "ansel", "session": { "backend": "hall", "name": "hall-dev", "pane_id": "p_a0" }, "live": true },
            { "name": "bertil", "session": { "backend": "hall", "name": "hall-dev", "pane_id": "p_b0" }, "live": true },
            { "name": "elsewhere", "session": { "backend": "hall", "name": "hall-dev-other", "pane_id": "p_p0" }, "live": true }
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
        "id": id, "office": null, "owner_reeve": owner,
        "hand_state": "working", "question": null,
        "waiting": false, "idle": false, "wedged": false,
        "session": { "backend": "hall", "name": "hall-dev", "pane_id": pane }
    })
}

pub(super) fn errand_mut<'a>(
    listing: &'a mut ReeveListing,
    id: &str,
) -> &'a mut crate::client::reeve_listing::ListedErrand {
    listing
        .errands
        .iter_mut()
        .find(|errand| errand.id == id)
        .expect("errand")
}

pub(super) fn reeve_state(listing: ReeveListing) -> ClientShellState {
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

pub(super) fn rows(frame: &FrameData) -> Vec<String> {
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
pub(super) fn tree_rows(frame: &FrameData) -> Vec<String> {
    rows(frame)
        .iter()
        .map(|row| {
            row.chars()
                .take(SIDEBAR)
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .take(11)
        .collect()
}

/// The tree's rows at a sidebar 25 wide: the 24 columns left of its divider.
fn narrow_tree_rows(frame: &FrameData) -> Vec<String> {
    rows(frame)
        .iter()
        .map(|row| {
            row.chars()
                .take(24)
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .take(11)
        .collect()
}

pub(super) fn compose(state: &mut ClientShellState) -> FrameData {
    state.compose(100, 30).expect("composed frame")
}

#[test]
fn reeves_sit_at_the_top_with_their_hands_beneath_and_plain_workspaces_apart() {
    let mut state = reeve_state(listing());
    let frame = compose(&mut state);
    assert_eq!(
        tree_rows(&frame),
        [
            " The Great Hall",
            " Reeves in attendance",
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
    // The heading is bold; the line beneath it is the panel's plain secondary text.
    let overlay = crate::protocol::color_to_u32(state.config.palette.overlay0);
    let heading = &frame.cells[1];
    assert_eq!(
        (heading.fg, heading.modifier),
        (overlay, Modifier::BOLD.bits())
    );
    let line = &frame.cells[frame.width as usize + 1];
    assert_eq!(
        (line.fg, line.modifier),
        (overlay, Modifier::empty().bits())
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
    // The first read raises the highlight of a hand with a flag up, and its reeve's.
    assert_eq!(tree[2], "▌· ansel               2");
    assert_eq!(tree[3], "  ├ ! ansel-fix   blocked");
    // The id first; of the notes, the ones that fit after it.
    assert_eq!(tree[4], "▌ └ ● ansel-review   asks");
    assert_eq!(tree[5], " · bertil");
    // A done hand shows no row (decision 73).
    assert_eq!(tree[6], "");

    // Back to working: the mark and the badge follow the next read.
    errand_mut(&mut listing, "ansel-fix").hand_state =
        crate::client::reeve_listing::HandState::Working;
    state.set_reeve_listing(Ok(listing));
    let tree = tree_rows(&compose(&mut state));
    assert_eq!(tree[2], "▌· ansel               1");
    assert_eq!(tree[3], "  ├ ● ansel-fix");
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
    // The first read raises the highlight of a hand with a flag up, and its reeve's.
    assert_eq!(tree[5], "▌· bertil              1");
    // At the default sidebar width the id stays whole and the notes that do not fit go: the
    // ones the mark already says (`decide`, and `asks` beside `?`) first, so `wedged` stays.
    assert_eq!(tree[6], "▌ └ ? bertil-probe wedged");
    let tree_model = state.current_reeve_tree().expect("tree");
    let probe = &tree_model.items[tree_model
        .position(&ReeveTreeKey::Hand("bertil-probe".into()))
        .expect("probe")];
    assert!(matches!(
        &probe.kind,
        crate::client::shell::reeve_tree::ReeveTreeItemKind::Hand { notes, .. }
            if notes == &["decide", "asks", "waiting", "wedged"]
    ));
    // With room for them all, every note shows, in decision 52's order.
    state.config.sidebar_max_width = 50;
    state.sidebar_width = 50;
    let row = rows(&compose(&mut state))[6]
        .chars()
        .take(49)
        .collect::<String>();
    assert_eq!(row, "▌ └ ? bertil-probe     decide asks waiting wedged");
}

#[test]
fn a_narrow_row_keeps_the_notes_the_mark_does_not_say() {
    let mut listing = listing();
    {
        // Working: `asks` says what the mark does not, and goes before `waiting`.
        let errand = errand_mut(&mut listing, "bertil-probe");
        errand.question = Some("Ship?".into());
        errand.waiting = true;
    }
    {
        // Blocked: the mark says blocked, so a flag that fits takes the room.
        let errand = errand_mut(&mut listing, "ansel-fix");
        errand.hand_state = crate::client::reeve_listing::HandState::Blocked;
        errand.idle = true;
    }
    let mut state = reeve_state(listing);
    let tree = tree_rows(&compose(&mut state));
    // The first read raises the highlight of a hand with a flag up.
    assert_eq!(tree[3], "▌ ├ ! ansel-fix      idle");
    assert_eq!(tree[6], "▌ └ ● bertil-probe   asks");
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
    // Finished: the hand is gone; its tab, still open, shows as a tab of its reeve.
    let tree = tree_rows(&compose(&mut state));
    assert_eq!(tree[3], "  ├ ● ansel-fix");
    assert_eq!(tree[4], "  └ tab ansel-review");
    // Cleaned up: the tab closes and nothing of the errand is left.
    let mut snapshot = hall_dev_snapshot();
    snapshot.tabs.retain(|tab| tab.tab_id != "ws_a_t2");
    snapshot.panes.retain(|pane| pane.tab_id != "ws_a_t2");
    state.set_snapshot(Box::new(snapshot));
    let tree = tree_rows(&compose(&mut state));
    assert_eq!(tree[3], "  └ ● ansel-fix");
    assert_eq!(tree[4], " · bertil");
    assert!(!tree.iter().any(|row| row.contains("ansel-review")));
}

#[test]
fn a_hand_without_a_pane_drops_and_a_gone_reeve_shows_dimmed() {
    let mut listing = listing();
    errand_mut(&mut listing, "ansel-fix").session.pane_id = None;
    listing.reeves[1].live = false;
    let mut state = reeve_state(listing);
    let tree = tree_rows(&compose(&mut state));
    assert_eq!(tree[3], "  ├ ● ansel-review");
    // Hall cannot tie the hand to its tab, so the tab shows as one of the reeve's.
    assert_eq!(tree[4], "  └ tab ansel-fix");
    assert_eq!(tree[5], " · bertil            gone");
}

#[test]
fn a_hand_row_names_its_office_before_its_errand() {
    let mut listing = listing();
    errand_mut(&mut listing, "ansel-fix").office = "scout".into();
    errand_mut(&mut listing, "ansel-review").office = "  ".into();
    errand_mut(&mut listing, "bertil-probe").office = "warden".into();
    let mut state = reeve_state(listing);
    let tree = tree_rows(&compose(&mut state));
    assert_eq!(tree[3], "  ├ ● scout-ansel-fix");
    // An office of blanks names none: the errand id alone.
    assert_eq!(tree[4], "  └ ● ansel-review");
    assert_eq!(tree[6], "  └ ● warden-bertil-probe");
    // The row still picks the hand by its errand.
    state.handle_input_bytes(b"\x02w\x1b[B");
    assert_eq!(
        state.navigate_reeve_key,
        Some(ReeveTreeKey::Hand("ansel-fix".into()))
    );
}

#[test]
fn a_working_hand_whose_pane_closed_shows_no_row_and_still_counts_on_its_reeve() {
    let mut listing = listing();
    errand_mut(&mut listing, "ansel-review").question = Some("Land it?".into());
    let mut state = reeve_state(listing.clone());
    let tree = tree_rows(&compose(&mut state));
    // Working with its pane open: a row, and its question counts toward the reeve's badge.
    assert_eq!(tree[2], " · ansel               1");
    assert_eq!(tree[4], "  └ ● ansel-review   asks");
    // Its pane closes while the listing still names it working and asking: no row, and its
    // question still counts (decisions 70, 74).
    let mut snapshot = hall_dev_snapshot();
    snapshot.tabs.retain(|tab| tab.tab_id != "ws_a_t2");
    snapshot.panes.retain(|pane| pane.tab_id != "ws_a_t2");
    state.set_snapshot(Box::new(snapshot));
    state.set_reeve_listing(Ok(listing));
    let tree = tree_rows(&compose(&mut state));
    assert_eq!(tree[2], " · ansel               1");
    assert_eq!(tree[3], "  └ ● ansel-fix");
    assert_eq!(tree[4], " · bertil");
    assert!(!tree.iter().any(|row| row.contains("ansel-review")));
}

#[test]
fn done_and_failed_hands_show_no_row_while_their_panes_are_open() {
    let mut listing = listing();
    {
        let errand = errand_mut(&mut listing, "ansel-review");
        errand.hand_state = crate::client::reeve_listing::HandState::Done;
        errand.waiting = true;
    }
    errand_mut(&mut listing, "bertil-probe").hand_state =
        crate::client::reeve_listing::HandState::Failed;
    let mut state = reeve_state(listing);
    let tree = tree_rows(&compose(&mut state));
    // Only the hand still working has a row. The done hand waiting on the liege still counts
    // toward its reeve's badge; the failed one, waiting on nothing, counts toward none.
    assert_eq!(tree[2], "▌· ansel               1");
    assert_eq!(tree[3], "  └ ● ansel-fix");
    assert_eq!(tree[4], " · bertil");
    assert_eq!(tree[5], "");
    assert_eq!(tree[6], " other");
    // Their tabs, still open, stay tied to them: no `tab` row stands for either.
    assert!(!tree.iter().any(|row| row.contains("tab ")));
    assert!(!tree
        .iter()
        .any(|row| row.contains("ansel-review") || row.contains("bertil-probe")));
    let model = state.current_reeve_tree().expect("tree");
    let beneath = model
        .items
        .iter()
        .filter(|item| matches!(item.key, ReeveTreeKey::Tab(_) | ReeveTreeKey::Hand(_)))
        .map(|item| item.key.clone())
        .collect::<Vec<_>>();
    assert_eq!(beneath, [ReeveTreeKey::Hand("ansel-fix".into())]);
    // With the done hand's tab focused, the focus shows on its reeve's row.
    let mut snapshot = hall_dev_snapshot();
    snapshot.focused_tab_id = Some("ws_a_t2".into());
    snapshot.focused_pane_id = Some("p_a2".into());
    state.set_snapshot(Box::new(snapshot));
    let model = state.current_reeve_tree().expect("tree");
    let focused = model
        .items
        .iter()
        .filter(|item| item.focused)
        .map(|item| item.key.clone())
        .collect::<Vec<_>>();
    assert_eq!(focused, [ReeveTreeKey::Reeve("ansel".into())]);
}

#[test]
fn a_failed_hand_that_waits_on_the_liege_marks_and_counts_on_its_reeve() {
    let mut listing = listing();
    errand_mut(&mut listing, "bertil-probe").hand_state =
        crate::client::reeve_listing::HandState::Failed;
    let mut state = reeve_state(listing.clone());
    let tree = tree_rows(&compose(&mut state));
    assert_eq!(tree[5], " · bertil");
    // It starts waiting and asks: the alert rings and marks its reeve's row, and it counts
    // once toward its reeve's badge, with no row of its own (decisions 63, 73).
    {
        let errand = errand_mut(&mut listing, "bertil-probe");
        errand.waiting = true;
        errand.question = Some("Retry?".into());
    }
    let effects = state.set_reeve_listing(Ok(listing));
    assert!(effects
        .iter()
        .any(|effect| matches!(effect, ClientShellNotificationEffect::Sound { .. })));
    let tree = tree_rows(&compose(&mut state));
    assert_eq!(tree[5], "▌· bertil              1");
    assert_eq!(tree[6], "");
    assert!(!tree.iter().any(|row| row.contains("bertil-probe")));
    // With the failed hand's tab focused, the focus shows on its reeve's row.
    let mut snapshot = hall_dev_snapshot();
    snapshot.focused_workspace_id = Some("ws_b".into());
    snapshot.focused_tab_id = Some("ws_b_t1".into());
    snapshot.focused_pane_id = Some("p_b1".into());
    state.set_snapshot(Box::new(snapshot));
    let model = state.current_reeve_tree().expect("tree");
    let focused = model
        .items
        .iter()
        .filter(|item| item.focused)
        .map(|item| item.key.clone())
        .collect::<Vec<_>>();
    assert_eq!(focused, [ReeveTreeKey::Reeve("bertil".into())]);
}

fn no_listing_state() -> ClientShellState {
    let mut state = reeve_state(listing());
    state.reset_reeve_listing();
    state
}

#[test]
fn before_the_first_read_the_tree_says_it_is_reading() {
    let mut state = no_listing_state();
    let tree = tree_rows(&compose(&mut state));
    assert_eq!(tree[2], " reading listing");
    assert_eq!(tree[4], " other");
    assert_eq!(tree[5], " ○ ansel");
}

#[test]
fn a_listing_error_with_no_good_read_shows_and_every_workspace_stays_reachable() {
    let mut state = no_listing_state();
    state.set_reeve_listing(Err("refusing home".into()));
    let frame = compose(&mut state);
    let tree = tree_rows(&frame);
    assert_eq!(tree[2], " listing: refusing home");
    assert_eq!(tree[4], " other");
    assert_eq!(tree[5], " ○ ansel");
    assert_eq!(tree[7], " ○ scratch");
    let cell = 2 * frame.width as usize + 2;
    assert_eq!(
        frame.cells[cell].fg,
        crate::protocol::color_to_u32(state.config.palette.yellow),
        "the reason stands out"
    );
}

#[test]
fn a_failed_read_after_a_good_one_keeps_the_tree_marked_stale() {
    let mut state = reeve_state(listing());
    state.set_reeve_listing(Err("timed out after 5s".into()));
    let tree = tree_rows(&compose(&mut state));
    assert_eq!(tree[2], " stale: timed out after");
    assert_eq!(tree[3], " 5s");
    assert_eq!(tree[4], " · ansel");
    assert_eq!(tree[5], "  ├ ● ansel-fix");
    // The next good read clears the mark.
    state.set_reeve_listing(Ok(listing()));
    let tree = tree_rows(&compose(&mut state));
    assert_eq!(tree[2], " · ansel");
}

#[test]
fn at_width_25_a_long_reason_wraps_onto_rows_of_its_own() {
    let mut state = no_listing_state();
    state.sidebar_width = 25;
    state.set_reeve_listing(Err("not found: /nonexistent/hall-listing".into()));
    let tree = narrow_tree_rows(&compose(&mut state));
    assert_eq!(state.layout(100, 30).sidebar.width, 25);
    assert_eq!(tree[2], " listing: not found:");
    assert_eq!(tree[3], " /nonexistent/hall-listi");
    assert_eq!(tree[4], " ng");
    assert_eq!(tree[6], " other");
}

#[test]
fn at_width_25_a_hand_keeps_its_id_before_its_notes() {
    let mut listing = listing();
    {
        let errand = errand_mut(&mut listing, "bertil-probe");
        errand.hand_state = crate::client::reeve_listing::HandState::NeedsDecision;
        errand.wedged = true;
    }
    {
        let errand = errand_mut(&mut listing, "ansel-fix");
        errand.id = "ansel-fix-the-long-one".into();
        errand.hand_state = crate::client::reeve_listing::HandState::Blocked;
    }
    let mut state = reeve_state(listing);
    state.sidebar_width = 25;
    let tree = narrow_tree_rows(&compose(&mut state));
    assert_eq!(state.layout(100, 30).sidebar.width, 25);
    // The heading and its line fit whole.
    assert_eq!(tree[0], " The Great Hall");
    assert_eq!(tree[1], " Reeves in attendance");
    // Too long for the row: the id is cut, no note.
    assert_eq!(tree[3], "  ├ ! ansel-fix-the-lon…");
    // `decide wedged` does not fit beside the whole id: the id stays, the mark says decide.
    // The bar is the highlight the first read raises for a hand with a flag up.
    assert_eq!(tree[6], "▌ └ ? bertil-probe");
}

#[test]
fn at_the_minimum_width_the_heading_line_ends_in_an_ellipsis() {
    let mut state = reeve_state(listing());
    state.sidebar_width = state.config.sidebar_min_width;
    let frame = compose(&mut state);
    assert_eq!(state.layout(100, 30).sidebar.width, 18);
    // The 17 columns left of the divider.
    let tree = rows(&frame)
        .iter()
        .map(|row| {
            row.chars()
                .take(17)
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .take(3)
        .collect::<Vec<_>>();
    assert_eq!(tree[0], " The Great Hall");
    // Cut the way the tree rows are, with an ellipsis in the last column, not mid-word.
    assert_eq!(tree[1], " Reeves in atten…");
    assert_eq!(tree[2], " · ansel");
}

/// The reeve section drawn alone, 24 wide and `height` rows high, and the body it hit-tests.
fn section_rows(state: &ClientShellState, height: u16) -> (Vec<String>, Rect) {
    let tree = state.current_reeve_tree().expect("tree");
    let area = Rect::new(0, 0, 24, height);
    let mut buffer = Buffer::empty(area);
    let mut hits = ShellHitMap::default();
    crate::client::shell::reeve_tree::render_reeve_section(
        &mut buffer,
        area,
        &tree,
        &state.config,
        None,
        &mut 0,
        &mut hits,
    );
    let rows = (0..height)
        .map(|y| {
            (0..area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect();
    (rows, hits.workspace_body)
}

#[test]
fn the_heading_is_cut_with_an_ellipsis_in_a_narrow_section() {
    let state = reeve_state(listing());
    let tree = state.current_reeve_tree().expect("tree");
    let first_row = |width: u16| {
        let area = Rect::new(0, 0, width, 6);
        let mut buffer = Buffer::empty(area);
        crate::client::shell::reeve_tree::render_reeve_section(
            &mut buffer,
            area,
            &tree,
            &state.config,
            None,
            &mut 0,
            &mut ShellHitMap::default(),
        );
        (0..width)
            .map(|x| buffer[(x, 0)].symbol())
            .collect::<String>()
            .trim_end()
            .to_owned()
    };
    assert_eq!(first_row(12), " The Great …");
    assert_eq!(first_row(15), " The Great Hall");
    assert_eq!(first_row(26), " The Great Hall");
}

#[test]
fn a_short_section_keeps_a_tree_row_before_the_heading_line() {
    let state = reeve_state(listing());
    // The last row is the footer's, drawn by the sidebar.
    assert_eq!(
        section_rows(&state, 2),
        (
            vec![" The Great Hall".into(), String::new()],
            Rect::new(0, 1, 24, 0)
        )
    );
    // Three rows: the one tree row wins over the heading line.
    assert_eq!(
        section_rows(&state, 3),
        (
            vec![" The Great Hall".into(), " · ansel".into(), String::new()],
            Rect::new(0, 1, 24, 1)
        )
    );
    // Four: room for both.
    assert_eq!(
        section_rows(&state, 4),
        (
            vec![
                " The Great Hall".into(),
                " Reeves in attendance".into(),
                " · ansel".into(),
                String::new()
            ],
            Rect::new(0, 2, 24, 1)
        )
    );
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

pub(super) fn picked_method(input: &ClientShellInput) -> Option<&crate::api::schema::Method> {
    match &input.actions[..] {
        [ClientShellAction::Endpoint { request, .. }] => Some(&request.method),
        _ => None,
    }
}

#[test]
fn a_tab_of_a_reeve_that_holds_no_hand_shows_dimmed_under_its_reeve() {
    let mut snapshot = hall_dev_snapshot();
    snapshot.tabs.push(ClientShellTab {
        tab_id: "ws_a_t3".into(),
        workspace_id: "ws_a".into(),
        number: 7,
        label: "notes".into(),
        custom_label: true,
        zoomed: false,
        focused: false,
        agent_status: AgentStatus::Idle,
    });
    snapshot.panes.push(ClientShellPane {
        pane_id: "p_a3".into(),
        workspace_id: "ws_a".into(),
        tab_id: "ws_a_t3".into(),
        label: None,
        cwd: None,
        foreground_cwd: None,
        focused: false,
        right_click_passthrough: false,
    });
    let mut state = reeve_state(listing());
    state.set_snapshot(Box::new(snapshot));
    let frame = compose(&mut state);
    let tree = tree_rows(&frame);
    assert_eq!(tree[2], " · ansel");
    assert_eq!(tree[3], "  ├ ● ansel-fix");
    assert_eq!(tree[4], "  ├ ● ansel-review");
    assert_eq!(tree[5], "  └ tab notes");
    assert_eq!(tree[6], " · bertil");
    let cell = 5 * frame.width as usize + 4;
    assert_eq!(
        frame.cells[cell].fg,
        crate::protocol::color_to_u32(state.config.palette.overlay0),
        "the tab row is dimmed"
    );

    // Walked to and picked, it brings the tab forward.
    state.handle_input_bytes(b"\x02w\x1b[B\x1b[B\x1b[B");
    assert_eq!(
        state.navigate_reeve_key,
        Some(ReeveTreeKey::Tab("ws_a_t3".into()))
    );
    let pick = state.handle_input_bytes(b"\r");
    assert!(matches!(
        picked_method(&pick),
        Some(crate::api::schema::Method::TabFocus(target)) if target.tab_id == "ws_a_t3"
    ));

    // A click on its row does the same.
    compose(&mut state);
    let (rect, _) = state.hits.reeve_tabs[0].clone();
    let click = state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: rect.x + 4,
        row: rect.y,
        modifiers: KeyModifiers::empty(),
    })]);
    assert!(matches!(
        picked_method(&click),
        Some(crate::api::schema::Method::TabFocus(target)) if target.tab_id == "ws_a_t3"
    ));
}

#[test]
fn the_walk_line_names_the_navigate_keys_as_bound() {
    let mut config = Config::default();
    config.reeve.listing_command = vec!["true".into()];
    config.keys.navigate_workspace_up = crate::config::BindingConfig::one("k");
    config.keys.navigate_workspace_down =
        crate::config::BindingConfig::Many(vec!["down".into(), "j".into()]);
    let mut shell_config = ClientShellConfig::from_config(&config);
    shell_config.reeve_tree_session = Some("hall-dev".into());
    let mut state = ClientShellState::new(shell_config);
    state.set_snapshot(Box::new(hall_dev_snapshot()));
    state.set_pane_surface(surface());
    state.set_reeve_listing(Ok(listing()));
    compose(&mut state);
    state.handle_input_bytes(b"\x02w");
    assert_eq!(
        rows(&compose(&mut state))
            .last()
            .expect("bottom row")
            .trim_end(),
        " k/↓,j move  enter show  esc back"
    );
    // And k does move the walk.
    state.handle_input_bytes(b"k");
    assert_eq!(
        state.navigate_reeve_key,
        Some(ReeveTreeKey::Workspace("ws_p".into()))
    );
}

#[test]
fn digits_in_the_walk_follow_the_trees_order() {
    // The listing names bertil before ansel, against hall's workspace order.
    let mut listing = listing();
    listing.reeves.swap(0, 1);
    let mut state = reeve_state(listing);
    let tree = tree_rows(&compose(&mut state));
    assert_eq!(tree[2], " · bertil");

    state.handle_input_bytes(b"\x02w");
    let pick = state.handle_input_bytes(b"1");
    assert_eq!(state.mode, ClientShellMode::Terminal);
    assert!(matches!(
        picked_method(&pick),
        Some(crate::api::schema::Method::PaneFocus(target)) if target.pane_id == "p_b0"
    ));

    // Hands are not counted: 3 is the first workspace under `other`.
    state.handle_input_bytes(b"\x02w");
    let pick = state.handle_input_bytes(b"3");
    assert!(matches!(
        picked_method(&pick),
        Some(crate::api::schema::Method::WorkspaceFocus(target)) if target.workspace_id == "ws_p"
    ));

    // A digit past the last row does nothing and stays in the walk.
    state.handle_input_bytes(b"\x02w");
    let pick = state.handle_input_bytes(b"9");
    assert!(pick.actions.is_empty());
    assert_eq!(state.mode, ClientShellMode::Navigate);
}

#[test]
fn workspace_actions_from_a_hand_row_or_a_paneless_reeve_do_nothing() {
    let mut listing = listing();
    listing.reeves[1].session.pane_id = None;
    let mut state = reeve_state(listing);
    compose(&mut state);
    state.handle_input_bytes(b"\x02w\x1b[B");
    assert_eq!(
        state.navigate_reeve_key,
        Some(ReeveTreeKey::Hand("ansel-fix".into()))
    );
    // Shift+d closes the selected workspace in herdr's walk; on a hand it closes nothing.
    let close = state.handle_input_bytes(b"D");
    assert!(close.actions.is_empty());
    assert!(
        state.overlay.is_none(),
        "no offer to close the reeve's workspace"
    );

    // bertil has no pane: its row names no workspace either.
    state.handle_input_bytes(b"\x02w\x1b[B\x1b[B\x1b[B");
    assert_eq!(
        state.navigate_reeve_key,
        Some(ReeveTreeKey::Reeve("bertil".into()))
    );
    let close = state.handle_input_bytes(b"D");
    assert!(close.actions.is_empty());
    assert!(state.overlay.is_none());

    // A reeve's own row still acts on its workspace, behind herdr's confirm.
    state.handle_input_bytes(b"\x02w");
    assert_eq!(
        state.navigate_reeve_key,
        Some(ReeveTreeKey::Reeve("ansel".into()))
    );
    state.handle_input_bytes(b"D");
    assert!(matches!(
        &state.overlay,
        Some(ClientShellOverlay::ConfirmClose(confirm)) if confirm.workspace_id == "ws_a"
    ));
}

#[test]
fn a_hand_that_finishes_while_selected_offers_no_workspace_action() {
    let mut listing = listing();
    let mut state = reeve_state(listing.clone());
    compose(&mut state);
    state.handle_input_bytes(b"\x02w\x1b[B\x1b[B");
    assert_eq!(
        state.navigate_reeve_key,
        Some(ReeveTreeKey::Hand("ansel-review".into()))
    );
    assert_eq!(state.navigate_workspace_id.as_deref(), Some("ws_a"));
    // The hand finishes mid-walk: its row becomes a tab of its reeve, and the selection
    // follows it there.
    listing.errands.retain(|errand| errand.id != "ansel-review");
    state.set_reeve_listing(Ok(listing));
    assert_eq!(tree_rows(&compose(&mut state))[4], "  └ tab ansel-review");
    assert_eq!(
        state.navigate_reeve_key,
        Some(ReeveTreeKey::Tab("ws_a_t2".into()))
    );
    assert_eq!(state.reeve_walk_workspace_target(), Some(None));
    let close = state.handle_input_bytes(b"D");
    assert!(close.actions.is_empty());
    assert!(
        state.overlay.is_none(),
        "no offer to close the reeve's workspace"
    );
}

#[test]
fn enter_on_a_hand_that_finished_while_selected_shows_its_tab() {
    let mut listing = listing();
    let mut state = reeve_state(listing.clone());
    compose(&mut state);
    state.handle_input_bytes(b"\x02w\x1b[B\x1b[B");
    listing.errands.retain(|errand| errand.id != "ansel-review");
    state.set_reeve_listing(Ok(listing));
    let frame = compose(&mut state);
    // The selection is drawn on the row that took the hand's place.
    assert_eq!(tree_rows(&frame)[4], "  └ tab ansel-review");
    let (rect, _) = state.hits.reeve_tabs[0].clone();
    let cell = usize::from(rect.y) * usize::from(frame.width) + 2;
    assert_eq!(
        frame.cells[cell].bg,
        crate::protocol::color_to_u32(state.config.palette.selection_bg),
        "the tab row is highlighted"
    );
    let pick = state.handle_input_bytes(b"\r");
    assert!(matches!(
        picked_method(&pick),
        Some(crate::api::schema::Method::TabFocus(target)) if target.tab_id == "ws_a_t2"
    ));
}

#[test]
fn a_hand_set_done_while_selected_passes_the_selection_to_its_reeve() {
    let mut listing = listing();
    let mut state = reeve_state(listing.clone());
    compose(&mut state);
    state.handle_input_bytes(b"\x02w\x1b[B\x1b[B");
    assert_eq!(
        state.navigate_reeve_key,
        Some(ReeveTreeKey::Hand("ansel-review".into()))
    );
    // The hand reports done mid-walk: its row goes while its pane is open (decision 73), and
    // the selection moves to its reeve's row.
    errand_mut(&mut listing, "ansel-review").hand_state =
        crate::client::reeve_listing::HandState::Done;
    state.set_reeve_listing(Ok(listing));
    let frame = compose(&mut state);
    assert_eq!(tree_rows(&frame)[4], " · bertil");
    assert_eq!(
        state.navigate_reeve_key,
        Some(ReeveTreeKey::Reeve("ansel".into()))
    );
    let (rect, _) = state
        .hits
        .agents
        .iter()
        .find(|(_, pane)| pane == "p_a0")
        .cloned()
        .expect("reeve row hit");
    let cell = usize::from(rect.y) * usize::from(frame.width) + 2;
    assert_eq!(
        frame.cells[cell].bg,
        crate::protocol::color_to_u32(state.config.palette.selection_bg),
        "the reeve row is highlighted"
    );
    let pick = state.handle_input_bytes(b"\r");
    assert!(matches!(
        picked_method(&pick),
        Some(crate::api::schema::Method::PaneFocus(target)) if target.pane_id == "p_a0"
    ));
}

#[test]
fn a_selected_working_hand_whose_pane_closes_passes_the_selection_to_its_reeve() {
    let mut state = reeve_state(listing());
    compose(&mut state);
    state.handle_input_bytes(b"\x02w\x1b[B\x1b[B");
    assert_eq!(
        state.navigate_reeve_key,
        Some(ReeveTreeKey::Hand("ansel-review".into()))
    );
    // Its pane closes mid-walk with the hand still working: the listing does not change, so
    // no read comes (the reader sends only changed ones), and the snapshot alone moves the
    // selection to its reeve's row.
    let mut snapshot = hall_dev_snapshot();
    snapshot.tabs.retain(|tab| tab.tab_id != "ws_a_t2");
    snapshot.panes.retain(|pane| pane.tab_id != "ws_a_t2");
    state.set_snapshot(Box::new(snapshot));
    assert_eq!(
        state.navigate_reeve_key,
        Some(ReeveTreeKey::Reeve("ansel".into()))
    );
    let frame = compose(&mut state);
    assert!(!tree_rows(&frame)
        .iter()
        .any(|row| row.contains("ansel-review")));
    let (rect, _) = state
        .hits
        .agents
        .iter()
        .find(|(_, pane)| pane == "p_a0")
        .cloned()
        .expect("reeve row hit");
    let cell = usize::from(rect.y) * usize::from(frame.width) + 2;
    assert_eq!(
        frame.cells[cell].bg,
        crate::protocol::color_to_u32(state.config.palette.selection_bg),
        "the reeve row is highlighted"
    );
    let pick = state.handle_input_bytes(b"\r");
    assert!(matches!(
        picked_method(&pick),
        Some(crate::api::schema::Method::PaneFocus(target)) if target.pane_id == "p_a0"
    ));
}

#[test]
fn a_snapshot_after_esc_leaves_the_ended_walks_selection_alone() {
    let mut state = reeve_state(listing());
    compose(&mut state);
    state.handle_input_bytes(b"\x02w\x1b[B\x1b[B");
    state.handle_input_bytes(b"\x1b");
    assert_eq!(state.mode, ClientShellMode::Terminal);
    // The walk is over, so a snapshot that closes the hand's pane passes nothing on: the key
    // left behind is only overwritten when the next walk starts.
    let mut snapshot = hall_dev_snapshot();
    snapshot.tabs.retain(|tab| tab.tab_id != "ws_a_t2");
    snapshot.panes.retain(|pane| pane.tab_id != "ws_a_t2");
    state.set_snapshot(Box::new(snapshot));
    assert_eq!(
        state.navigate_reeve_key,
        Some(ReeveTreeKey::Hand("ansel-review".into()))
    );
    state.handle_input_bytes(b"\x02w");
    assert_eq!(state.mode, ClientShellMode::Navigate);
    assert_ne!(
        state.navigate_reeve_key,
        Some(ReeveTreeKey::Hand("ansel-review".into()))
    );
}

#[test]
fn a_selected_tab_that_the_listing_names_a_hand_keeps_the_selection() {
    let full = listing();
    let mut listing = full.clone();
    listing.errands.retain(|errand| errand.id != "ansel-review");
    // The hand's tab is open before the listing names the errand: a tab row of its reeve.
    let mut state = reeve_state(listing);
    compose(&mut state);
    state.handle_input_bytes(b"\x02w\x1b[B\x1b[B");
    assert_eq!(
        state.navigate_reeve_key,
        Some(ReeveTreeKey::Tab("ws_a_t2".into()))
    );
    // The next read lists it: the row becomes the hand's, and the selection follows.
    state.set_reeve_listing(Ok(full));
    let frame = compose(&mut state);
    assert_eq!(tree_rows(&frame)[4], "  └ ● ansel-review");
    assert_eq!(
        state.navigate_reeve_key,
        Some(ReeveTreeKey::Hand("ansel-review".into()))
    );
    let pick = state.handle_input_bytes(b"\r");
    assert!(matches!(
        picked_method(&pick),
        Some(crate::api::schema::Method::PaneFocus(target)) if target.pane_id == "p_a2"
    ));
}

#[test]
fn enter_on_a_finished_hands_tab_that_is_gone_from_the_tree_does_nothing() {
    let mut listing = listing();
    let mut state = reeve_state(listing.clone());
    compose(&mut state);
    state.handle_input_bytes(b"\x02w\x1b[B\x1b[B");
    // Finished and cleaned up: the listing drops it, so the selection follows its tab, left
    // as a tab row (decision 55), and then that tab is closed too.
    listing.errands.retain(|errand| errand.id != "ansel-review");
    state.set_reeve_listing(Ok(listing));
    assert_eq!(
        state.navigate_reeve_key,
        Some(ReeveTreeKey::Tab("ws_a_t2".into()))
    );
    let mut snapshot = hall_dev_snapshot();
    snapshot.tabs.retain(|tab| tab.tab_id != "ws_a_t2");
    snapshot.panes.retain(|pane| pane.pane_id != "p_a2");
    state.set_snapshot(Box::new(snapshot));
    compose(&mut state);
    assert_eq!(
        state.navigate_reeve_key,
        Some(ReeveTreeKey::Tab("ws_a_t2".into()))
    );
    let pick = state.handle_input_bytes(b"\r");
    assert!(
        pick.actions.is_empty(),
        "no focus of the reeve's workspace or pane: {:?}",
        pick.actions
    );
    assert_eq!(state.mode, ClientShellMode::Terminal);
}

#[test]
fn with_several_machines_no_key_help_row_is_kept() {
    use crate::client::endpoint::{ClientEndpointId, ClientEndpointStatus, ProfileId};
    let mut state = reeve_state(listing());
    let profile = crate::client::endpoint::SavedSshEndpoint {
        id: ProfileId::parse("0123456789abcdef0123456789abcdef").unwrap(),
        label: "Build".into(),
        target: "dev@build.example".into(),
        session: "agents".into(),
        enabled: true,
    };
    let endpoint_id = ClientEndpointId::Ssh(profile.id.clone());
    state.set_endpoint_catalog(&[profile]);
    state.set_endpoint_status(&endpoint_id, ClientEndpointStatus::Online);
    assert!(
        state.current_reeve_tree().is_none(),
        "herdr's machines view"
    );
    assert!(state.layout(100, 30).key_help.is_empty());
    assert_eq!(state.layout(100, 30).sidebar.height, 30);
    let frame = compose(&mut state);
    assert!(!rows(&frame).iter().any(|row| row.contains("reeve tree")));
}

#[test]
fn a_config_reload_applies_the_reeve_section_live() {
    let _guard = crate::config::test_config_env_lock().lock().unwrap();
    let base = std::env::temp_dir().join(format!(
        "hall-reeve-reload-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(&base).expect("create reload test dir");
    let path = base.join("hall.toml");
    let saved = std::env::var_os("HALL_CONFIG_PATH");
    std::env::set_var("HALL_CONFIG_PATH", &path);

    let mut state = reeve_state(listing());
    // A new timeout for the same command keeps the last good tree.
    std::fs::write(
        &path,
        "[reeve]\nlisting_command = [\"true\"]\nlisting_timeout_ms = 3500\n",
    )
    .expect("write config");
    state.reload_client_config();
    let spec = state.reeve_listing_spec().expect("listing command");
    assert_eq!(spec.timeout, std::time::Duration::from_millis(3500));
    assert_eq!(state.reeve_listing, Some(listing()));
    // The reload takes the session from `HALL_SESSION`; this test's tree is hall-dev's.
    state.config.reeve_tree_session = Some("hall-dev".into());
    assert_eq!(tree_rows(&compose(&mut state))[2], " · ansel");

    std::fs::write(
        &path,
        "[reeve]\nlisting_command = [\"other-listing\", \"--json\"]\nlisting_timeout_ms = 2000\n",
    )
    .expect("write config");
    state.reload_client_config();
    let spec = state.reeve_listing_spec().expect("listing command");
    assert_eq!(spec.command, ["other-listing", "--json"]);
    assert_eq!(spec.timeout, std::time::Duration::from_secs(2));
    // A read of the old command is not the new one's.
    assert_eq!(state.reeve_listing, None);
    assert_eq!(tree_rows(&compose(&mut state))[2], " reading listing");

    std::fs::write(&path, "[reeve]\n").expect("write config");
    state.reload_client_config();
    assert!(state.reeve_listing_spec().is_none());
    assert!(state.current_reeve_tree().is_none());
    assert!(state.layout(100, 30).key_help.is_empty());
    let frame = compose(&mut state);
    assert_eq!(tree_rows(&frame)[0], " spaces");
    assert!(!rows(&frame).iter().any(|row| row.contains("reeve tree")));

    match saved {
        Some(value) => std::env::set_var("HALL_CONFIG_PATH", value),
        None => std::env::remove_var("HALL_CONFIG_PATH"),
    }
    std::fs::remove_dir_all(&base).expect("remove reload test dir");
}
