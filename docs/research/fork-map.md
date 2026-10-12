_Source: the fork-map errand report (`~/.reeve/errands/fork-map/report.md`), written 2026-10-12, copied into hall unchanged below this line._
# fork-map: turning herdr v0.9.0 into hall

Source: herdr at `v0.9.0` in `~/.reeve/errands/herdr-launch/src/herdr` (read only; paths below relative to it, at v0.9.0, unless marked). Reeve at `b4b6cbe` (paths `reeve:`). Hall at `eb40645` (paths `hall:`). No herdr, server or session was run. One build of v0.9.0 was made in my session scratchpad (section 6). Facts already in `herdr-launch/report.md` are cited as "launch §N", not re-derived.

## Answer first

- **Separation is cheap at the root, expensive at the edges.** One function names every config and state dir (`src/config/io.rs:22-28`). Renaming it to `hall` moves sessions, sockets, logs, plugins, detection overrides, manifest cache, announcements and endpoints in one line. What remains: about 10 inbound env names, 4 network paths, `hall update` (it would overwrite hall with herdr), 2 shared paths outside the dir root, and the binary name.
- **Reeve unchanged cannot dispatch into hall safely.** macOS panes start a login shell (`src/pane.rs:1629-1637`, `:1710-1712`), so `/etc/zprofile` `path_helper` puts `/opt/homebrew/bin` (from `/etc/paths.d/homebrew`) ahead of any shim dir hall injects. Reeve then runs Homebrew `herdr`, whose `session list` names the **live** default, and hands land in the live herdr. Smallest safe fix is on reeve's side (section 3).
- **Reeve tree: map reeve = workspace, hand = tab.** That is the layout reeve's herdr backend already builds (`reeve:backends/herdr.sh:159-201`, `:236-246`), the client snapshot already carries every tab with its own agent status (`src/protocol/wire.rs:1036-1046`), and herdr already renders reeve-pushed tokens in the sidebar (`workspace/pane report-metadata`, `src/cli/spec.rs:213-221`, `:671-687`; `$name` tokens, `src/main.rs:336-355`). Mostly a client-side sidebar change.
- **Sound needs almost nothing new.** In a fork the hall window is herdr's foreground client, so herdr's own request and done sounds play. Decision 19's reason for hall playing its own sound is gone.

## 1. Rename and separation

### Where the names live

| What | Where | Size of change |
|---|---|---|
| Config and state dir name | `src/config/io.rs:22-28` `app_dir_name()`: `herdr` (release), `herdr-dev` (debug). Used by `config_dir()` `:30-35`, `state_dir()` `:37-42` | 2 strings. Moves everything in launch §3's table except the rows below |
| Session dirs, sockets | `src/session.rs:157-185`: `CFG/sessions/<n>/herdr.sock`, `herdr-client.sock` | Free with the dir rename. Socket file names can stay (path differs) |
| Logs | `herdr-server.log` `src/server/headless/bootstrap.rs:209,217`; `herdr-client.log` `src/client/config_reload.rs:4`; `herdr.log` `src/logging.rs:36-37`; filter env `HERDR_LOG` `src/logging.rs:23` | Free with dir rename; names cosmetic |
| Binary name | `Cargo.toml:2` `name = "herdr"`; self-exec uses `current_exe()` (launch §1), so the name is not load bearing | 1 line (`[[bin]] name = "hall"` or package rename) + 55 `CARGO_BIN_EXE_herdr` refs in `tests/` (`git grep -c`), mechanical |
| Worktree dir default | `~/.herdr/worktrees`: `src/config/model.rs:1153`, default config text `src/main.rs:228` | 2 lines, else shared with live |
| Pane graphics dir | `$XDG_RUNTIME_DIR` or `/var/tmp` `/herdr-pane-graphics-<uid>`: `src/pane_graphics_files.rs:207` | 1 line, else shared with live per uid |
| Sound temp file | `$TMPDIR/herdr-sound-<pid>-<id>.mp3`: `src/sound.rs:110` | Per pid, no clash. Cosmetic |
| launchd label | None in source (`git grep -i 'launchd\|plist\|launchctl'` over `src packaging distribution`: no hit). Only Homebrew's `sh.brew.herdr.plist` (launch §1) | Nothing, unless hall wants KeepAlive later |
| Plugin manifest file name | `herdr-plugin.toml` (`src/app/api/plugins/manifest.rs:124`, 13 sites) | Leave: plugin dirs already move with the dir rename |
| User-facing text | 1962 string literals containing "herdr" in `src/**/*.rs` (`git grep -c`), most in tests and help | Cosmetic, do last or never |

### Env names (constants)

Inbound: these decide which server or config a `hall` process uses. Outbound: set into every pane.

