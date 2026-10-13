//! hall: the reeve tree in the sidebar (plan step 3, decisions 12, 16, 17, 34, 49 to 54).
//!
//! Every reeve the listing places in this hall session sits at the top level, its hands in
//! flight beneath it (reeve = workspace, hand = tab), a mark per hand and a badge per reeve.
//! Workspaces that hold no reeve follow in a section of their own. The tree is built from the
//! server's snapshot and reeve's listing on every frame; nothing of it is kept but the
//! selection.

use super::*;
use crate::client::reeve_listing::{HandState, ListedErrand, ReeveListingRead};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ReeveTreeKey {
    Reeve(String),
    Hand(String),
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
    Workspace {
        status: crate::api::schema::AgentStatus,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ReeveTreeItem {
    pub(super) key: ReeveTreeKey,
    pub(super) kind: ReeveTreeItemKind,
    pub(super) label: String,
    /// The pane picking the item shows: the reeve's or the hand's own, when hall has it.
    pub(super) pane_id: Option<String>,
    pub(super) workspace_id: Option<String>,
    pub(super) focused: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct ReeveTree {
    pub(super) items: Vec<ReeveTreeItem>,
    pub(super) listing_error: Option<String>,
}

impl ReeveTree {
    pub(super) fn position(&self, key: &ReeveTreeKey) -> Option<usize> {
        self.items.iter().position(|item| &item.key == key)
    }

    fn focused_position(&self) -> Option<usize> {
        self.items.iter().position(|item| item.focused)
    }
}

fn hand_notes(errand: &ListedErrand, placed: bool) -> Vec<&'static str> {
    let mut notes = Vec::new();
    match errand.hand_state {
        HandState::Working | HandState::Unknown => {}
        HandState::NeedsDecision => notes.push("decide"),
        HandState::Blocked => notes.push("blocked"),
        HandState::Done => notes.push("done"),
        HandState::Failed => notes.push("failed"),
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
    if !placed {
        notes.push("no pane");
    }
    notes
}

/// Builds the tree for `session` from the snapshot and the latest listing read.
pub(super) fn reeve_tree(
    snapshot: &ClientShellSnapshot,
    listing: Option<&ReeveListingRead>,
    session: &str,
) -> ReeveTree {
    let pane = |pane_id: Option<&String>| {
        pane_id.and_then(|pane_id| snapshot.panes.iter().find(|pane| &pane.pane_id == pane_id))
    };
    let mut tree = ReeveTree::default();
    let listing = match listing {
        Some(Ok(listing)) => Some(listing),
        Some(Err(error)) => {
            tree.listing_error = Some(error.clone());
            None
        }
        None => None,
    };
    let mut reeve_workspaces = HashSet::new();
    for reeve in listing
        .iter()
        .flat_map(|listing| &listing.reeves)
        .filter(|reeve| reeve.session.herdr_session == session)
    {
        let reeve_pane = pane(reeve.session.pane_id.as_ref());
        let workspace_id = reeve_pane.map(|pane| pane.workspace_id.clone());
        if let Some(workspace_id) = &workspace_id {
            reeve_workspaces.insert(workspace_id.clone());
        }
        let hands = listing
            .iter()
            .flat_map(|listing| &listing.errands)
            .filter(|errand| {
                errand.owner_reeve == reeve.name && errand.session.herdr_session == session
            })
            .collect::<Vec<_>>();
        let hand_tabs = hands
            .iter()
            .filter_map(|errand| pane(errand.session.pane_id.as_ref()))
            .map(|pane| pane.tab_id.as_str())
            .collect::<HashSet<_>>();
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
        tree.items.push(ReeveTreeItem {
            key: ReeveTreeKey::Reeve(reeve.name.clone()),
            kind: ReeveTreeItemKind::Reeve {
                badge: hands
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
                && !snapshot
                    .focused_tab_id
                    .as_deref()
                    .is_some_and(|tab_id| hand_tabs.contains(tab_id)),
            workspace_id,
        });
        for (index, errand) in hands.iter().enumerate() {
            let hand_pane = pane(errand.session.pane_id.as_ref());
            tree.items.push(ReeveTreeItem {
                key: ReeveTreeKey::Hand(errand.id.clone()),
                kind: ReeveTreeItemKind::Hand {
                    state: errand.hand_state,
                    notes: hand_notes(errand, hand_pane.is_some()),
                    last: index + 1 == hands.len(),
                },
                label: errand.id.clone(),
                pane_id: hand_pane.map(|pane| pane.pane_id.clone()),
                workspace_id: hand_pane.map(|pane| pane.workspace_id.clone()),
                focused: hand_pane.is_some_and(|pane| {
                    snapshot.focused_tab_id.as_deref() == Some(pane.tab_id.as_str())
                }),
            });
        }
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
        });
    }
    tree
}

fn hand_mark(state: HandState, palette: &Palette) -> (&'static str, ratatui::style::Color) {
    match state {
        HandState::Working => ("●", palette.yellow),
        HandState::NeedsDecision => ("?", palette.mauve),
        HandState::Blocked => ("!", palette.red),
        HandState::Done => ("✓", palette.teal),
        HandState::Failed => ("✗", palette.red),
        HandState::Unknown => ("·", palette.overlay0),
    }
}

enum TreeLine<'a> {
    Note(String),
    Gap,
    Header(&'static str),
    Item(&'a ReeveTreeItem),
}

/// Draws the reeve tree where herdr draws its spaces: a `reeves` header, the tree, then the
/// workspaces that hold no reeve under `other`. The section's last row stays herdr's footer.
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
        " reeves",
        Style::default()
            .fg(palette.overlay0)
            .add_modifier(Modifier::BOLD),
    );
    let mut lines = Vec::new();
    if let Some(error) = &tree.listing_error {
        lines.push(TreeLine::Note(format!("listing: {error}")));
    }
    let reeves = tree
        .items
        .iter()
        .filter(|item| matches!(item.kind, ReeveTreeItemKind::Reeve { .. }))
        .count();
    if reeves == 0 && tree.listing_error.is_none() {
        lines.push(TreeLine::Note("no reeve in this session".into()));
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

    let body = Rect::new(
        area.x,
        area.y.saturating_add(1),
        area.width,
        area.height.saturating_sub(2),
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
            TreeLine::Note(text) => render::put_text(
                buffer,
                rect.x + 1,
                rect.y,
                rect.width.saturating_sub(1),
                text,
                Style::default().fg(palette.overlay0),
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
    let (prefix, mark, label_style, note, note_style) = match &item.kind {
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
                note,
                note_style,
            )
        }
        ReeveTreeItemKind::Hand { state, notes, last } => {
            let mark = hand_mark(*state, palette);
            let label_style = if item.pane_id.is_some() {
                label_style
            } else {
                Style::default().fg(palette.overlay0)
            };
            (
                if *last { "  └ " } else { "  ├ " },
                mark,
                label_style,
                notes.join(" "),
                Style::default().fg(mark.1),
            )
        }
        ReeveTreeItemKind::Workspace { status } => (
            " ",
            (
                status_icon(*status, config.status_indicators),
                status_color(*status, palette),
            ),
            label_style,
            String::new(),
            Style::default(),
        ),
    };
    let mut x = render::put_segment(
        buffer,
        rect.x,
        rect.y,
        right,
        prefix,
        Style::default().fg(palette.overlay0),
    );
    x = render::put_segment(
        buffer,
        x,
        rect.y,
        right,
        mark.0,
        Style::default().fg(mark.1),
    );
    x = x.saturating_add(1);
    // The note keeps at most half the row, so a long one never hides the label entirely.
    let note_width = render::display_width(&note).min(right.saturating_sub(x) / 2);
    let label_right = right.saturating_sub(note_width + u16::from(note_width > 0));
    let label_width = label_right.saturating_sub(x);
    render::put_text(
        buffer,
        x,
        rect.y,
        label_width,
        &crate::ui::truncate_end(&item.label, usize::from(label_width)),
        label_style,
    );
    if note_width > 0 {
        render::put_text(
            buffer,
            right.saturating_sub(note_width),
            rect.y,
            note_width,
            &crate::ui::truncate_end(&note, usize::from(note_width)),
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
    // The walk's own keys are named as herdr's navigate bar names them.
    let segments = if mode == ClientShellMode::Navigate {
        vec![
            ("↑/↓".to_owned(), " move  "),
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
        Some(reeve_tree(snapshot, self.reeve_listing.as_ref(), session))
    }

    pub(in crate::client) fn set_reeve_listing(&mut self, read: ReeveListingRead) {
        self.reeve_listing = Some(read);
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

    /// Picks the selected item: shows a reeve's or a hand's own pane, or focuses a workspace.
    /// None when the tree is not drawn or nothing is selected, so herdr's own pick runs.
    pub(super) fn pick_navigate_reeve_target(&self) -> Option<Option<crate::api::schema::Method>> {
        let tree = self.current_reeve_tree()?;
        let item = tree
            .items
            .get(tree.position(self.navigate_reeve_key.as_ref()?)?)?;
        Some(if let Some(pane_id) = &item.pane_id {
            Some(crate::api::schema::Method::PaneFocus(
                crate::api::schema::PaneTarget {
                    pane_id: pane_id.clone(),
                },
            ))
        } else {
            item.workspace_id.as_ref().map(|workspace_id| {
                crate::api::schema::Method::WorkspaceFocus(crate::api::schema::WorkspaceTarget {
                    workspace_id: workspace_id.clone(),
                })
            })
        })
    }
}
