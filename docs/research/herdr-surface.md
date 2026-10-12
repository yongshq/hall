_Source: the herdr-surface errand report (`~/.reeve/errands/herdr-surface/report.md`), written 2026-10-11, herdr 0.9.0 installed, docs read for 0.9.3._
# herdr-surface: can a reeve-centred TUI be built ON herdr?

Short answer: yes, both ways are possible. herdr 0.9.0 (installed) has a public JSON socket API with a
live event stream, full read and write on panes, and a read-only or writable **live terminal frame
stream** for any pane (`herdr terminal session observe|control`). So an external TUI can list
reeves and hands, react to events, and render live pane content itself. The other route works too:
herdr's own UI can be reshaped from outside, but only within limits. Plugins have no native UI
outside a terminal pane. The sidebar can be filtered, sorted and given custom tokens, but it cannot
gain new grouping levels.

Versions: installed `herdr 0.9.0` (Homebrew, `/opt/homebrew/Cellar/herdr/0.9.0`), protocol 22,
endpoint generation 1 (`herdr status`). Docs read are for **0.9.3** (`web/llms.txt:3`, "Current
stable release: 0.9.3"). Where the two differ, it is flagged.

Evidence files (scratch, not durable): schema from `herdr api schema --json` (275 KB), `herdr --skill`,
`herdr --default-config`. Web docs: `/Users/yong/.reeve/errands/herdr-surface/web/llms-full.txt`
(cited below as `full:<line>`).

## 1. Socket API surface

- Transport: newline-delimited JSON over a Unix socket. Default session socket is
  `~/.config/herdr/herdr.sock`. Named sessions use `~/.config/herdr/sessions/<name>/herdr.sock`
  (full:4964, full:4991-4992). Request `{"id","method","params"}`, reply `{"id","result"|"error"}`
  (full:4969-4975).
- Schema: `herdr api schema --json` gives protocol 22, schema_version 1, and these sections:
  `error_response, event, request, subscription_event, success_response`.
- 102 methods in the 0.9.0 schema (`jq '.schemas.request.oneOf[].properties.method.const'`).
  - **Read / query**: `ping`, `session.snapshot`, `workspace.list|get`, `tab.list|get`,
    `pane.list|get|current|layout|process_info|neighbor|edges|read|selection.read`,
    `agent.list|get|read|explain`, `worktree.list`, `layout.export`, `plugin.list|action.list|log.list`,
    `integration.list`, `server.agent_manifests`, `pane.graphics.info`.
  - **Wait / stream (read-only)**: `events.subscribe`, `events.wait`, `agent.wait`, `pane.wait_for_output`.
  - **Display-only writes** (change no process): `pane.report_metadata`,
    `workspace.report_metadata`, `agent.view.set|clear`, `notification.show`,
    `client.window_title.set|clear`.
  - **Mutations**: `workspace.create|focus|rename|move|move_block|close`, `tab.create|focus|rename|move|close`,
    `pane.split|swap|move|zoom|resize|focus|focus_direction|rename|close|scroll|input.set|link.activate|copy_motion|copy_search|edit_scrollback`,
    `pane.send_text|send_keys|send_input`, `agent.prompt|send_keys|start|rename|focus`,
    `pane.report_agent|report_agent_session|clear_agent_authority|release_agent`,
    `layout.apply|set_split_ratio`, `worktree.create|open|remove`, `plugin.*` (link, enable, invoke, pane open),
    `integration.install|uninstall`, `popup.close`, `server.stop|reload_config|live_handoff|reload_agent_manifests`,
    `release_notes.dismiss`, `product_announcement.dismiss`, `command.invoke`, `client_shell.surface.set`.
  - Not in the documented method table (full:4589-4604) but present in the 0.9.0 schema:
    `client_shell.surface.set` ("Updates whether the requesting client shell receives and controls pane
    presentation"), `command.invoke` ("Opaque endpoint-issued command identifier from the client-shell
    projection"), `server.live_handoff`, `pane.copy_*`, `pane.scroll`, `pane.selection.read`,
    `pane.link.activate`, `pane.edit_scrollback`, `integration.list`, `*.dismiss`. These look like
    internal methods for herdr's own client. A guess: undocumented, so treat them as unstable.
  - `pane.graphics.info|set|clear` are in the 0.9.0 schema, but the 0.9.3 docs say they "have been
    removed ... There is no replacement socket image API" (full:4653). Version drift.
- **Event stream: yes, push-based, not polling only.** `events.subscribe` keeps the connection open
  and pushes events (full:5128-5142, full:4980).
  - Probed live, read-only, for 4 seconds on `default`: the reply was
    `{"id":"sub_1","result":{"type":"subscription_started"}}`, with no events in that window.
  - Subscription types (schema): `workspace.created|updated|metadata_updated|renamed|moved|reordered|closed|focused`,
    `worktree.created|opened|removed`, `tab.created|closed|focused|renamed|moved`,
    `pane.created|closed|updated|focused|moved|exited|agent_detected`, `pane.output_matched` (needs pane_id, source, match),
    `pane.agent_status_changed` (needs **pane_id**: per pane, no global form), `pane.scroll_changed` (needs pane_id),
    `layout.updated`.
  - Limits: history is not durable. A slow reader gets `events_lost` and the connection is closed
    (full:5144). Events and snapshots share no sequence. The documented recovery is: resubscribe,
    take `session.snapshot`, and treat events as invalidations (full:5146-5148).
    `pane.agent_status_changed` needs one subscription entry per pane, and one bad pane id rejects
    the whole request (full:5142).
  - `events.wait` gives a one-shot match (EventMatch in the schema, which includes `pane_output_changed`).
- `session.snapshot` / `herdr api snapshot` bootstraps a client cache (full:4610-4612). Probed on
  `default`, the result keys are
  `agents, focused_pane_id, focused_tab_id, focused_workspace_id, layouts, panes, protocol, tabs, version, workspaces`.
  The agent keys include `agent_status, state_change_seq, terminal_title_stripped, agent_session, foreground_cwd`.
  At the time of the probe: workspace `wS` was labelled `Aldric`, `wS:p1` was claude idle "Pending items",
  and `wS:p9Z` was claude working "Herdr-surface brief".
- How reeve uses the API today: only CLI wrappers, always with explicit `--session`
  (`backends/herdr.sh:19-28`). It uses `agent explain` rather than `agent get` because `agent_status`
  latches (`backends/herdr.sh:539-542`), and `agent wait` with no idle wait because of #3530
  (`backends/herdr.sh:550-553`). It never uses `events.subscribe` (grep: no hit in `backends/herdr.sh`).

## 2. Pane content: stream, input, embedding

- **Live stream: yes, for external clients.** `herdr terminal session observe <target> [--cols N] [--rows N]`
  is read-only. It prints NDJSON `terminal.frame` records with base64 ANSI, then `terminal.closed`.
  Many observers may watch one terminal "without taking input, resize, scroll, or takeover
  ownership" (full:3988-3994).
  - Probed on my own pane `wS:p9Z` for 3 seconds (`perl -e 'alarm 3; exec @ARGV' herdr terminal session observe ...`).
    Two frames came back: `{"seq":1,"full":true,"width":100,"height":30,"encoding":"ansi","len":5576}`,
    then `{"seq":2,"full":false,...}`. The decoded bytes start `ESC[?2026h ESC[?25l ... ESC[2J ESC[1;1H`,
    so they are ready-to-paint ANSI with synchronized output.
  - The pane itself is 212x53 (`herdr pane layout`), yet the frames came back at the requested
    100x30. So the observer gets a rendering at its own size. Nothing left running afterwards
    (`pgrep` found none).
- **Interactive stream with input: yes.** `herdr terminal session control <target> --takeover --cols --rows`
  gives the same frames and reads JSON commands on stdin: `terminal.input` (text or base64),
  `terminal.resize`, `terminal.scroll`, `terminal.mouse`, `terminal.release`. One controller owns input
  and resize at a time (full:3996-4002). Present in 0.9.0 (`herdr terminal` usage lists it).
- Direct attach: `herdr agent attach <name>` and `herdr terminal attach <terminal_id> [--takeover]` take
  over the current terminal for a single pane. Linux and macOS only (full:3958-3982, full:594-616).
- Snapshots: `pane.read` / `agent.read` take sources `visible|recent|recent_unwrapped|detection` and
  formats `text|ansi` (schema `ReadSource`, `ReadFormat`). `pane.wait_for_output` and the
  `pane.output_matched` subscription match by substring or regex.
- Input: `pane.send_text`, `pane.send_keys`, `pane.send_input` (text plus keys), `pane run` (text plus
  Enter, atomically), `agent.prompt` (bracketed paste plus Enter, optional wait), `agent.send_keys`
  (full:4616, skill.md:127-150). Reeve's known caveat: Enter does not always take, see herdr #2422
  (`backends/herdr.sh:336-349`).
- **Embedding in a separate TUI: feasible through observe/control.** That route makes the TUI a
  terminal emulator or ANSI compositor for the frames: it parses ANSI into its own grid, which is
  the TUI's job and not herdr's.
  - herdr's own client speaks a different protocol: "numbered binary protocol ... for same-install
    and internal operations, including direct terminal attach and live handoff". The client-rendered
    UI uses "endpoint generation" codecs for snapshot, screen, input and blob (full:5264-5266).
    That client protocol is not documented for third parties (gap).
  - Inside herdr itself, a plugin pane is a normal terminal pane (full:4219), so a reeve TUI could
    instead run as a herdr pane or tab.

## 3. Plugins

- What one is: a directory holding `herdr-plugin.toml` plus any argv commands. The language is
  anything: "Bash script, JavaScript app, Lua script, Rust binary, or any other argv command"
  (full:4008). There is no SDK: "The entire Herdr CLI is the plugin API" (full:4014).
- Manifest sections: `[[build]]`, `[[startup]]`, `[[actions]]`, `[[events]]` (`on = "<event>"`), `[[panes]]`,
  `[[link_handlers]]`. Required fields: `id, name, version, min_herdr_version` (full:4034-4082).
- Hooks:
  - `[[startup]]` runs once after restore, and again after a live handoff. It is not a daemon
    (full:4176-4184).
  - `[[events]]` runs a command per matching event, with `HERDR_PLUGIN_EVENT` and
    `HERDR_PLUGIN_EVENT_JSON` set (full:4190).
  - `workspace.metadata_updated` "does not invoke plugin event hooks" (full:5114).
  - Plugins can react to events. Each event spawns a process; there is no long-lived plugin
    process unless the plugin opens a pane or holds its own subscription.
- UI:
  - "Runtime action registration and native non-terminal plugin UI are not part of plugin v1"
    (full:4016). So a plugin **cannot draw into the sidebar or the status bar directly**.
  - What it can do:
    - `[[panes]]` open a terminal UI as overlay, popup, split, tab or zoomed (full:4198-4219,
      `PluginPanePlacement` enum). A plugin pane is a normal pane after it opens.
    - Shape the built-in Agents sidebar with `agent.view.set`: filter by status, workspace, tab,
      pane, agent, seen or `{token}`, and sort by attention, order, status or `{token}`. Transient
      until cleared; reapply it from a startup hook (full:4794-4845).
    - Push `$name` tokens into sidebar rows via `pane.report_metadata` / `workspace.report_metadata`,
      at most 16 keys per report and 32 per resource, values up to 80 characters, with optional TTL.
      Not restored after a server restart (full:5104-5122).
    - Reorder workspaces with `workspace.move_block` (full:4608).
    - Add `ui.tab_bar_right` entries of type `command`, which is config and not a plugin
      (default-config:251-255).
  - **Custom grouping of workspaces: no.** The only grouping is worktree parent and child
    (full:2267-2269). There is no folder or section concept in the config reference (grep for
    group|folder|section: only worktree hits).
- Installed here: one plugin, `layouts` (Python, actions only,
  `/Users/yong/dotfiles/herdr/.config/herdr/plugins/github/layouts-7c31f497d1da/herdr-plugin.toml`).
  The registry is `~/.config/herdr/plugins.json`. Plugins are global per user, across sessions (full:4162).
- Storage: none managed. `HERDR_PLUGIN_CONFIG_DIR` and `HERDR_PLUGIN_STATE_DIR` are paths only (full:4243-4247).
- Marketplace: GitHub topic `herdr-plugin` (full:4253).

## 4. Customising herdr's own UI (config.toml, read only)

- Current overrides (`~/.config/herdr/config.toml`): `onboarding=false`, theme `catppuccin`,
  `terminal.new_cwd="follow"`, alt+hjkl pane focus, two `[[keys.command]]` bindings of type
  `plugin_action` (layouts), and `[ui.sidebar.agents] rows = [["state_icon", {token="tab",bold=true,dim=false}], ["terminal_title_stripped"]]`.
- Sidebar keys (full:1830-1964; default-config:167-296):
  - Width, min/max, start collapsed, collapsed mode, `mobile_width_threshold`.
  - `ui.agent_panel_sort` = `spaces` (grouped by workspace) | `priority` (attention queue).
  - `ui.status_indicators` = `dots|symbols`.
  - `ui.sidebar.agents.rows` and `rows_by_agent`: built-ins `state_icon, state_text, machine, workspace, tab, pane, agent, terminal_title, terminal_title_stripped`, plus `$name` metadata tokens.
  - `ui.sidebar.spaces.rows`: built-ins `state_icon, workspace, branch, git_status`, plus `$name` workspace tokens.
  - Per-token style rules (`equals|contains|starts_with|gt|lt`) are documented for 0.9.3
    (full:2588-2601) but absent from the 0.9.0 `--default-config` (`grep rules` gave no hit). Treat
    them as possibly 0.9.1 or later (gap).
- Labels: workspace, tab and pane labels can be renamed by API. Reeve already labels workspaces
  with the reeve name and tabs `<Office> of <Name>` (`backends/herdr.sh:159-200`, `590-606`).
  `herdr agent rename` sets a display name, and `display_agent` / `state_labels` come through
  metadata (full:559-592, full:5104).
- Tab bar: `ui.tab_bar_position`, `hide_tab_bar_when_single_tab`, `tab_bar_right` (zoom, hostname,
  datetime, text, command). `ui.window_title` takes tokens.
- Notifications: `ui.toast.delivery = off|herdr|terminal|system` (default off),
  `delay_seconds`, `ui.toast.herdr.position`. The `ui.sound.*` keys can be set per agent. The API
  method `notification.show` returns `shown|disabled|rate_limited|no_foreground_client|busy`
  (full:1964-1986, full:4737-4757).
- Keybindings: `[keys]` for every action, `[[keys.command]]` with `type = shell|pane|popup|plugin_action`,
  and indexed jumps (`tabs`, `workspaces`, `agents`) (default-config:69-156, full:2286-2408).

## 5. Multiple clients and sessions

- **Two clients on one session at once: yes.** "By default, Herdr runs as a background server plus
  one or more attached clients ... With multiple clients, each can view its own workspace and tab";
  on a shared tab, the last client to interact controls the pane sizes (full:1240-1244). Done/seen
  badges are tracked per client (full:1218). `pane.focused` events report any client's selection
  but do not identify the client (full:5150).
- Named sessions: `--session <name>`, `herdr session list|attach|stop|delete`. Each one is a separate
  server, with its own socket and runtime but a shared global config (full:3857-3881). Only
  `default` runs here: `herdr session list --json` gave
  `{"name":"default","default":true,"running":true,...}`. No throwaway session was created: not needed.
- Remote: `herdr --remote <ssh-target> [--session]`. Here the remote server owns the panes and the
  local herdr draws the UI. Saved SSH machines (`herdr machine ...`) combine several servers in one
  window (full:3889-3956, full:2805+).
- Socket API clients are separate from UI clients: any number of API connections and observers
  can run alongside them.

## 6. Open source, release cadence, feature requests

- Open source: yes. `github.com/herdrdev/herdr`, written in Rust, licence Apache-2.0
  (`web/repo.json`: `"language":"Rust","license":"Apache-2.0"`; `web/LICENSE` is the Apache 2.0 text).
  Default branch `master`. Created 2026-03-27, last pushed 2026-10-11T02:20:37Z. 43,335 stars,
  338 open issues. Issues and discussions both enabled.
- Cadence: from 0.7.x, mentioned in the reeve backend notes, to 0.9.3 between creation (March 2026)
  and now. `~/.config/herdr/release-notes.json` shows `0.9.3`, "a hotfix release for v0.9.2", and
  issue numbers have reached #4751. There is also a preview channel built from `master` as GitHub
  prereleases (full:3185-3228). Exact release dates were not established (gap).
- Local install is Homebrew, so `herdr update` and live handoff are disabled. Updating means
  `brew upgrade`, and the old server keeps running until it is restarted (full:3185-3228,
  full:4486). Installed 0.9.0 is 3 patch versions behind stable.
- Feature-request route: GitHub issues and discussions on `herdrdev/herdr` (`has_issues`,
  `has_discussions` true). The docs cite issues for bugs (full:5310, 5332, 5364, 5374). No
  CONTRIBUTING file or written feature-request policy was found in the material available (gap).
  Plugins are the stated route for workflows: "Plugins exist so Herdr can stay lean" (full:4010).

## 7. What herdr already shows about agents (overlap with a reeve view)

- Detection covers claude, codex and about 20 more, through process detection, screen manifests
  and integrations (full:430-486). States: `blocked, working, done, idle, unknown` (full:1210-1216).
- Rollup: a blocked agent marks its pane, tab and workspace as blocked, and a working agent marks
  its workspace as active (full:549-557). Today that is reeve-shaped already: workspace `Aldric`
  holds the reeve pane plus hand tabs.
- Agents sidebar: per-agent rows (state icon, workspace, tab, title), `priority` attention sort,
  and the Goto picker (`prefix+g`), which filters blocked, working, idle and done, grouped by
  workspace (full:3724).
- Notifications: toasts and sound on done or needs-input, after `delay_seconds` (full:1964-1990).
- `herdr agent list` here shows only `agent, pane_id, workspace_id, agent_status`. `name` is null
  and `tokens` are null, so reeve reports no metadata today.
- What herdr does not know: errand id, office, owning reeve, status-file state (`needs-decision`,
  `blocked:`, `done:`), idle and wedged thresholds, branch-landed state. These live in
  `$REEVE_HOME`. Herdr's `blocked` means a dialog on screen, which matches reeve's `waiting` wake;
  it does not mean reeve's `blocked:` line.
- Bridges that exist: reeve could push errand facts as pane tokens (`$errand`, `$office`, `$state`)
  plus a workspace token per reeve, then render them via `ui.sidebar.agents.rows` and filter or
  sort via `agent.view.set {token}`. Herdr's `blocked` and `attention` sorting would still come
  from screen state, never from reeve's status file, because `state` comes only through
  `pane.report_agent`, which takes lifecycle authority from the claude integration (full:5026,
  full:5104). A fact, not a recommendation.

## Gaps

- Did not run `events.subscribe` long enough to capture real events (4 seconds, none fired). The
  payload shapes come from the schema and docs only.
- Did not test `terminal session control` (writable, takes over input). Only `observe` was probed,
  on my own pane.
- herdr's internal client protocol (endpoint generation, binary codecs) is undocumented, and
  `client_shell.surface.set` and `command.invoke` purposes are inferred from the schema text only.
- 0.9.0 vs 0.9.3 drift: the docs read are for 0.9.3. Confirmed in 0.9.0: method list, subscription
  types, `terminal session observe|control`, `plugin pane`, `agent.view.set`. Not confirmed: sidebar
  token style `rules`. `pane.graphics.*` is in 0.9.0 but removed per 0.9.3 docs.
- No access to the GitHub releases list or CONTRIBUTING: release dates and the formal
  feature-request policy are unknown.
- No source code read (web tools barred), so behaviour claims rest on docs, schema and probes.
- Plugin event hook latency and concurrency (one process per event) is not measured.