| Name | Defined | Direction | Separation risk if kept |
|---|---|---|---|
| `HERDR_SOCKET_PATH` | `src/api/mod.rs:20` | in and out (`src/integration/env.rs:29`) | **High.** `hall` typed in a live herdr pane inherits it; without `--session` it wins (`src/session.rs:80-83`, `:172-181`) and the hall client derives the client socket from it (launch §3), so hall would attach to the live server (same protocol 22) |
| `HERDR_CLIENT_SOCKET_PATH` | `src/server/socket_paths.rs:9` | in | High, same route |
| `HERDR_SESSION` | `src/session.rs:10` | in | Medium: picks a session name, now under hall's own dir |
| `HERDR_CONFIG_PATH` | `src/config.rs:56` | in | Medium: a live pane exporting it would point hall at herdr's config |
| `HERDR_ENV` | `src/main.rs:3` (nested guard `:444-448`, `:462`) | in and out (`src/pane.rs:143`) | **High, other way:** `hall` typed in a live herdr pane is refused as nested unless `[experimental] allow_nested` |
| `HERDR_PANE_ID`, `HERDR_TAB_ID`, `HERDR_WORKSPACE_ID` | `src/integration/env.rs:8-10`, set `src/pane.rs:153-155` | out | Read by reeve (section 3) and by the liege's installed claude hook |
| `HERDR_BIN_PATH` | literal `src/integration/env.rs:31` | out | Read by plugins (`src/app/api/plugins/panes.rs:265`, `runtime.rs:51`) |
| `HERDR_STARTUP_CWD` | `src/server/autodetect.rs:30` | internal | Low |
| `HERDR_DISABLE_SOUND`, `HERDR_LOG`, `HERDR_PLUGIN_*` (`src/app/api/plugins/env.rs:20-26`, `runtime.rs:46-78`), `HERDR_ROLE` (`src/app/api/layouts.rs:752`) | as cited | mixed | Low |
| Test and fake-only (`HERDR_FAKE_*`, `HERDR_TEST_*`) | `src/update.rs:33-34`, `src/product_announcements.rs:9-12`, ... | in | None |

- 33 integration asset files read `HERDR_*` (`git grep -c HERDR_ -- src/integration/assets` = 33 files). They are written to `~/.claude`, `~/.codex` and so on only by an explicit `integration install` (launch §3), under the same file names herdr uses (`src/integration/mod.rs:32-50`: `herdr-agent-state.sh`). **`hall integration install` would overwrite the live herdr's hooks.** The liege has `~/.claude/hooks/herdr-agent-state.sh` installed and referenced from `~/.claude/settings.json` (checked: `ls ~/.claude/hooks`, `grep herdr ~/.claude/settings.json`).
- That installed hook routes by `HERDR_ENV`, `HERDR_SOCKET_PATH`, `HERDR_PANE_ID` only (`src/integration/assets/claude/herdr-agent-state.sh:20-22`). So inside hall panes it reports to hall if hall keeps those three names outbound, and does nothing if hall renames them.

### Network calls (all through `curl`, `src/noninteractive_process.rs:10`)

| Call | Where | Gate |
|---|---|---|
| Update manifest `https://herdr.dev/latest.json`, `preview.json` (carries product announcements, `src/update.rs:365-378`) | `src/update.rs:25-26`, fetched `:340` | `[update] version_check` + `auto_updates_enabled` `src/app/mod.rs:166-168`, thread spawn `:540-548` |
| Homebrew formula API | `src/update.rs:27`, `:582` | same |
| Release download and **binary replace** | `src/update.rs:624-693` (`fs::rename` over `current_exe` `:693`); explicit `herdr update` subcommand `src/main.rs:556` | **None for the explicit command.** `hall update` would download herdr and replace hall's binary with it |
| Agent detection catalog `https://herdr.dev/agent-detection/index.toml` | `src/detect/manifest_update.rs:16`, `:509-552`; also `herdr server update-agent-manifests` `src/cli/server.rs:14` | `[update] manifest_check`, same gate `src/app/mod.rs:549-555` |
| Remote install (`--remote`): fetches herdr from herdr.dev and installs `~/.local/bin/herdr` on the remote host | `src/remote/attach.rs:28-29`, `:191`, `:456`, `:1530`, `:1554` | Only on `--remote` |

### Minimal patch set: hall shares nothing with an installed herdr

| # | Patch | Size |
|---|---|---|
| P1 | `app_dir_name()` returns `hall` / `hall-dev` (`src/config/io.rs:22-28`) | 2 lines |
| P2 | Binary `hall` (`Cargo.toml`), test `CARGO_BIN_EXE_` refs | 1 line + 55 mechanical |
| P3 | Inbound env renamed to `HALL_*`: socket path, client socket path, session, config path, env marker (`src/api/mod.rs:20`, `src/server/socket_paths.rs:9`, `src/session.rs:10`, `src/config.rs:56`, `src/main.rs:3`). Tests set these names (e.g. `tests/cli/harness.rs:180-240`, `tests/auto_detect.rs:110-156`), so tests change too | 5 constants + test sed. Outbound names: see section 3, this is the one real choice |
| P4 | Updates off at the source: `auto_updates_enabled` returns false (`src/app/mod.rs:166-168`); `update` subcommand removed or refused (`src/main.rs:556`); `update-agent-manifests` refused (`src/cli/server.rs:14`); default `version_check`/`manifest_check` false (`src/config/model.rs:43-44`) so the config says so too | ~15 lines. Config alone is not enough: the explicit `update` path ignores it |
| P5 | `--remote` disabled, or its URLs and install path renamed (`src/remote/attach.rs`) | ~10 lines to refuse |
| P6 | `~/.herdr/worktrees` to `~/.hall/worktrees` (`src/config/model.rs:1153`, `src/main.rs:228`); pane graphics dir prefix (`src/pane_graphics_files.rs:207`) | 3 lines |
| P7 | `integration install` refused, or asset install names renamed (`src/integration/mod.rs:25-60`) so it never overwrites `~/.claude/hooks/herdr-agent-state.sh` | refuse: ~5 lines; rename: ~40 assets |
| P8 | Protocol number bumped (`src/protocol/wire.rs:20`, 22 to e.g. 1022) so any herdr CLI, client or observer that reaches a hall socket, or the reverse, fails at the guard (`src/cli.rs:764-799`, `src/protocol/wire.rs:1694-1710`) instead of half working | 1 line. Belt and braces for P3 |
| P9 | Agent-facing text: `src/cli.rs:50-52` points agents at herdr.dev docs; `--skill` prints herdr's skill | cosmetic |

