//! hall: the reeve tree in the sidebar (plan step 3, decisions 12, 16, 17, 34, 49 to 56, 70,
//! 73; the alert highlight of plan step 4, decisions 62 and 63).
//!
//! Every reeve the listing places in this hall session sits at the top level, its hands still
//! working beneath it (reeve = workspace, hand = tab), a mark per hand and a badge per reeve,
//! then the reeve's other tabs, dimmed. Workspaces that hold no reeve follow in a section of
//! their own. The tree is built from the server's snapshot and reeve's listing on every frame;
//! nothing of it is kept but the selection.

use super::reeve_alerts::ReeveAlerts;
use super::*;
use crate::client::reeve_listing::{HandState, ListedErrand, ReeveListing, ReeveListingRead};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ReeveTreeKey {
    Reeve(String),
    Hand(String),
    /// A tab of a reeve's workspace that holds neither the reeve nor a listed hand.
    Tab(String),
    Workspace(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ReeveTreeItemKind {
    Reeve {
        badge: usize,
        live: bool,
        status: crate::api::schema::AgentStatus,
    },
    Hand {
        state: HandState,
        notes: Vec<&'static str>,
        last: bool,
    },
    Tab {
        last: bool,
    },
    Workspace {
        status: crate::api::schema::AgentStatus,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ReeveTreeItem {
    pub(super) key: ReeveTreeKey,
    pub(super) kind: ReeveTreeItemKind,
    pub(super) label: String,
    /// The pane picking the item shows: the reeve's own when hall has it, a hand's always.
    pub(super) pane_id: Option<String>,
    pub(super) workspace_id: Option<String>,
    pub(super) focused: bool,
    /// An alert stands for this reeve (its own pane or any of its hands) or this hand
    /// (decision 63).
    pub(super) alert: bool,
}

/// What the tree says about the listing itself, on rows of its own above the reeves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ListingNote {
    /// No read has come back yet.
    Reading,
    /// The listing read well and places no reeve in this session.
    NoReeve,
    /// The latest read failed and none has read well yet: every workspace sits under `other`.
    Failed(String),
    /// The latest read failed; the tree is the last good one (decision 56).
    Stale(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct ReeveTree {
    pub(super) items: Vec<ReeveTreeItem>,
    pub(super) note: Option<ListingNote>,
}

impl ReeveTree {
    pub(super) fn position(&self, key: &ReeveTreeKey) -> Option<usize> {
        self.items.iter().position(|item| &item.key == key)
    }

    fn focused_position(&self) -> Option<usize> {
        self.items.iter().position(|item| item.focused)
    }
}

fn hand_notes(errand: &ListedErrand) -> Vec<&'static str> {
    let mut notes = Vec::new();
    match errand.hand_state {
        // A done or failed hand gets no row, so no notes (decision 73).
        HandState::Working | HandState::Unknown | HandState::Done | HandState::Failed => {}
        HandState::NeedsDecision => notes.push("decide"),
        HandState::Blocked => notes.push("blocked"),
    }
    if errand
        .question
        .as_deref()
        .is_some_and(|question| !question.is_empty())
    {
        notes.push("asks");
    }
    for (flag, note) in [
        (errand.waiting, "waiting"),
        (errand.idle, "idle"),
        (errand.wedged, "wedged"),
    ] {
        if flag {
            notes.push(note);
        }
    }
    notes
}

/// A hand's row names the hand as reeve names its worktree, its office before its errand
/// (`scout-mend-lead-finder`), or the errand alone when the listing names no office or only
/// blanks (decision 71).
fn hand_label(errand: &ListedErrand) -> String {
    let office = errand.office.trim();
    if office.is_empty() {
        errand.id.clone()
    } else {
        format!("{office}-{}", errand.id)
    }
}

/// The hand's pane in `snapshot`, when hall has it. Only a hand with one is active and gets a
/// row; one without (a hand whose pane closed before its teardown, say) gets none, though what
/// it waits on still counts on its reeve's row (decision 70).
fn hand_pane<'a>(
    snapshot: &'a ClientShellSnapshot,
    errand: &ListedErrand,
) -> Option<&'a crate::protocol::ClientShellPane> {
    let pane_id = errand.session.pane_id.as_ref()?;
    snapshot.panes.iter().find(|pane| &pane.pane_id == pane_id)
}

/// Builds the tree for `session` from the snapshot, the last listing that read well and why
/// the latest read failed, if it did.
pub(super) fn reeve_tree(
    snapshot: &ClientShellSnapshot,
    listing: Option<&ReeveListing>,
    listing_error: Option<&str>,
    session: &str,
    alerts: &ReeveAlerts,
) -> ReeveTree {
    let pane = |pane_id: Option<&String>| {
        pane_id.and_then(|pane_id| snapshot.panes.iter().find(|pane| &pane.pane_id == pane_id))
    };
    let mut tree = ReeveTree {
        note: match (listing, listing_error) {
            (None, None) => Some(ListingNote::Reading),
            (None, Some(error)) => Some(ListingNote::Failed(error.to_owned())),
            (Some(_), Some(error)) => Some(ListingNote::Stale(error.to_owned())),
            (Some(_), None) => None,
        },
        ..ReeveTree::default()
    };
    let mut reeve_workspaces = HashSet::new();
    for reeve in listing
        .iter()
        .flat_map(|listing| &listing.reeves)
        .filter(|reeve| reeve.session.in_session(session))
    {
        let reeve_pane = pane(reeve.session.pane_id.as_ref());
        let workspace_id = reeve_pane.map(|pane| pane.workspace_id.clone());
        if let Some(workspace_id) = &workspace_id {
            reeve_workspaces.insert(workspace_id.clone());
        }
        let listed = listing
            .iter()
            .flat_map(|listing| &listing.errands)
            .filter(|errand| errand.owner_reeve == reeve.name && errand.session.in_session(session))
            .collect::<Vec<_>>();
        let listed_panes = listed
            .iter()
            .filter_map(|errand| Some((*errand, hand_pane(snapshot, errand)?)))
            .collect::<Vec<_>>();
        // Only a hand still working gets a row: a done or failed one shows none, even while its
        // pane is open (decision 73).
        let hands = listed_panes
            .iter()
            .copied()
            .filter(|(errand, _)| !matches!(errand.hand_state, HandState::Done | HandState::Failed))
            .collect::<Vec<_>>();
        let hand_tabs = hands
            .iter()
            .map(|(_, pane)| pane.tab_id.as_str())
            .collect::<HashSet<_>>();
        // Every listed hand with a pane holds its tab, a row or not, so a done or failed hand's
        // tab shows as no `tab` row of the reeve's (decisions 55, 73).
        let listed_tabs = listed_panes
            .iter()
            .map(|(_, pane)| pane.tab_id.as_str())
            .collect::<HashSet<_>>();
        // The reeve's workspace's other tabs: neither the reeve's own nor a listed hand's
        // (decision 55).
        let other_tabs = reeve_pane
            .map(|reeve_pane| {
                snapshot
                    .tabs
                    .iter()
                    .filter(|tab| {
                        tab.workspace_id == reeve_pane.workspace_id
                            && tab.tab_id != reeve_pane.tab_id
                            && !listed_tabs.contains(tab.tab_id.as_str())
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let status = reeve_pane
            .and_then(|reeve_pane| {
                snapshot
                    .agents
                    .iter()
                    .find(|agent| agent.pane_id == reeve_pane.pane_id)
            })
            .map_or(crate::api::schema::AgentStatus::Unknown, |agent| {
                agent.agent_status
            });
        let hand_alert = |errand: &ListedErrand| {
            alerts.hand(
                &errand.id,
                hand_pane(snapshot, errand).map(|pane| pane.pane_id.as_str()),
            )
        };
        tree.items.push(ReeveTreeItem {
            alert: alerts.pane(reeve_pane.map(|pane| pane.pane_id.as_str()))
                // Every listed hand counts here, a row or not, so an alert from a hand with no
                // pane still marks its reeve (decisions 63, 70).
                || listed.iter().any(|errand| hand_alert(errand)),
            key: ReeveTreeKey::Reeve(reeve.name.clone()),
            kind: ReeveTreeItemKind::Reeve {
                badge: listed
                    .iter()
                    .filter(|errand| errand.waits_on_liege())
                    .count(),
                live: reeve.live,
                status,
            },
            label: reeve.name.clone(),
            pane_id: reeve_pane.map(|pane| pane.pane_id.clone()),
            focused: workspace_id.is_some()
                && snapshot.focused_workspace_id == workspace_id
                && !snapshot.focused_tab_id.as_deref().is_some_and(|tab_id| {
                    hand_tabs.contains(tab_id) || other_tabs.iter().any(|tab| tab.tab_id == tab_id)
                }),
            workspace_id,
        });
        for (index, (errand, hand_pane)) in hands.iter().enumerate() {
            tree.items.push(ReeveTreeItem {
                alert: hand_alert(errand),
                key: ReeveTreeKey::Hand(errand.id.clone()),
                kind: ReeveTreeItemKind::Hand {
                    state: errand.hand_state,
                    notes: hand_notes(errand),
                    last: index + 1 == hands.len() && other_tabs.is_empty(),
                },
                label: hand_label(errand),
                pane_id: Some(hand_pane.pane_id.clone()),
                workspace_id: Some(hand_pane.workspace_id.clone()),
                focused: snapshot.focused_tab_id.as_deref() == Some(hand_pane.tab_id.as_str()),
            });
        }
        for (index, tab) in other_tabs.iter().enumerate() {
            tree.items.push(ReeveTreeItem {
                key: ReeveTreeKey::Tab(tab.tab_id.clone()),
                kind: ReeveTreeItemKind::Tab {
                    last: index + 1 == other_tabs.len(),
                },
                label: format!("tab {}", tab.label),
                pane_id: None,
                workspace_id: Some(tab.workspace_id.clone()),
                focused: snapshot.focused_tab_id.as_deref() == Some(tab.tab_id.as_str()),
                alert: false,
            });
        }
    }
    if tree.note.is_none()
        && !tree
            .items
            .iter()
            .any(|item| matches!(item.kind, ReeveTreeItemKind::Reeve { .. }))
    {
        tree.note = Some(ListingNote::NoReeve);
    }
    for workspace in snapshot
        .workspaces
        .iter()
        .filter(|workspace| !reeve_workspaces.contains(&workspace.workspace_id))
    {
        tree.items.push(ReeveTreeItem {
            key: ReeveTreeKey::Workspace(workspace.workspace_id.clone()),
            kind: ReeveTreeItemKind::Workspace {
                status: workspace.agent_status,
            },
            label: workspace.label.clone(),
            pane_id: None,
            workspace_id: Some(workspace.workspace_id.clone()),
            focused: workspace.focused,
            alert: false,
        });
    }
    tree
}

fn text_width(text: &str) -> usize {
    unicode_width::UnicodeWidthStr::width(text)
}

/// The notes, in order and whole, that fit `width` together with a space between each.
fn fit_notes(notes: &[String], width: usize) -> String {
    let mut fitted = String::new();
    for note in notes {
        let next = if fitted.is_empty() {
            note.clone()
        } else {
            format!("{fitted} {note}")
        };
        if text_width(&next) > width {
            break;
        }
        fitted = next;
    }
    fitted
}

/// The notes of a hand row that fit `width`: all of them when they fit. When they do not, each
/// is kept whole by how much it tells beyond the mark, `wedged` first and the words the mark
/// already says (the state word, and `asks` beside a `?`) last, skipping one that does not fit;
/// the ones kept read in decision 52's order (decision 59).
fn fit_hand_notes(state: HandState, notes: &[&str], width: usize) -> String {
    let all = notes.join(" ");
    if text_width(&all) <= width {
        return all;
    }
    let rank = |note: &str| {
        let repeats_mark = matches!(note, "decide" | "blocked")
            || (note == "asks" && state == HandState::NeedsDecision);
        if repeats_mark {
            usize::MAX
        } else {
            ["wedged", "asks", "waiting", "idle"]
                .iter()
                .position(|word| *word == note)
                .unwrap_or(usize::MAX - 1)
        }
    };
    let mut order = (0..notes.len()).collect::<Vec<_>>();
    order.sort_by_key(|&index| rank(notes[index]));
    let mut kept = vec![false; notes.len()];
    let mut used = 0;
    for index in order {
        let needed = text_width(notes[index]) + usize::from(used > 0);
        if used + needed <= width {
            kept[index] = true;
            used += needed;
        }
    }
    notes
        .iter()
        .zip(kept)
        .filter_map(|(note, kept)| kept.then_some(*note))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Wraps `text` at spaces to rows of at most `width` columns, breaking a longer word.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut rows = Vec::new();
    let mut row = String::new();
    for word in text.split_whitespace() {
        let mut pieces = vec![String::new()];
        for character in word.chars() {
            let piece = pieces.last_mut().expect("a piece");
            if !piece.is_empty() && text_width(&format!("{piece}{character}")) > width {
                pieces.push(character.to_string());
            } else {
                piece.push(character);
            }
        }
        for piece in pieces {
            let joined = if row.is_empty() {
                piece
            } else if text_width(&row) + 1 + text_width(&piece) <= width {
                format!("{row} {piece}")
            } else {
                rows.push(std::mem::take(&mut row));
                piece
            };
            row = joined;
        }
    }
    if !row.is_empty() {
        rows.push(row);
    }
    rows
}

fn hand_mark(state: HandState, palette: &Palette) -> (&'static str, ratatui::style::Color) {
    match state {
        HandState::Working => ("●", palette.yellow),
        HandState::NeedsDecision => ("?", palette.mauve),
        HandState::Blocked => ("!", palette.red),
        // A done or failed hand gets no row to mark (decision 73).
        HandState::Unknown | HandState::Done | HandState::Failed => ("·", palette.overlay0),
    }
}

enum TreeLine<'a> {
    Note(String, ratatui::style::Color),
    Gap,
    Header(&'static str),
    Item(&'a ReeveTreeItem),
}

/// Draws the reeve tree where herdr draws its spaces: a `The Great Hall` header over a dim
/// `Reeves in attendance` line, the tree, then the workspaces that hold no reeve under `other`.
/// The section's last row stays herdr's footer.
pub(super) fn render_reeve_section(
    buffer: &mut Buffer,
    area: Rect,
    tree: &ReeveTree,
    config: &ClientShellConfig,
    selected: Option<&ReeveTreeKey>,
    scroll: &mut usize,
    hits: &mut ShellHitMap,
) {
    let palette = &config.palette;
    render::put_text(
        buffer,
        area.x,
        area.y,
        area.width,
        &crate::ui::truncate_end(" The Great Hall", usize::from(area.width)),
        Style::default()
            .fg(palette.overlay0)
            .add_modifier(Modifier::BOLD),
    );
    // The heading's second row, drawn only when a tree row is left for it to sit over: a
    // section of 3 rows keeps its one tree row instead (the last row is always the footer).
    let described = area.height > 3;
    if described {
        render::put_text(
            buffer,
            area.x,
            area.y.saturating_add(1),
            area.width,
            &crate::ui::truncate_end(" Reeves in attendance", usize::from(area.width)),
            Style::default().fg(palette.overlay0),
        );
    }
    let mut lines = Vec::new();
    if let Some(note) = &tree.note {
        let (text, style) = match note {
            ListingNote::Reading => ("reading listing".to_owned(), palette.overlay0),
            ListingNote::NoReeve => ("no reeve in this session".to_owned(), palette.overlay0),
            ListingNote::Failed(error) => (format!("listing: {error}"), palette.yellow),
            ListingNote::Stale(error) => (format!("stale: {error}"), palette.yellow),
        };
        // A reason is wrapped onto rows of its own, never cut, so it reads at any width.
        lines.extend(
            wrap(&text, usize::from(area.width.saturating_sub(1)))
                .into_iter()
                .map(|line| TreeLine::Note(line, style)),
        );
    }
    for item in &tree.items {
        if matches!(item.kind, ReeveTreeItemKind::Workspace { .. })
            && !lines.iter().any(|line| matches!(line, TreeLine::Header(_)))
        {
            lines.push(TreeLine::Gap);
            lines.push(TreeLine::Header(" other"));
        }
        lines.push(TreeLine::Item(item));
    }

    let heading_rows = if described { 2 } else { 1 };
    let body = Rect::new(
        area.x,
        area.y.saturating_add(heading_rows),
        area.width,
        area.height.saturating_sub(heading_rows + 1),
    );
    hits.workspace_body = body;
    let height = usize::from(body.height);
    let max_scroll = lines.len().saturating_sub(height);
    if let Some(line) = selected.and_then(|key| {
        lines
            .iter()
            .position(|line| matches!(line, TreeLine::Item(item) if &item.key == key))
    }) {
        if line < *scroll {
            *scroll = line;
        } else if height > 0 && line >= *scroll + height {
            *scroll = line + 1 - height;
        }
    }
    *scroll = (*scroll).min(max_scroll);
    hits.workspace_max_scroll = max_scroll;
    hits.workspace_scroll_metrics = None;

    for (row, line) in lines.iter().skip(*scroll).take(height).enumerate() {
        let rect = Rect::new(body.x, body.y + row as u16, body.width, 1);
        match line {
            TreeLine::Note(text, color) => render::put_text(
                buffer,
                rect.x + 1,
                rect.y,
                rect.width.saturating_sub(1),
                text,
                Style::default().fg(*color),
            ),
            TreeLine::Gap => {}
            TreeLine::Header(text) => render::put_text(
                buffer,
                rect.x,
                rect.y,
                rect.width,
                text,
                Style::default()
                    .fg(palette.overlay0)
                    .add_modifier(Modifier::BOLD),
            ),
            TreeLine::Item(item) => {
                render_item(buffer, rect, item, selected == Some(&item.key), config);
                if let Some(pane_id) = &item.pane_id {
                    hits.agents.push((rect, pane_id.clone()));
                } else if let ReeveTreeKey::Tab(tab_id) = &item.key {
                    hits.reeve_tabs.push((rect, tab_id.clone()));
                } else if let Some(workspace_id) = item
                    .workspace_id
                    .as_ref()
                    .filter(|_| matches!(item.kind, ReeveTreeItemKind::Workspace { .. }))
                {
                    hits.workspaces.push(WorkspaceHit {
                        rect,
                        endpoint_id: ClientEndpointId::Local,
                        workspace_id: workspace_id.clone(),
                        indented: false,
                        group_toggle: None,
                    });
                }
            }
        }
    }
}

/// The mark in a row's first column while an alert stands for it (decision 63).
pub(super) const ALERT_BAR: &str = "▌";

fn render_item(
    buffer: &mut Buffer,
    rect: Rect,
    item: &ReeveTreeItem,
    selected: bool,
    config: &ClientShellConfig,
) {
    let palette = &config.palette;
    let background = if selected {
        Some(palette.selection_bg)
    } else if item.focused {
        Some(palette.active_row_bg)
    } else {
        None
    };
    if let Some(background) = background {
        buffer.set_style(rect, Style::default().bg(background));
    }
    let label_style = Style::default()
        .fg(if item.focused {
            palette.text
        } else {
            palette.subtext0
        })
        .add_modifier(if item.focused {
            Modifier::BOLD
        } else {
            Modifier::empty()
        });
    let right = rect.right();
    let (prefix, mark, label_style, notes, note_style) = match &item.kind {
        ReeveTreeItemKind::Reeve {
            badge,
            live,
            status,
        } => {
            let note = if !live {
                "gone".to_owned()
            } else if item.pane_id.is_none() {
                "no pane".to_owned()
            } else if *badge > 0 {
                format!(" {badge} ")
            } else {
                String::new()
            };
            let notes = if note.is_empty() {
                Vec::new()
            } else {
                vec![note]
            };
            let note_style = if *live && item.pane_id.is_some() {
                Style::default()
                    .fg(panel_contrast_fg(palette))
                    .bg(palette.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette.overlay0)
            };
            let label_style = if *live && item.pane_id.is_some() {
                label_style.add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette.overlay0)
            };
            (
                " ",
                (
                    status_icon(*status, config.status_indicators),
                    status_color(*status, palette),
                ),
                label_style,
                notes,
                note_style,
            )
        }
        ReeveTreeItemKind::Hand { state, last, .. } => {
            let mark = hand_mark(*state, palette);
            (
                if *last { "  └ " } else { "  ├ " },
                mark,
                label_style,
                // Fitted by `fit_hand_notes` below.
                Vec::new(),
                Style::default().fg(mark.1),
            )
        }
        ReeveTreeItemKind::Tab { last } => (
            if *last { "  └ " } else { "  ├ " },
            ("", palette.overlay0),
            Style::default().fg(palette.overlay0),
            Vec::new(),
            Style::default(),
        ),
        ReeveTreeItemKind::Workspace { status } => (
            " ",
            (
                status_icon(*status, config.status_indicators),
                status_color(*status, palette),
            ),
            label_style,
            Vec::new(),
            Style::default(),
        ),
    };
    // An alert marks the row with a bar in its first column and its label in the alert
    // colour (decision 63), whatever the sound does.
    let alert_style = Style::default()
        .fg(palette.peach)
        .add_modifier(Modifier::BOLD);
    let label_style = if item.alert { alert_style } else { label_style };
    let mut x = render::put_segment(
        buffer,
        rect.x,
        rect.y,
        right,
        prefix,
        Style::default().fg(palette.overlay0),
    );
    if item.alert {
        render::put_segment(buffer, rect.x, rect.y, right, ALERT_BAR, alert_style);
    }
    if !mark.0.is_empty() {
        x = render::put_segment(
            buffer,
            x,
            rect.y,
            right,
            mark.0,
            Style::default().fg(mark.1),
        );
        x = x.saturating_add(1);
    }
    // The label (a reeve's name, a hand's office and id) comes first: the notes get what is
    // left after it, whole notes, and the ones that do not fit are dropped (decisions 52, 59).
    let width = usize::from(right.saturating_sub(x));
    let label_width = text_width(&item.label);
    let room = width.saturating_sub(label_width + 1);
    let note = match &item.kind {
        ReeveTreeItemKind::Hand { state, notes, .. } => fit_hand_notes(*state, notes, room),
        _ => fit_notes(&notes, room),
    };
    render::put_text(
        buffer,
        x,
        rect.y,
        width as u16,
        &crate::ui::truncate_end(&item.label, width),
        label_style,
    );
    if !note.is_empty() {
        let note_width = render::display_width(&note);
        render::put_text(
            buffer,
            right.saturating_sub(note_width),
            rect.y,
            note_width,
            &note,
            note_style,
        );
    }
    if let Some(background) = background {
        for x in rect.x..right {
            buffer[(x, rect.y)].set_bg(background);
        }
    }
}

/// The one line of key help at the bottom (decision 12): how to reach and walk the tree, or,
/// while walking it, how to move, pick and leave.
pub(super) fn render_key_help(
    buffer: &mut Buffer,
    area: Rect,
    mode: ClientShellMode,
    keybinds: &LiveKeybindConfig,
    palette: &Palette,
) {
    if area.is_empty() {
        return;
    }
    let base = Style::default().fg(palette.overlay0).bg(palette.panel_bg);
    let key = Style::default()
        .fg(palette.accent)
        .bg(palette.panel_bg)
        .add_modifier(Modifier::BOLD);
    buffer.set_style(area, base);
    for x in area.x..area.right() {
        buffer[(x, area.y)].set_symbol(" ");
    }
    let prefix = crate::config::format_key_combo(keybinds.prefix);
    let prefixed = |bindings: &crate::config::ActionKeybinds| {
        bindings
            .prefix_rhs_label()
            .map_or_else(|| "unset".to_owned(), |rhs| format!("{prefix} {rhs}"))
    };
    // The walk moves on the navigate keys as bound; enter and esc are fixed in herdr's walk.
    let walk_keys = |bindings: &crate::config::ActionKeybinds| {
        let labels = bindings
            .labels()
            .iter()
            .map(|label| match label.as_str() {
                "up" => "↑".to_owned(),
                "down" => "↓".to_owned(),
                label => label.to_owned(),
            })
            .collect::<Vec<_>>();
        if labels.is_empty() {
            "unset".to_owned()
        } else {
            labels.join(",")
        }
    };
    let segments = if mode == ClientShellMode::Navigate {
        vec![
            (
                format!(
                    "{}/{}",
                    walk_keys(&keybinds.keybinds.navigate.workspace_up),
                    walk_keys(&keybinds.keybinds.navigate.workspace_down)
                ),
                " move  ",
            ),
            ("enter".to_owned(), " show  "),
            ("esc".to_owned(), " back"),
        ]
    } else {
        vec![
            (
                prefixed(&keybinds.keybinds.workspace_picker),
                " reeve tree  ",
            ),
            (prefixed(&keybinds.keybinds.help), " keys  "),
            (prefixed(&keybinds.keybinds.detach), " detach"),
        ]
    };
    let mut x = area.x.saturating_add(1);
    for (keys, label) in segments {
        x = render::put_segment(buffer, x, area.y, area.right(), &keys, key);
        x = render::put_segment(buffer, x, area.y, area.right(), label, base);
    }
}

impl ClientShellState {
    /// The reeve tree as it stands, when the reeve tree is on and drawn (not in the mobile
    /// layout, which keeps herdr's own switcher).
    pub(super) fn current_reeve_tree(&self) -> Option<ReeveTree> {
        let session = self.config.reeve_tree_session.as_deref()?;
        if self.mobile_layout_active() || self.endpoints.len() > 1 {
            return None;
        }
        let snapshot = self.snapshot.as_deref()?;
        Some(reeve_tree(
            snapshot,
            self.reeve_listing.as_ref(),
            self.reeve_listing_error.as_deref(),
            session,
            &self.reeve_alerts,
        ))
    }

    /// The listing command the client should be running, from the config as last (re)loaded.
    pub(in crate::client) fn reeve_listing_spec(
        &self,
    ) -> Option<&crate::client::reeve_listing::ReeveListingSpec> {
        self.config.reeve_listing.as_ref()
    }

    /// Takes a read: a good one replaces the tree, a failed one leaves the last good tree,
    /// marked stale with the reason (decision 56). A hand's row the read takes away while the
    /// listing still names the hand (reported done or failed, decision 73, or its pane closed)
    /// passes the selection to its reeve's row. Otherwise a row selected in the walk that the
    /// read turns into another kind of row for the same tab keeps the selection: a hand the
    /// read drops leaves its tab behind as a tab row (decision 55), and a tab row the read lists
    /// as a hand becomes that hand's row (decision 61). A good read also raises the alerts of
    /// the hands it shows newly stuck or asking, and gives the sound to play for them (decision
    /// 62).
    pub(in crate::client) fn set_reeve_listing(
        &mut self,
        read: ReeveListingRead,
    ) -> Vec<ClientShellNotificationEffect> {
        let selected_tab = self.selected_row_tab();
        let effects = match read {
            Ok(listing) => {
                self.reeve_listing = Some(listing);
                self.reeve_listing_error = None;
                self.read_listing_alerts()
            }
            Err(error) => {
                self.reeve_listing_error = Some(error);
                Vec::new()
            }
        };
        if let (Some(key), Some(tree)) =
            (self.navigate_reeve_key.as_ref(), self.current_reeve_tree())
        {
            if tree.position(key).is_none() {
                if let Some(item) = self.hand_reeve_row(&tree, key).or_else(|| {
                    selected_tab.and_then(|tab_id| self.tree_row_for_tab(&tree, &tab_id))
                }) {
                    self.navigate_reeve_key = Some(item.key.clone());
                }
            }
        }
        effects
    }

    /// The row in `tree` of the reeve the listing names as the owner of the hand `key` names,
    /// when the listing still names the hand in this session.
    fn hand_reeve_row<'a>(
        &self,
        tree: &'a ReeveTree,
        key: &ReeveTreeKey,
    ) -> Option<&'a ReeveTreeItem> {
        let ReeveTreeKey::Hand(id) = key else {
            return None;
        };
        let session = self.config.reeve_tree_session.as_deref()?;
        let errand = self
            .reeve_listing
            .as_ref()?
            .errands
            .iter()
            .find(|errand| &errand.id == id && errand.session.in_session(session))?;
        let index = tree.position(&ReeveTreeKey::Reeve(errand.owner_reeve.clone()))?;
        tree.items.get(index)
    }

    /// The tab a hand's or a tab's row selected in the walk stands for, if one is selected.
    fn selected_row_tab(&self) -> Option<String> {
        let key = self.navigate_reeve_key.as_ref()?;
        match key {
            ReeveTreeKey::Tab(tab_id) => Some(tab_id.clone()),
            ReeveTreeKey::Hand(_) => {
                let tree = self.current_reeve_tree()?;
                let pane_id = tree.items.get(tree.position(key)?)?.pane_id.as_ref()?;
                self.pane_tab(pane_id)
            }
            ReeveTreeKey::Reeve(_) | ReeveTreeKey::Workspace(_) => None,
        }
    }

    /// The hand's or the tab's row that stands for `tab_id` in `tree`.
    fn tree_row_for_tab<'a>(&self, tree: &'a ReeveTree, tab_id: &str) -> Option<&'a ReeveTreeItem> {
        tree.items.iter().find(|item| match &item.key {
            ReeveTreeKey::Tab(tab) => tab == tab_id,
            ReeveTreeKey::Hand(_) => item
                .pane_id
                .as_ref()
                .and_then(|pane_id| self.pane_tab(pane_id))
                .is_some_and(|tab| tab == tab_id),
            ReeveTreeKey::Reeve(_) | ReeveTreeKey::Workspace(_) => false,
        })
    }

    fn pane_tab(&self, pane_id: &str) -> Option<String> {
        self.snapshot
            .as_deref()?
            .panes
            .iter()
            .find(|pane| pane.pane_id == pane_id)
            .map(|pane| pane.tab_id.clone())
    }

    /// Forgets every read, when the listing command changes or goes.
    pub(super) fn reset_reeve_listing(&mut self) {
        self.reeve_listing = None;
        self.reeve_listing_error = None;
        self.navigate_reeve_key = None;
        self.reset_reeve_alerts();
    }

    /// Starts a walk of the tree at the item that holds the focus.
    pub(super) fn start_reeve_tree_navigation(&mut self) {
        self.navigate_reeve_key = self.current_reeve_tree().and_then(|tree| {
            tree.focused_position()
                .map(|index| tree.items[index].key.clone())
        });
    }

    /// Moves the walk one item up or down, wrapping as herdr's workspace walk does. False when
    /// the tree is not drawn, so herdr's own walk runs instead.
    pub(super) fn move_navigate_reeve_tree(&mut self, delta: isize) -> bool {
        let Some(tree) = self.current_reeve_tree() else {
            return false;
        };
        if tree.items.is_empty() {
            return true;
        }
        let current = self
            .navigate_reeve_key
            .as_ref()
            .and_then(|key| tree.position(key))
            .or_else(|| tree.focused_position());
        let next = match current {
            Some(current) => (current as isize + delta).rem_euclid(tree.items.len() as isize),
            None if delta < 0 => tree.items.len() as isize - 1,
            None => 0,
        } as usize;
        let item = &tree.items[next];
        self.navigate_reeve_key = Some(item.key.clone());
        if let Some(workspace_id) = &item.workspace_id {
            self.navigate_workspace_id = Some(workspace_id.clone());
        }
        true
    }

    /// Picks the selected item: shows a reeve's or a hand's own pane, brings a tab forward or
    /// focuses a workspace. None when the tree is not drawn or nothing is selected, so herdr's
    /// own pick runs. A selected key the tree no longer has (a finished hand whose tab is gone
    /// too, or a hand whose reeve's row went with its own) picks nothing, rather than herdr's
    /// pick of the reeve's workspace.
    fn pick_navigate_reeve_target(&self) -> Option<Option<crate::api::schema::Method>> {
        let tree = self.current_reeve_tree()?;
        let Some(item) = tree
            .position(self.navigate_reeve_key.as_ref()?)
            .and_then(|index| tree.items.get(index))
        else {
            return Some(None);
        };
        Some(if let Some(pane_id) = &item.pane_id {
            Some(crate::api::schema::Method::PaneFocus(
                crate::api::schema::PaneTarget {
                    pane_id: pane_id.clone(),
                },
            ))
        } else if let ReeveTreeKey::Tab(tab_id) = &item.key {
            Some(crate::api::schema::Method::TabFocus(
                crate::api::schema::TabTarget {
                    tab_id: tab_id.clone(),
                },
            ))
        } else {
            item.workspace_id
                .as_ref()
                .filter(|_| matches!(item.kind, ReeveTreeItemKind::Workspace { .. }))
                .map(|workspace_id| {
                    crate::api::schema::Method::WorkspaceFocus(
                        crate::api::schema::WorkspaceTarget {
                            workspace_id: workspace_id.clone(),
                        },
                    )
                })
        })
    }

    /// Enter in the walk: picks the selected item and leaves the walk. False when the tree is
    /// not drawn, so herdr's own pick runs.
    pub(super) fn pick_navigate_reeve_tree(&mut self, outcome: &mut ClientShellInput) -> bool {
        let Some(method) = self.pick_navigate_reeve_target() else {
            return false;
        };
        self.mode = ClientShellMode::Terminal;
        self.navigate_workspace_id = None;
        self.navigate_reeve_key = None;
        if let Some(crate::api::schema::Method::PaneFocus(target)) = &method {
            // Showing a reeve's or a hand's pane clears its alert, also when it already has
            // the focus (decision 63).
            self.show_reeve_alerts(&target.pane_id);
        }
        if let Some(method) = method {
            self.push_endpoint_method(method, outcome);
        }
        outcome.repaint = true;
        true
    }

    /// A digit in the walk: picks the `index`th reeve or plain workspace in the tree's own order
    /// (its top-level rows, hands and tabs not counted), as herdr's digits pick the `index`th
    /// workspace of its list. A digit past the last does nothing. False when the tree is not
    /// drawn, so herdr's digits run.
    pub(super) fn pick_navigate_reeve_tree_number(
        &mut self,
        index: usize,
        outcome: &mut ClientShellInput,
    ) -> bool {
        let Some(tree) = self.current_reeve_tree() else {
            return false;
        };
        if let Some(item) = tree
            .items
            .iter()
            .filter(|item| {
                matches!(
                    item.kind,
                    ReeveTreeItemKind::Reeve { .. } | ReeveTreeItemKind::Workspace { .. }
                )
            })
            .nth(index)
        {
            self.navigate_reeve_key = Some(item.key.clone());
            self.pick_navigate_reeve_tree(outcome);
        }
        true
    }

    /// The workspace a workspace action (close, rename, worktree, new from) acts on while the
    /// walk has a tree row selected: a reeve's row or a plain workspace's names its workspace;
    /// a hand's row, a tab's row and a reeve with no pane name none, so the action does nothing
    /// rather than act on the reeve's whole workspace. None when no tree walk is on: herdr keeps
    /// `navigate_workspace_id` for exactly as long as a walk lasts, and its walk leaves navigate
    /// mode before it runs the action a key names, so the mode cannot tell.
    pub(super) fn reeve_walk_workspace_target(&self) -> Option<Option<String>> {
        self.navigate_workspace_id.as_ref()?;
        let key = self.navigate_reeve_key.as_ref()?;
        let tree = self.current_reeve_tree()?;
        // A key no longer in the tree (a hand that finished while selected) names no
        // workspace, rather than the reeve's that `navigate_workspace_id` still holds.
        let Some(item) = tree.position(key).and_then(|index| tree.items.get(index)) else {
            return Some(None);
        };
        Some(match item.kind {
            ReeveTreeItemKind::Reeve { .. } | ReeveTreeItemKind::Workspace { .. } => {
                item.workspace_id.clone()
            }
            ReeveTreeItemKind::Hand { .. } | ReeveTreeItemKind::Tab { .. } => None,
        })
    }
}
