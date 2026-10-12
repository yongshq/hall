# Modifications to herdr

Hall is a modified version of herdr v0.9.0 (commit b99002ac99b09e00b4ca692436cb15a6b0d676f1), Apache-2.0; see [NOTICE](NOTICE). Every herdr file hall changes is listed here, kept current with each change. Files hall adds (`crates/`, `scripts/`, hall's own `docs/*.md`, `docs/adr/`, `docs/research/`, `GLOSSARY.md`, `NOTICE`, this file and `tests/hall_separation.rs`) are hall's own and not listed.

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