- Total for P1 to P8: roughly 50 to 80 changed lines plus mechanical test renames. Guess, not built.
- Debug builds already use `herdr-dev` dirs and skip background updates (launch §3), so a debug build of the fork is separated by P1 alone while P2 to P8 land.

## 2. Reeves at the top level

### herdr's model and screen

- Server state: `Workspace { tabs: Vec<Tab>, worktree_space, metadata_tokens, ... }` (`src/workspace.rs:176-206`); `Tab { panes, layout, root_pane, ... }` (`src/workspace/tab.rs:38-51`). Agent state lives on the terminal (`src/workspace/aggregate.rs:26-45`); a workspace shows its highest-priority pane state (`:50-76`: blocked, then unseen idle, then working).
- The client draws the chrome, not the server. The server sends a `ClientShellSnapshot` (`src/protocol/wire.rs:907-939`, built in `src/server/client_shell.rs`, 661 lines) with `workspaces`, `tabs` (each with its own `agent_status`, `:1036-1046`), `panes`, `agents` (with `tokens`, `state_labels`, `:1061-1075`). Rendering is `src/client/shell/` (47k lines in `src/client/`).
- Sidebar top: "spaces" (`src/client/shell/sidebar.rs:183-300`). **Already a two-level tree**: workspaces of the same repo group under a parent row with indented, collapsible children, keyed by `worktree.key` (`workspace_entries`, `:448-535`; collapse `:557-585`).
- Sidebar bottom: the agents panel, one row per detected agent pane, rows built from config templates (`src/client/shell/agent_sidebar.rs:237-312`); sort "spaces" or "priority" (`:20-50`); an external view can impose order and a label (`agent.view.set`, `src/api/schema/agents.rs:53-61`).
- Row templates take built-ins plus any `$name` token pushed through `workspace report-metadata` or `pane report-metadata` (`src/main.rs:336-355`; CLI `src/cli/spec.rs:213-221`, `:671-687`: `--token NAME=VALUE`, `--ttl-ms`, `--state-label STATUS=TEXT`, `--display-agent`, `--title`).

### Where the reeve tree fits

| | A. Map: reeve = workspace, hand = tab | B. New level: reeve groups workspaces, hand = workspace |
|---|---|---|
| Matches reeve today | Yes: `ensure_group` makes or adopts the reeve's workspace labelled with its name (`reeve:backends/herdr.sh:159-201`), each hand is a tab in it (`:236-246`) | No: hands as workspaces is only reeve's fallback (`:253-271`) |
| Data | Already in the snapshot (tabs per workspace with status) | Needs a reeve key per workspace: a metadata token (`reeve=<name>`) could serve as the group key in place of `worktree.key` |
| hall code | `sidebar.rs`: emit tab rows under each workspace row (~150 to 300 lines); hit testing in `mouse.rs` (2288 lines) and keyboard selection in `state.rs`/`actions.rs` (~100 to 200 lines). No server or protocol change | Reuse `workspace_entries` grouping with a token key (~100 to 200 lines in `sidebar.rs` + config), plus hit testing as A. Reeve backend change to put hands in workspaces |
| Marks and badge (decisions 12, 17) | Hand mark: herdr's own per-tab status today; reeve's hand state as a pane token. Badge: workspace token (`$waiting`) | Same tokens |
| Risk | Non-reeve workspaces of the liege mix into the same list (filter or section needed); a hand whose tab failed falls back to its own workspace and shows at top level (`herdr.sh:249`) | More reeve change; more churn exposure (sidebar.rs changed +77/-39 in v0.9.0..v0.9.3) |

- Rough size of A: 300 to 600 lines of client code and tests. Guess from file sizes, not designed.
- Possible zero-code first look (not tried): `agent_panel_sort = "spaces"` with rows `[["state_icon","workspace","tab"],["$hand_state","$question"]]` already shows reeve name (workspace label) and hand label (tab label, `Scout of Aldric`) per agent row, once reeve pushes tokens.

### How hall learns identity and state

