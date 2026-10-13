# Modifications to herdr

Hall is a modified version of herdr v0.9.0 (commit b99002ac99b09e00b4ca692436cb15a6b0d676f1), Apache-2.0; see [NOTICE](NOTICE). Every herdr file hall changes is listed here, kept current with each change. Files hall adds (`crates/`, `scripts/`, hall's own `docs/*.md`, `docs/adr/`, `docs/research/`, `GLOSSARY.md`, `NOTICE`, this file, `tests/hall_separation.rs`, `tests/hall_stop_guard.rs` and `tests/fixtures/hall-endpoint-welcome-1001.json`) are hall's own and not listed.

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
| `tests/api_ping.rs`, `tests/auto_detect.rs`, `tests/broken_pipe.rs`, `tests/client_mode.rs`, `tests/cross_area.rs`, `tests/detach_reattach.rs`, `tests/multi_client.rs`, `tests/cli/agents.rs`, `tests/cli/harness.rs`, `tests/cli/plugins.rs`, `tests/cli/sessions.rs`, `tests/cli/surface.rs` | Binary `hall`, inbound `HALL_*` names, `hall-dev` folders, protocol 1022. |
| `tests/live_handoff.rs` | As above; two tests ignored on macOS, where they fail by platform behaviour (SO_RCVTIMEO after the peer closes; another process's environment of `/bin/sleep`). |
| `tests/machine_setup.rs` | As above; the explicit `--remote` handoff test now checks that `--remote` is refused. |

## Step 1 repair (decision 44)

| File | Change |
|---|---|
| `src/session.rs` | 44a: restart and attach guidance names `hall` (`hall server stop`, `hall session stop <name>`, `hall session attach <name>`, `hall`), the socket-override form included; usage lines; tests to match. |
| `src/main.rs` | 44a: help usage and common commands, usage errors and the nested-guard message name `hall`; the default config's header and worktree folder come from `app_dir_name()` (`hall` or `hall-dev`); 44b: a test that `HALL_ENV` refuses and `HERDR_ENV` alone does not; a default-config test. |
| `src/config.rs` | `app_dir_name()` no longer test-only, for the default config text. |
| `src/config/model.rs` | The worktree-folder test asserts `app_dir_name()`'s folder, so it passes in a release build. |
| `src/cli.rs`, `src/cli/agent.rs`, `src/cli/api.rs`, `src/cli/completion.rs`, `src/cli/integration.rs`, `src/cli/machine.rs`, `src/cli/notification.rs`, `src/cli/server.rs`, `src/cli/server_not_running.rs`, `src/cli/spec.rs`, `src/cli/status.rs`, `src/cli/tab.rs`, `src/cli/workspace.rs`, `src/cli/worktree.rs` | 44a: usage, help and guidance text names `hall` commands; tests to match. |
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
