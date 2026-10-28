# Modifications to herdr

Hall is a modified version of herdr v0.9.0 (commit b99002ac99b09e00b4ca692436cb15a6b0d676f1), Apache-2.0; see [NOTICE](NOTICE). Every herdr file hall changes is listed here, kept current with each change. Files hall adds (`crates/`, `scripts/`, hall's own `docs/*.md`, `docs/adr/`, `docs/research/`, `GLOSSARY.md`, `NOTICE`, this file, `tests/hall_separation.rs`, `tests/hall_stop_guard.rs`, `tests/fixtures/hall-endpoint-welcome-1001.json`, `src/client/reeve_listing.rs`, `src/client/shell/reeve_tree.rs`, `src/client/shell/reeve_alerts.rs`, `src/client/shell/tests/reeve_tree.rs` and `src/client/shell/tests/reeve_alerts.rs`) are hall's own and not listed.

## Repository and build

| File | Change |
|---|---|
| `Cargo.toml` | Package renamed `hall` (binary `hall`), description, `publish = false`; `[workspace]` with `crates/*` beside the root package. |
| `Cargo.lock` | Package renamed; the stand-in's dependencies added. |
| `.gitignore` | Hall's docs let through herdr's `/docs/*` rule. |
| `.cargo/config.toml` | `RUST_TEST_THREADS=1`: herdr's tests share process-wide state and run upstream under nextest. |
| `build.rs` | External-contributor cargo warning dropped. |
| `README.md` | Replaced by hall's README. |
| `AGENTS.md` | Replaced by hall's; herdr's kept unchanged at `docs/herdr/AGENTS.md` (`CLAUDE.md` still links to `AGENTS.md`). |
| `docs/next/api/herdr-api.schema.json` | Generated schema's protocol number, 1022. |

## Separation (decision 33)

| File | Change |
|---|---|
| `src/config/io.rs` | P1: config and state folders `hall` (release) and `hall-dev` (debug). |
| `src/api/mod.rs` | P3: inbound `HALL_SOCKET_PATH`; `PANE_SOCKET_PATH_ENV_VAR` keeps `HERDR_SOCKET_PATH` for what hall sets into panes. |
| `src/server/socket_paths.rs` | P3: inbound `HALL_CLIENT_SOCKET_PATH`. |
| `src/session.rs` | P3: inbound `HALL_SESSION`; one test's expected guidance text. |
| `src/config.rs` | P3: inbound `HALL_CONFIG_PATH`. |
| `src/main.rs` | P3: the nested guard reads `HALL_ENV` (panes still get `HERDR_ENV`); P4: `hall update` refused, default config text; P5: `--remote` refused; P6: default worktree folder text; `--version` and the help header say hall. |
| `src/integration/env.rs` | P3: panes get `HERDR_SOCKET_PATH` through the outbound constant. |
| `src/app/custom_commands.rs` | P3: custom commands get `HERDR_SOCKET_PATH` through the outbound constant; a test's scratch file is `hall-`. |
| `src/app/api/plugins/panes.rs` | P3: plugin panes get `HERDR_SOCKET_PATH` through the outbound constant. |
| `src/app/api/plugins/runtime.rs` | P3: plugin commands get `HERDR_SOCKET_PATH` through the outbound constant. |
| `src/cli/plugin.rs` | P3: plugin builds scrub both the inbound `HALL_*` and the outbound names. |
| `src/server/autodetect.rs` | P3: the client socket name is the inbound constant. |
| `src/app/mod.rs` | P4: `auto_updates_enabled` is always false. |
| `src/config/model.rs` | P4: `version_check` and `manifest_check` default false; P6: worktree folder `~/.hall/worktrees` (`~/.hall-dev/worktrees` in a debug build); tests to match. |
| `src/update.rs` | P4: `self_update` and the background check refused before any fetch. |
| `src/detect/manifest_update.rs` | P4: agent manifests are never fetched from herdr.dev (only `file://` catalogs, which herdr's tests use). |
| `src/cli/server.rs` | P4: `hall server update-agent-manifests` refused. |
| `src/remote/attach.rs` | P5: herdr's release download and manifest fetch refused; a captured status in a test carries generation 1001. |
| `src/pane_graphics_files.rs` | P6: pane-graphics folder `hall-pane-graphics-<uid>`; a test's scratch folder is `hall-`. |
| `src/server/clipboard_image.rs` | Clipboard image staging folder `hall-clipboard-images-<uid>`, so hall never prunes the live herdr's. |
| `src/cli/integration.rs` | P7: `hall integration install` and `uninstall` refused. |
| `src/integration/mod.rs` | P7: `HALL_REFUSES_INSTALL`. |
| `src/app/api/integrations.rs` | P7: the API's integration install and uninstall refused. |
| `src/protocol/wire.rs` | P8: protocol number 1022. |
| `src/protocol/endpoint.rs` | Endpoint protocol generation 1001, so the client handshake refuses herdr's client and herdr's server refuses hall's; the frozen generation-one fixture test checks the fixture's own 1. |
| `src/cli/spec.rs` | Clap command name `hall`, its about line and subcommand usage lines; tests to match. |
| `src/cli/completion.rs` | Completion scripts are generated for `hall`. |

## Tests

| File | Change |
|---|---|
| `src/platform/unix_common.rs` | Under `cfg(test)`, CLI output leaves SIGPIPE ignored, so one test's print cannot kill the shared unit-test process. |
| `src/app/tab_bar_status.rs` | A test's scratch files are `hall-`. |
| `tests/support/mod.rs` | Protocol 1022, endpoint generation 1001; reads `HALL_SOCKET_PATH`. |
| `tests/server_headless.rs` | As below; a test that a hall server rejects herdr's endpoint generation 1. |
| `tests/api_ping.rs`, `tests/auto_detect.rs`, `tests/broken_pipe.rs`, `tests/client_mode.rs`, `tests/cross_area.rs`, `tests/detach_reattach.rs`, `tests/multi_client.rs`, `tests/cli/agents.rs`, `tests/cli/harness.rs`, `tests/cli/plugins.rs`, `tests/cli/sessions.rs`, `tests/cli/surface.rs` | Binary `hall`, inbound `HALL_*` names, `hall-dev` folders (`hall` in a release build), protocol 1022. |
| `tests/live_handoff.rs` | As above; two tests ignored on macOS, where they fail by platform behaviour (SO_RCVTIMEO after the peer closes; another process's environment of `/bin/sleep`). |
| `tests/machine_setup.rs` | As above; the explicit `--remote` handoff test now checks that `--remote` is refused. |

## Step 1 repair (decision 44)

| File | Change |
|---|---|
| `src/session.rs` | 44a: restart and attach guidance names `hall` (`hall server stop`, `hall session stop <name>`, `hall session attach <name>`, `hall`), the socket-override form included; usage lines; tests to match. |
| `src/main.rs` | 44a: help usage and common commands, usage errors and the nested-guard message name `hall`; the default config's header and worktree folder come from `app_dir_name()` (`hall` or `hall-dev`); 44b: a test that `HALL_ENV` refuses and `HERDR_ENV` alone does not; a default-config test. |
| `src/config.rs` | `app_dir_name()` no longer test-only, for the default config text. |
| `src/config/model.rs` | The worktree-folder test asserts `app_dir_name()`'s folder, so it passes in a release build. |
| `src/cli.rs`, `src/cli/agent.rs`, `src/cli/api.rs`, `src/cli/completion.rs`, `src/cli/integration.rs`, `src/cli/machine.rs`, `src/cli/notification.rs`, `src/cli/server.rs`, `src/cli/server_not_running.rs`, `src/cli/spec.rs`, `src/cli/status.rs`, `src/cli/tab.rs`, `src/cli/workspace.rs`, `src/cli/worktree.rs` | 44a: usage, help and guidance text names `hall` commands, the server-not-running guidance's socket-override form included; tests to match (in `tests/cli/sessions.rs` too). |
| `src/cli/pane.rs` | 44a as above; 44b: the default pane target reads `HALL_PANE_ID` only, never `HERDR_PANE_ID`; a test. |
| `src/cli/plugin.rs` | 44a as above; 44b: plugin builds also scrub `HALL_PANE_ID`, `HALL_TAB_ID` and `HALL_WORKSPACE_ID`. |
| `src/pane.rs` | 44b: panes get `HALL_ENV=1`, `HALL_PANE_ID`, `HALL_TAB_ID` and `HALL_WORKSPACE_ID` beside herdr's names; tests. |
| `src/integration/env.rs`, `src/integration/mod.rs` | 44b: the `HALL_*` pane id names. |
| `src/app/api/plugins/runtime.rs`, `src/app/api/plugins/panes.rs` | 44b: plugin commands and plugin panes get the `HALL_*` names beside herdr's; `HALL_ENV` is protected in plugin panes. |
| `src/integration/registry.rs` | 44a: the integration update notice names `hall integration install`. |
| `src/config/io.rs`, `src/app/mod.rs`, `src/app/actions.rs`, `src/app/state.rs`, `src/server/autodetect.rs`, `src/server/client_shell.rs`, `src/server/headless/bootstrap.rs`, `src/server/socket_paths.rs`, `src/client/errors.rs`, `src/protocol/wire.rs`, `src/remote.rs`, `src/remote/saved.rs`, `src/remote/attach.rs`, `src/update.rs`, `src/ui/release_notes.rs` | 44a: toasts, errors and guidance that give a command name `hall` (`hall config check`, `hall update`, `hall server`, `hall --remote`); tests to match. |
| `src/protocol/endpoint.rs` | 44d: a test of hall's own frozen welcome at generation 1001, beside herdr's generation-one fixture. |
| `src/client/shell/tests/startup_overlays.rs` | A test that the onboarding write ignores `HERDR_CONFIG_PATH` and follows `HALL_CONFIG_PATH`; command text as 44a. |
| `src/client/shell/tests/mod.rs`, `src/client/tests/mod.rs`, `src/server/headless/tests/mod.rs`, `src/server/headless/tests/pane_graphics.rs`, `tests/cli/workspace.rs` | Expected command text as 44a. |
| `skills/herdr/SKILL.md` | 44a and 44b: the skill `hall --skill` prints gives `hall` commands and the `HALL_*` pane names. |
| `tests/support/mod.rs` | Server cleanup works on macOS (process list, argv and environment from `proc_listallpids` and `KERN_PROCARGS2`, as `/proc` gives them on Linux) and matches the `hall` binary, so no test server or `server --handoff-import` outlives a test or writes back into a removed `/tmp/hlh-*` folder; tests. |

## Step 1 second repair (decision 45)

| File | Change |
|---|---|
| `src/api/status.rs`, `src/api/mod.rs` | `HALL_NUMBER_FLOOR` (1000) and `RuntimeStatus::is_hall_server`: a server is hall's only when its protocol and endpoint generation are both at or above the floor; a test. |
| `src/session.rs` | `ensure_hall_server_at` pings first and refuses a server that is not hall's; `hall server stop` and `hall session stop` call it before `server.stop`; `not_hall_server_message`, which gives no stop command; stop tests answer ping as a hall server. |
| `src/cli/server.rs` | `hall server live-handoff` refuses a server that is not hall's before sending the handoff. |
| `src/cli.rs`, `src/cli/protocol_guard.rs` | A CLI request to a server that is not hall's prints `not_hall_server` instead of restart guidance; the agent help footer drops herdr.dev's agent guide and llms.txt. |
| `src/server/autodetect.rs` | The client launch says a server that is not hall's, or whose status is unavailable, is not one hall will attach to, stop or hand off, with no stop command; tests. |
| `src/update.rs` | The (refused) update flow's stop and handoff also refuse a server that is not hall's. |
| `tests/cli/surface.rs` | The help test asserts no herdr.dev link. |

## Step 2 (decision 46)

| File | Change |
|---|---|
| `src/cli/status.rs` | `hall status` says `hall_server: no` and why for a server that is not hall's (`hall_server` in the JSON), and gives `restart_needed` and `server_binary_stale` no value for it instead of restart guidance; tests. It bounds its status request (2 seconds) and says of a listener that does not answer it what the client launch says, instead of hanging; `tests/hall_stop_guard.rs` tests it. |
| `src/server/autodetect.rs` | The client launch says hall cannot confirm a server is hall's when the server does not answer its status check (a silent listener, `WouldBlock` on macOS), instead of the raw error; a test. That message is `unanswered_status_message`, and `STATUS_REQUEST_TIMEOUT` is crate-visible, both shared with `hall status` and every CLI request's protocol ping. |
| `src/api/client.rs` | `ApiClient::status_with_timeout`: `status` with its wait bounded; `status` and it share the ping request and the reply's reading. Once every caller bounds its ping, `status` and `request`, which nothing else used, are removed. |
| `src/cli.rs` | Every CLI request's protocol ping (`ensure_server_protocol_compatible`) is bounded as `hall status`'s is, and a listener that does not answer it gets the same message, instead of hanging; the request itself stays unbounded (`agent wait`). `tests/hall_stop_guard.rs` tests `pane list` and `workspace list`. |
| `src/main.rs` | `main` runs the old body as `run` and prints an error of kind `Other` as its message (`Error: <message>`), not as Rust's `Debug` of the `io::Error`; exit status 1 as before. |

## Step 3: reeve tree (decisions 49 to 60)

| File | Change |
|---|---|
| `src/config/model.rs` | `[reeve]` section (`ReeveConfig`): `listing_command`, `listing_interval_ms` and `listing_timeout_ms`. |
| `src/config.rs` | Exports `ReeveConfig`. |
| `src/config/io.rs` | `reeve` is a known section and reloads live like the others: on reload the client restarts the listing reader with the new command, or stops it and goes back to herdr's sidebar when the command is removed. |
| `src/client/mod.rs` | The `reeve_listing` module; the client loop keeps the listing reader in step with the shell's config (started, restarted on a changed `[reeve]`, stopped when the command goes, and dropped with the loop, which kills and reaps a running listing command on detach), and redraws on each changed read of the current reader. |
| `src/client/events.rs` | `ClientLoopEvent::ReeveListing`, tagged with the reader's generation. |
| `src/client/shell.rs` | The `reeve_tree` module. |
| `src/client/shell/state.rs` | `ClientShellConfig::reeve_tree_session` and `reeve_listing`; `ClientShellLayout::key_help`, kept only while the tree is drawn (one machine); `ShellHitMap::reeve_tabs`; the shell keeps the last good listing, why the latest read failed, and the tree item a navigate walk has selected. |
| `src/client/shell/config.rs` | `from_config` and `apply_live_config` turn the reeve tree on for the client's session when a listing command is configured, off when none is; a reload with a changed command forgets the old command's reads, and a changed interval or timeout keeps them; `layout` keeps the bottom row for the key help line while the tree is shown. |
| `src/client/shell/render.rs` | `ShellRenderState` carries the tree and its selection. |
| `src/client/shell/sidebar.rs` | Herdr's spaces section moved, unchanged, into `render_spaces_section`; the reeve tree is drawn in its place when it is on. |
| `src/client/shell/composition.rs` | The tree is built for each frame; the key help line is drawn. |
| `src/client/shell/input.rs` | In navigate mode ↑ and ↓ walk the tree, enter picks its item and the digits follow the tree's order when the tree is on. |
| `src/client/shell/overlay_input.rs` | `workspace_action_id`: in the tree walk, a hand's row, a tab's row or a pane-less reeve's row names no workspace, so workspace actions do nothing there. |
| `src/client/shell/mouse.rs` | A click on a tab row of the reeve tree focuses that tab. |
| `src/client/shell/actions.rs` | Opening navigate mode starts the tree walk at the focused item. |
| `src/client/shell/tests/mod.rs` | The `reeve_tree` tests. |
| `README.md` | Status line for step 3. |
| `docs/next/website/src/data/config-reference.json` | The `[reeve]` keys, so `scripts/config_reference_check.py` stays in sync. |

## Step 4: alerts (decisions 61 to 65)

| File | Change |
|---|---|
| `src/detect/manifests/claude.toml` | Rule `numbered_choice_overlay` (blocked, priority 845): numbered choices with the cursor on one and no composer, a consent whose footer no other rule knows (Claude in Chrome's `2. Deny (esc)`), with one choice that answers a consent (yes, no, allow, deny, ...) so Claude's own numbered menus stay idle; manifest version `2026.10.14.2`. |
| `src/detect/manifest/tests.rs` | The Chrome consent reads blocked by that rule; a numbered list or the quoted consent above a visible composer stays idle; Claude's rewind menu stays idle. |
| `src/config/model.rs` | `[reeve] alert_focused_reply`, default true. |
| `src/client/shell.rs` | The `reeve_alerts` module. |
| `src/client/shell/state.rs` | `ClientShellConfig::reeve_alert_focused_reply`; the shell keeps the alerts standing on the reeve tree. |
| `src/client/shell/config.rs` | `apply_reeve_config` takes `alert_focused_reply`, at startup and on reload. |
| `src/client/shell/notification_policy.rs` | For a pane the listing names: herdr's done for a hand's pane is dropped, toast and all; with `alert_focused_reply` a reeve's done is current while its pane reads idle and its sound is not skipped for the focused active tab; every alert that passes raises the reeve tree's highlight, apart from the sound; a dialog on such a pane waits for the snapshot to read it blocked whatever the toast delay. |
| `src/client/shell/composition.rs` | Alerts whose trigger has cleared, or whose pane the focus moved onto, are cleared before each frame. |
| `src/client/shell/mouse.rs` | A click on an agent row (the reeve tree's reeve and hand rows among them) clears that pane's alert. |
| `src/client/shell/input.rs` | When the hall window gets focus back, a reeve's reply highlight on the pane in view clears, as herdr counts it seen. |
| `src/client/mod.rs` | A read of the listing gives the sound for hands newly stuck or asking, played through the shell's notification effects. |
| `src/client/notifications.rs` | Every sound the shell asks for is logged at info (`sound.play`, played or muted). |
| `src/client/shell/tests/mod.rs` | The `reeve_alerts` tests. |
| `README.md` | Status line for step 4. |
| `docs/next/website/src/data/config-reference.json` | `reeve.alert_focused_reply`. |

## Step 5: bridge to reeve (decisions 66 to 68)

| File | Change |
|---|---|
| `src/integration/env.rs`, `src/integration/mod.rs` | `HALL_SOCKET_PATH` (this hall's socket), `HALL_BIN_PATH` (the running binary) and `REEVE_BACKEND=hall`, set into every pane after its launch's own env; herdr's outbound names kept; the names' constants exported for the plugin build scrub. |
| `src/pane.rs` | Test: a pane gets those three over any `--env` extra, and keeps `HERDR_SOCKET_PATH` and `HERDR_BIN_PATH`. |
| `src/app/api/plugins/runtime.rs` | Plugin commands get the same three names. |
| `src/app/api/plugins/panes.rs` | Plugin panes get the same three names, protected from the plugin's own env. |
| `src/app/api/plugins/mod.rs` | Tests: a plugin pane's env cannot override the three names; a plugin command gets them. |
| `src/client/shell/notification_policy.rs` | Every herdr request this client rings for is remembered while its pane reads blocked, the listing naming the pane or not, so a hand's later `waiting` on that dialog does not ring it again (window B); `COMPLETION_EVIDENCE_GRACE` shared with the alerts. |
| `src/config/model.rs` | The `[reeve] listing_command` comment names `docs/listing.md`, the settled listing doc. |
| `src/app/custom_commands.rs` | Shell custom commands and tab bar status commands (`custom_command_env`) get the same three names; test. |
| `src/cli/plugin.rs` | Plugin builds also scrub `HALL_BIN_PATH` and `REEVE_BACKEND`; test. |
| `src/logging.rs` | Default log filter `hall=warn` (herdr's `herdr=info` matches no target in the `hall` crate), so hall's warns (a dropped listing entry) are written by default and no info line is; tests. |
| `README.md` | Status line for step 5, hall's side. |

## Hand panes not restored (the liege's choice A, 2026-10-14)

| File | Change |
|---|---|
| `src/terminal/state.rs` | `TerminalState::no_agent_resume`, the `--no-agent-resume` mark. |
| `src/persist/snapshot.rs` | `PaneSnapshot::no_agent_resume`, saved only when set and read as false from older files; tests of the round trip and of an older file. |
| `src/persist/restore.rs` | `drop_no_resume_panes`: before a cold restore, marked panes are dropped from the snapshot (and its pane history), then tabs and workspaces left empty; the active tab, active and selected workspace move to the nearest one kept, and the dropped tab and workspace ids stay reserved. Restored terminals keep the mark (a live handoff, which keeps every pane). Tests: the pruning, an unmarked session left alone, and a restore that drops the hands and still resumes the reeve's claude. |
| `src/persist.rs` | Exports `drop_no_resume_panes`. |
| `src/workspace.rs` | `reserve_public_workspace_ids`, which `reserve_workspace_ids` now calls, for the ids of dropped workspaces. |
| `src/app/mod.rs` | The cold restore calls `drop_no_resume_panes` first; the live handoff does not. |
| `src/app/creation.rs` | `mark_root_pane_no_agent_resume`; `PaneInfo::agent_resume` from the mark. |
| `src/app/api/tabs.rs`, `src/app/api/workspaces.rs`, `src/app/api/worktrees.rs` | `tab.create`, `workspace.create` and `worktree.open` mark the root pane they create when asked: for `worktree.open`, that of each workspace it opens (the worktree's, and the repo parent's when it opens that too), never one it finds already open; tests; test params gain the field. |
| `src/api/schema.rs`, `src/api/schema/tabs.rs`, `src/api/schema/workspaces.rs`, `src/api/schema/worktrees.rs`, `src/api/schema/panes.rs` | `no_agent_resume` on the three create params, left out when false; `PaneInfo::agent_resume`, shown only when false. |
| `src/api/schema/tests.rs`, `src/api/server.rs`, `src/api/subscriptions.rs`, `src/client/shell/tests/mod.rs`, `src/server/headless/tests/mod.rs` | Test values gain the new fields. |
| `src/client/shell/actions.rs`, `src/client/shell/context_menu.rs`, `src/client/shell/overlay_input.rs`, `src/client/shell/worktrees.rs` | The client's own creates send `no_agent_resume: false`. |
| `src/cli/tab.rs`, `src/cli/workspace.rs`, `src/cli/worktree.rs` | `--no-agent-resume` on `tab create`, `workspace create` and `worktree open`; each parse split out (`parse_*_args`, as `src/cli/pane.rs` has them) and tested; usage lines. |
| `src/cli/spec.rs` | The flag in the three commands' help and completions. |
| `docs/next/api/herdr-api.schema.json` | Regenerated with the new fields. |
| `docs/next/website/src/content/docs/cli-reference.mdx` | The flag on the three commands, and what a restore does with a marked pane. |
| `tests/fixtures/endpoint-method-shapes-v1.json` | New digests for `tab.create`, `workspace.create` and `worktree.open`: the added `no_agent_resume` is optional, defaults to false and is left off the wire when false (the client never sets it), so an older client or server reads the same requests. |

## Agent row like the liege's herdr (2026-10-15)

| File | Change |
|---|---|
| `src/ui/sidebar/tokens.rs` | In an agent row, a built-in value that repeats in the row (ignoring case and surrounding space) shows once, so a reeve's workspace and tab of the same name read `Bran`, not `Bran · Bran`: the first repeat with an inline style stays, and a styled token, a custom `$name` token or the machine is never dropped. An unnamed tab shows the workspace's name when the rows show no `workspace`: a workspace's lone tab as `hall`, one of several, labelled by its number, as `hall 1`. `terminal_title_stripped` is left out while the pane has no title, as in herdr; tests. |
| `src/config/sidebar.rs` | The default agent rows are the liege's herdr config with `machine` before the tab, which shows only while more than one machine is connected or still shows cached agents (reconnecting or needing attention), `[["state_icon", "machine", { token = "tab", bold = true, dim = false }], ["terminal_title_stripped"]]`, instead of herdr's `[["state_icon", "machine", "workspace", "tab"], ["agent"]]`; test to match. |
| `src/main.rs` | The default config text's agent rows. |
| `src/client/shell/tests/agents_worktrees_notifications.rs` | A reeve's agent row renders as `○ Bran` over its title and alone with no title, a lone unnamed tab as its workspace, one of several unnamed tabs as its workspace and number, and a hand's row as `○ artificer-agent-row` over its title, the name bold and not dimmed. |
| `src/server/headless/tests/mod.rs` | The test of a title change that needs no sidebar render sets agent rows that show no title, now that the default rows do. |
| `src/client/shell/agent_sidebar.rs` | Tells the agent row tokens whether the tab is unnamed, so a numbered tab can show the workspace's name. |
| `src/client/shell/tests/endpoints.rs` | With two machines connected, the default agent rows name each row's machine, also a machine named like its tab; with a second machine saved but connecting or disabled, they name none, and a reconnecting machine's cached rows keep its name. |
| `src/client/shell/endpoint_agents.rs` | The agent rows of the machines' list name the machine only while more than one machine is connected or still shows cached agents (reconnecting or needing attention), so such a machine keeps its name. |
| `docs/next/website/src/content/docs/configuration.mdx`, `docs/next/website/src/content/docs/agents.mdx` | The new default rows, the unnamed tab's fallback, the missing title, the repeated value, and the `machine` token shown only while more than one machine is connected or still shows cached agents (reconnecting or needing attention). |
| `docs/next/website/src/data/config-reference.json` | The default of `ui.sidebar.agents.rows`. |
| `docs/next/website/src/content/docs/connecting-machines.mdx` | The `machine` token shows while more than one machine is connected or still shows cached agents (reconnecting or needing attention). |

## The walk's selection on a hand whose pane closes (2026-10-28)

| File | Change |
|---|---|
| `src/client/shell/state.rs` | Taking a snapshot passes the reeve tree walk's selection on when the snapshot takes its row away, as a listing read does: a working hand whose pane closes changes no read, so the selection moves to its reeve's row here. Only while the walk is on, so a selection left from a walk ended by esc costs a snapshot no tree builds. |