| Route | Identity | State | Fits ADR 0002 | Notes |
|---|---|---|---|---|
| Reeve listing (plan, stand-in today) | `session` location per reeve and errand (`hall:docs/listing-draft.md`) | reconciled hand state, question, flags | Yes | Hall polls; maps `pane_id` to its own panes. `herdr_session` becomes a hall session name |
| Reeve pushes tokens (`hall workspace report-metadata --source reeve --token waiting=2`, `hall pane report-metadata <pane> --token hand_state=blocked`) | implicit (pushed onto the reeve's workspace and hand's pane) | same reconciled values | Yes in spirit: reeve still owns reconciliation, hall reads no file | Reeve change. TTL (`--ttl-ms`) drops stale marks if a reeve dies. Herdr renders it with no new code |
| Pane env | `REEVE_HAND=<id>` is on every hand's command line (`reeve:bin/reeve-harness:213`) | none | n/a | Herdr already reads other processes' `KERN_PROCARGS2` for argv (`src/platform/macos.rs:394-448`), which carries env too. Brittle, identity only. Not tried |
| Labels | workspace = reeve name, tab = `<Office> of <Reeve>` | none | n/a | Reeve's own rule: labels are never authority (`reeve:backends/README.md:93-94`) |
| Herdr agent detection | agent kind only | working, blocked, idle, done per pane | For alerts only (decision 19) | Same signal reeve reads (`agent explain`) |

## 3. Reeve compatibility

### What reeve calls (`reeve:backends/herdr.sh`)

| CLI | API method | Lines |
|---|---|---|
| `herdr session list --json` | none: reads the config dir (`src/session.rs:187-226`) | 15, 131, 429, 451 |
| `herdr status server --json` | `ping` | 116, 480 |
| `herdr --version` | none | 122; also `reeve:bin/reeve-doctor:95` |
| `workspace get/rename/create/close` | `workspace.get/rename/create/close` | 65, 188, 192, 197, 209, 221, 265, 468, 585, 622 |
| `tab get/create/close/rename` | `tab.get/create/close/rename` | 63, 237, 248, 576, 604-605, 618 |
| `worktree open` | `worktree.open` | 257 |
| `pane list/get/read/process-info/close` | `pane.list/get/read/process_info/close` | 90, 241, 269, 327, 374, 417, 434, 454, 476, 489, 577, 587 |
| `pane run` | `pane.send_input` with Enter (`src/cli/pane.rs:1047-1059`) | 348, 386 |
| `pane send-keys` | `pane.send_keys` | 358, 412 |
| `agent get/explain/wait` | `agent.get/explain/wait` | 407, 488, 543, 559 |

Env reeve reads: `HERDR_SOCKET_PATH` (`herdr.sh:187`, `:601`; `reeve:bin/reeve-lib.sh:681`), `HERDR_WORKSPACE_ID` (`herdr.sh:186`; unset for hands `reeve:bin/reeve-harness:213`), `HERDR_TAB_ID` (`herdr.sh:597`), `HERDR_PANE_ID` (`reeve:bin/reeve-lib.sh:680-681`, the pane key every reeve record holds), `HERDR_ENV` (`reeve:bin/reeve-name:52`), `REEVE_HERDR_SESSION` (`herdr.sh:14`). Backend name `herdr` is also hardcoded at `reeve:bin/reeve-lib.sh:699`, `:709` and is the default in `reeve:bin/reeve-backend:23` and every `meta_get ... backend herdr` (`reeve-dispatch:74`, `reeve-sentry:718`, `:1084`, `reeve-status:169`, `reeve-teardown:338`, `reeve-adopt:116`, `reeve-lib.sh:1318`).

### Drop-in (reeve unchanged) needs all of

1. `herdr` on PATH in every hall pane resolving to hall. **Fails on this Mac**: pane shells are login shells on macOS (`src/pane.rs:1629-1637`, mode Auto is login on macOS), `/etc/zprofile:10-11` runs `path_helper`, which puts `/etc/paths.d/homebrew` (`/opt/homebrew/bin`) first and appends inherited PATH entries after. So a shim dir hall prepends lands behind Homebrew's `herdr` (`which -a herdr`: `/opt/homebrew/bin/herdr`). A shim in `~/.local/bin` (prepended by `~/.zshrc:33`) would win but is global, so it would capture the live herdr's panes too.
2. Outbound `HERDR_SOCKET_PATH`, `HERDR_PANE_ID`, `HERDR_TAB_ID`, `HERDR_WORKSPACE_ID`, `HERDR_ENV` kept in hall panes.
3. Hall's `session list` reporting hall's root as `default: true` (it does, `src/session.rs:187-226`, once P1 moves the root), so `_h_session` resolves hall's own default. Decision 28's "never the session default" then needs to read "never the live default".
4. Same protocol as Homebrew herdr (22), so P8 is off the table.

- Trigger if item 1 fails: reeve runs Homebrew `herdr`; `_h_session` lists the live dirs and returns live `default`; every call carries `--session default`, which beats the pane's `HERDR_SOCKET_PATH` (`src/session.rs:80-83`). `ensure_group` sees the sockets differ (`herdr.sh:187`), skips the reeve's own hall workspace, and creates a new workspace **in the live herdr** (`:197`); hands open there.
- Masking condition: everything succeeds and hands run, only in the wrong multiplexer; the socket-mismatch note goes to stderr (`:182`) and `_h` drops stderr (`:27`).

### Smallest reeve change instead

- **Option R1, a `hall` backend**: `reeve:backends/hall.sh` = `herdr.sh` with `herdr` to `hall` and `HERDR_` to `HALL_` (626 lines, mechanical), plus the shared sites above: `reeve-lib.sh:680-681` (pane key from `HALL_PANE_ID`/`HALL_SOCKET_PATH` when set), `:699`, `:709` (backend of the record, not literal `herdr`), `reeve-name:52`, `reeve-harness:213` (`-u HALL_WORKSPACE_ID` too). About 10 shared lines. Selected by `REEVE_BACKEND=hall` in hall's server env, which every pane inherits (launch §3). Lets hall rename every env (P3 in full) and bump the protocol (P8).
- **Option R2, keep `herdr` names outbound and call `"${HERDR_BIN_PATH:-herdr}"`** in `herdr.sh` (12 call sites). Hall sets `HERDR_BIN_PATH` to itself (`src/integration/env.rs:31` already does). Smaller (one file), and the liege's claude hook keeps working in hall panes. But hall panes then carry `HERDR_SOCKET_PATH`, so any Homebrew `herdr` call in a hall pane without `--session` reaches hall's server; P8 turns that into a loud protocol error.
- Judgement (one line): R1. It is the only option where no `herdr` name in a hall pane can reach either server by accident; R2 is the fallback if the liege wants one reeve file changed instead of a new backend.
- Either is a change to the live reeve, so under decision 9 it waits for the liege, as decision 11 already does for the listing.

## 4. Alerts and sound (decisions 19, 22)

What herdr already has:

- Agent state per pane: working, blocked, idle, done, unknown, from screen manifests (`src/detect/`, 5.5k lines) plus hooks. `agent explain` recomputes from the live screen (reeve uses it, `reeve:backends/herdr.sh:539-544`).
- Sounds, bundled mp3 (`src/sound.rs:27-28`), played with `afplay` (`:120`) by the **client** (`src/client/notifications.rs:9-20`, `:62-73`). The server sends them to the "foreground client", meaning the client currently driving the session (`src/server/headless.rs:209-210`, `:1688-1693`), not the window with OS focus.
- When: blocked plays "request" always; idle after working plays "done" unless the pane is in the active tab and the outer terminal has focus (`src/app/actions.rs:86-110`, `:57-62`; server side `src/server/headless/notifications.rs:137-160`).
- Config: `[ui.sound] enabled`, one `path` or `done_path`/`request_path` mp3, per-agent `on|off` (`src/main.rs:377-389`). Mute: `enabled = false`; `HERDR_DISABLE_SOUND` (`src/sound.rs:18`).
- Toasts: in-app, terminal (OSC), or system notification (`src/main.rs:362-372`; `src/client/notifications.rs:21-33`).
- `notification.show` API and `herdr notification show <title> [--sound none|done|request]` (`src/cli/notification.rs:28`; `src/api/schema/common.rs:104-125`): any process, reeve included, can raise a sound.
- Unseen marker per pane (`seen`, `src/workspace/aggregate.rs:15`, `:50-57`) already ranks "idle and not looked at" above working.

Reuse for decision 19 triggers:

| Trigger | Herdr covers it | Gap |
|---|---|---|
| Reeve session at a dialog | Yes, blocked plays request | A dialog claude paints full screen can read idle with no blocked rule (`reeve:backends/herdr.sh:516-529`); then it plays "done" or nothing, not "request" |
| Reeve session ends its turn (decision 22, every reply) | Yes, done on completion | Suppressed when that reeve's tab is active and the hall window has focus (`src/app/actions.rs:57-62`): arguably right, the liege is looking at it; a config switch if not |
| Hand waiting (dialog) | Yes, same as above, for the hand's pane | as above |
| Hand blocked, idle, wedged, question (reeve file states) | No: herdr does not know them | Reeve calls `hall notification show --sound request`, or hall watches the listing. Small either way |
| Highlight on the reeve | Workspace row shows its highest pane state (`aggregate_state`) | Reeve-state highlight via a pushed token style, or in the sidebar code of section 2 |

- So decision 19's reason ("herdr's own sounds need a herdr window in the foreground") no longer holds: the hall window is that client. Sound still needs a hall client attached, which was also true of the plan's own hall process.

## 5. Repo layout

Hall today: cargo workspace `members = ["crates/*"]`, edition 2024 (`hall:Cargo.toml`), 14 commits, no remote (decision 18). Herdr v0.9.0: 1546 commits, 2503 files, 48.4 MiB packed (`git count-objects -vH`), single package at the root, no `[workspace]`.

Paths both trees have: `.gitignore`, `Cargo.lock`, `Cargo.toml`, `README.md` only (`comm` of both file lists). `docs/` and `scripts/` exist in both with no file in common.

| Option | How | Ports later | Cost |
|---|---|---|---|
| **H1. Herdr at hall's root** | In hall: add remote `upstream`, fetch tags, `git merge --allow-unrelated-histories v0.9.0`; resolve 4 files: herdr's `Cargo.toml` becomes the root package plus `[workspace] members = [".", "crates/*"]`, keep herdr's `Cargo.lock` and add the stand-in's deps, merge `.gitignore`, rewrite `README.md` | `git merge v0.9.x` or cherry-pick at identical paths. v0.9.3 itself adds `[workspace]` with `crates/ghostty-vt` (`git diff v0.9.0 v0.9.3 -- Cargo.toml`), which lands beside `crates/hall-standin` without a path clash | Hall history and docs kept; repo grows ~48 MiB |
| H2. Herdr under a prefix (`crates/hall/` via `git subtree add`) | Herdr's tree in a subdirectory | Every port needs `-Xsubtree` or `git subtree merge`; from v0.9.3 herdr's own `[workspace]` nests inside hall's, which cargo refuses (two workspace roots) | Worst for ports |
| H3. Fresh repo from herdr v0.9.0 | New repo, copy hall docs, stand-in and scripts in | Native | Hall's 14 commits of history stay in the old repo; two repos |

- Remote: `git remote add upstream https://github.com/herdrdev/herdr` then `git remote set-url --push upstream DISABLED` (any non-URL) so a push fails (decision 18). Fetching is outward-facing network read only.
- Toolchain: herdr's `rust-toolchain.toml` (1.96.1) applies to the whole repo under H1; hall's stand-in then builds on 1.96.1 too (it uses edition 2024, supported).
- Agent instructions: herdr ships `AGENTS.md` (305 lines, `CLAUDE.md` links to it) with an "external contributor guardrail" that applies "whenever ... the work is happening in a fork" unless "clearly a private or custom fork" (`AGENTS.md:295-305`), and `build.rs:50-52` prints a cargo warning telling agents to read `CONTRIBUTING.md`. Hands working in hall would read both. Replace `AGENTS.md` with hall's own (keep herdr's as a reference file) and drop that warning line.
- `.github/` (18 files: workflows, MAINTAINERS, APPROVED_CONTRIBUTORS): inert with no remote; delete or keep.

### Licence housekeeping

- Herdr: Apache-2.0 (`LICENSE`; `Cargo.toml:7`). No `NOTICE` file (launch §5). The `LICENSE` appendix is the unfilled template ("Copyright [yyyy] [name of copyright owner]").
- Vendored, MIT: `vendor/libghostty-vt/LICENSE` (plus `vendor/libghostty-vt/pkg/afl++/LICENSE`), `vendor/portable-pty/LICENSE.md`. Both already carry local patches with their own records (`vendor/libghostty-vt.patches.md`, `vendor/portable-pty.patches.md`).
- To do in the fork: keep `LICENSE` and both vendor licences; add a short `NOTICE` or README line "hall is a modified version of herdr (https://github.com/herdrdev/herdr), Apache-2.0"; mark modified files (Apache 4b), e.g. one line in each changed file's header or a `MODIFICATIONS.md` listing them; do not use the herdr name or logo as hall's brand (Apache 6).
- Section 4 obligations bite on redistribution only; while hall stays private and local (decision 18) it is use. Doing the notices from the first commit costs nothing and avoids an audit later. Crate dependency licences were not enumerated (launch §5). Not legal advice.

## 6. Build

- Toolchain: Rust 1.96.1 pinned with clippy and rustfmt (`rust-toolchain.toml`); Zig **0.15.2** exactly (`vendor/libghostty-vt/build.zig.zon:6`; `build.rs:63-91` runs `zig build` on the vendored libghostty-vt, `ZIG` env overrides the binary); `zig build` fetches one non-lazy package from `deps.files.ghostty.org` (`uucode`, `build.zig.zon:40-43`). No other C deps: libghostty-vt's C++ (`src/simd/*.cpp`, simdutf, highway) builds with zig's own clang and libc++. Linked statically on macOS (`build.rs:99-101`).
- Not installed here: `zig` (`which zig`: not found; `brew info zig@0.15`: not installed), toolchain 1.96.1 (only `stable` 1.99, `rustup toolchain list`).
- **Zig 0.15.2 does not link on this Mac as installed.** macOS 26.6.2 with Command Line Tools SDKs 26.5 and 27.0, whose `libSystem.tbd` lists only `arm64e-macos` targets, not `arm64-macos` (`head` of both tbd files). Zig rejects them: `zig build-exe hello.zig` fails with `undefined symbol: _sigaction`, `_waitpid`, ... (official zig 0.15.2 tarball, run in scratchpad). Works when the SDK is not found (zig falls back to its own stubs, which list `arm64-macos`), but libghostty-vt's build insists on the SDK (`LibCInstallation.findNative` -> `DarwinSdkNotFound`, from `src/build/SharedDeps.zig:146`). SDK 27.0 headers then also break zig's libc++ (`clamp_to_integral.h:47: use of undeclared identifier 'INFINITY'`).
- Workaround that built (scratchpad only): a copy of the 26.5 SDK's `usr/lib/*.tbd` with `arm64-macos` added to every target list, served to zig through a fake `xcrun` first on zig's PATH. Homebrew's `zig@0.15` (built on LLVM 20) may behave differently: not tried, installing it writes outside the scratchpad. **Expect the first fork build to need this, or a fixed zig.**
- Measured (scratchpad `CARGO_TARGET_DIR`, `CARGO_HOME`, `RUSTUP_HOME`, zig caches; crates and zig packages already downloaded; zig global cache warm from earlier attempts):
  - libghostty-vt alone: 33 s.
  - `cargo build --release --locked`, clean target: **2 m 10 s** (`Finished release profile ... in 2m 10s`), binary 21 MB. 10-core Mac, 16 GB.
  - `cargo test --locked --no-run` right after (debug profile, libghostty-vt reused from the zig cache): **1 m 24 s**.
  - Both builds print `cargo:warning=external contributor policy ...` (`build.rs:50-52`).
- Test suite on macOS: **3105 tests** by `cargo test --locked -- --list` (lists only, runs nothing): 3001 unit tests in the binary, 104 integration tests (`client_mode` 21, `live_handoff` 20, `server_headless` 15, `detach_reattach` 12, `api_ping` 11, `cross_area` 9, `multi_client` 8, `machine_setup` 5, `broken_pipe` 3). `tests/cli.rs` and `tests/auto_detect.rs` list 0 here: both are `#![cfg(all(unix, not(target_os = "macos")))]` (line 1 and 3), so herdr's CLI integration suite does not run on a Mac at all. Static count: 3161 `#[test]`/`#[tokio::test]` attributes in `src/`, 207 in `tests/` (`git grep -c`), 2 `#[ignore]`. Upstream runs it with `cargo nextest run --locked` plus 11 python contract test modules and docs/asset checks (`justfile:6-13`, `:16-17`). Integration tests start real servers from the built binary, each with a temp `XDG_CONFIG_HOME` and explicit `HERDR_SOCKET_PATH` (e.g. `tests/cli/harness.rs:180-240`, `tests/auto_detect.rs:110-156`): P3's env rename must rename them in the same change, or they would fall back to the real config dir. Not run here (brief: start no server).

## 7. Upstream churn v0.9.0..v0.9.3

- 229 commits, 2026-09-07 to 2026-09-29 (`git rev-list --count`, tag dates). `1226 files changed, 205020 insertions(+), 32708 deletions(-)`.
- Upstream kept moving: `origin/master` in that clone is 70 commits past v0.9.3 (294 past v0.9.0), dated 2026-10-12, still version 0.9.3.

| Area | + | - | Files |
|---|---|---|---|
| `vendor/` (libghostty-vt re-vendored at `44f2a44d`, was `c5a21edf`) | 109368 | 15320 | 633 |
| `docs/` | 32327 | 1188 | 232 |
| `crates/ghostty-vt` (new crate: bindings moved out of `src/ghostty`, `build.rs` moved) | 12082 | 99 | 11 |
| `src/client` | 15707 | 2239 | 74 |
| `src/server` | 6717 | 2361 | 30 |
| `src/platform` | 4310 | 129 | 16 |
| `src/integration` | 3323 | 228 | 27 |
| `src/protocol` | 2775 | 27 | 8 |
| `src/app` | 2706 | 2605 | 21 |
| `src/remote` | 2663 | 288 | 4 |
| `src/pane` + `src/pane.rs` | 3237 | 605 | 10 |
| `tests` | 2392 | 112 | 14 |
| `src/api` | 1290 | 1440 | 13 |
| `src/cli` | 1266 | 137 | 11 |
| `src/detect` | 971 | 881 | 10 |
| `src/config` | 390 | 50 | 7 |
| `src/ui` | 114 | 10 | 4 |

Files hall's patches touch (`git diff --numstat v0.9.0 v0.9.3 -- <file>`):

| File | Patch | Upstream churn |
|---|---|---|
| `src/config/io.rs` | P1 | +92 -2 (BOM handling; `app_dir_name` unchanged) |
| `src/session.rs` | P3 | +29 -2 |
| `src/server/socket_paths.rs`, `src/detect/manifest_update.rs` | P3, P4 | none |
| `src/api/mod.rs` | P3 | +1 -8 |
| `src/main.rs` | P3, P4, P6 | +50 -21 |
| `src/app/mod.rs` | P4 | +65 -30 |
| `src/update.rs` | P4 | +223 -20 |
| `src/remote/attach.rs` | P5 | +2320 -252 |
| `src/pane_graphics_files.rs` | P6 | +0 -529 (moved into `crates/ghostty-vt/src/pane_graphics_files.rs`) |
| `src/config/model.rs` | P4, P6 | +48 -8 |
| `src/protocol/wire.rs` | P8 | +217 -13 (`PROTOCOL_VERSION` still 22 at v0.9.3, launch §6) |
| `src/client/shell/sidebar.rs` | section 2 | +77 -39 |
| `src/client/shell/agent_sidebar.rs` | section 2 | +76 -77 |
| `src/client/shell/state.rs` | section 2 | +216 -129 |

- Judgement: the rename patches sit in quiet files and port cheaply. The reeve-tree work sits in the client shell, the most active `src/` area; keep it in few, new functions to limit conflicts. Vendor and the crate split are the large, mechanical part of any port past v0.9.0; take them whole, never by hand.

## 8. Hall step 1 in a fork

| Piece | Status | Why |
|---|---|---|
| Stand-in and draft listing (`hall:crates/hall-standin`, `docs/listing-draft.md`) | **Keeps** | Hall still needs reeve's reconciled state (ADR 0002); `herdr_session` becomes a hall session name. Its home checks (`~/.reeve` refused etc.) are about reeve, unchanged |
| Test reeve home `~/.reeve-hall-dev`, `REEVE_HOME` on every test command line (decision 26) | **Keeps** | Reeve's plugin hooks still run in every claude session, in hall panes too |
| Test setup scripts | **Keep, retarget** | Every herdr call goes through one array, `HALL_HERDR=(env -u HERDR_SOCKET_PATH -u HERDR_SESSION herdr)` (`hall:scripts/test-setup-lib.sh:19`), plus `command -v herdr` (`hall:scripts/test-setup:94`). Pointing them at the hall binary is a few lines, plus the fake binary name in `tests/scripts_fake_herdr.rs` |
| `hall-dev`, `hall-dev-2` sessions | **Keep as hall sessions** | Two hall servers still show decision 6 across sessions; under P1 they live in `~/.config/hall/sessions/`, not in herdr's list |
| "Never the session default" guards, live baseline from `herdr session list` (decision 28; `test-setup-lib.sh:106`) | **Partly moot** | Once hall has its own dirs (P1), hall's `default` is not the live one and hall never sees herdr's sessions. The live-setup check becomes: herdr's own dirs, sockets and `herdr session list` unchanged, and no hall process holding a herdr socket |
| `env -u HERDR_SOCKET_PATH -u HERDR_SESSION` | **Keeps value only until P3** | After P3 hall ignores those names; it then needs `env -u HALL_*` instead when run from inside a hall pane |
| Plan steps 3 and 4 (`observe`/`control` through herdr's CLI, ANSI to cells, `vt100`/`alacritty_terminal`) | **Moot** | Hall is the multiplexer: herdr's own client already renders panes with libghostty-vt and handles input |
| Plan open questions "frames and input", "control and size", "literal ctrl+b", "synchronised output", "which sessions to attach" (`hall:docs/plan.md:146-152`) | **Moot** | Same reason; herdr's direct-attach answers (prefix twice) come with it |
| Research of herdr's documented API, "off limits: anything not documented" (`hall:docs/plan.md:51`) | **Moot as a rule** | Hall owns the code; undocumented internals are now its own |
| Turn-end signal question (`hall:docs/plan.md:148`) | **Changes shape** | Herdr's completion sound already fires on turn end (section 4); the question becomes whether its idle and done reads are good enough, the latch issue reeve hit (`reeve:backends/herdr.sh:539-542`) included |

## Plan impact

- ADR 0001 (own app on herdr): superseded, as decision 31 says. Its consequences ("turn ANSI frames into a picture", "documented API only") lapse.
- ADR 0003 (separate from the live setup): holds, but the mechanism changes from "name the session" to "own dirs, own env names, own binary, own protocol" (P1, P2, P3, P8). Its `HERDR_CONFIG_PATH` advice lapses.
- ADR 0002 (state through reeve): holds. New fact: reeve can push reconciled state as metadata tokens, which hall renders with no code.
- Decision 13: already replaced by 30; 30's "pinned copy" replaced by 31.
- Decision 14 (prefix ctrl+b): herdr's default prefix, comes free.
- Decision 15 (typing control): moot, hall is the multiplexer; focus works as in herdr.
- Decision 19: "hall plays the sound itself, because herdr's own sounds need a herdr window in the foreground" no longer holds; herdr's sounds play in the hall window (section 4). Hand-file triggers still need a small addition.
- Decision 22: met by herdr's done-on-completion sound, except the focused active tab case.
- Decision 24: rewrite for the fork (no `HERDR_SESSION`/`HERDR_SOCKET_PATH` reliance becomes no `HALL_*` reliance; no herdr call at all from hall's test setup).
- Decision 27 (herdr stays 0.9.0): becomes "hall forks at 0.9.0"; the live Homebrew herdr is free to upgrade without breaking hall once P1 to P8 land.
- Decision 28: its wording "the session default" needs "the live herdr's default" once hall has its own default.
- Plan: steps 3 and 4 drop; step 2 becomes "sidebar shows reeves with hands beneath" (section 2, option A); step 5 shrinks (section 4). A new first step: the rename and separation patch set (section 1) plus the build fix (section 6).

## Open for the liege

1. **Reeve's side of the bridge (section 3).** Reeve cannot reach hall unchanged on this Mac. R1: a `hall` backend in reeve (new file plus about 10 shared lines). R2: herdr backend calls `$HERDR_BIN_PATH` (one file). Recommend **R1**, landed when hall is ready for the live setup (with the listing change, decision 11). Until then hall's test reeves do not dispatch.
2. **Reeve tree shape (section 2).** A: reeve = workspace, hand = tab (matches reeve today, client-only change). B: new grouping level, hands as workspaces (needs a reeve change). Recommend **A**.
3. **Repo layout (section 5).** H1: herdr's history merged at hall's root. H3: a fresh repo. Recommend **H1**, with `upstream` as a fetch-only remote.
4. **How hall learns hand state (section 2).** Keep the plan's listing that hall reads, or have reeve push tokens that hall shows with no code. Recommend **reeve pushes tokens** for marks and badge, and keep the listing for the tree's membership. Both need reeve changes the liege approves.

## Not checked

- No fork patch was written or compiled; sizes are estimates from the code read.
- The test suite was built only as far as the build line above says; no test was run (they start servers).
- Homebrew `zig@0.15` on this SDK; a zig newer than 0.15.2 against herdr's `build.zig`.
- Whether reeve's hooks or skills call the backend in more places than `bin/` greps show (skills were not read).
- The `KERN_PROCARGS2` env route; the zero-code agents-panel view; whether `agent.view.set` can show a reeve grouping.
- Crate dependency licences.
- One slip, fixed: one zig test call ran without the scratchpad cache env and created `~/.cache/zig` (born 20:16:03, all 841 files newer than the run); it was removed at once. `~/.cache` itself predates the run (born 2026-06-15) and was left alone.
