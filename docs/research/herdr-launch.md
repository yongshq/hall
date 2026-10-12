_Source: the herdr-launch errand report (`~/.reeve/errands/herdr-launch/report.md`), written 2026-10-12, copied into hall unchanged below this line._
# herdr-launch: how `hall` starts and reattaches to a hidden herdr

Source: herdr checked out at `v0.9.0` (`git describe --tags` = v0.9.0) in `~/.reeve/errands/herdr-launch/src/herdr`. Paths below are relative to it, at v0.9.0, unless marked `v0.9.3:`. Reeve at `b4b6cbe`. Read only. No herdr server or session was started, stopped or called. Only `herdr --version` and `herdr server --help` were run.

## 1. Hidden start

**Yes. `herdr server` is the headless server. It runs in the foreground and does not daemonize itself. The `herdr` client detaches it, and hall can do the same thing directly.**

- Entry point: `main.rs:545-547`: `args[1] == "server"` leads to `server::headless::run_server()`. Before that, `cli::maybe_run` returns NotCli for a bare `server` (`cli.rs:105-110`, `cli/server.rs:3-6`, args empty gives `None`). `stop`, `reload-config`, `live-handoff`, `agent-manifests` and the other subcommands are handled there instead (`cli/server.rs:8-24`).
- `--session` works with `server`. `session::configure_from_args` takes `--session <n>` or `--session=<n>` from anywhere in argv (`session.rs:54-78`). It then sets `HERDR_SESSION=<n>` in the process env and sets the explicit flag (`session.rs:448-457`). The remaining args are `[herdr, server]`. So `herdr server --session hall` and `herdr --session hall server` are the same.
  - This settles the open point in `hall:docs/test-setup.md:65` ("if it ignored --session it would aim at the live server"). It does not ignore it: with the explicit flag, the API socket is `api_socket_path_for(name)` (`session.rs:173-176`) and the client socket is `client_socket_path_for(name)` (`server/socket_paths.rs:24-26`).
  - Read from source only, not run.
- How herdr itself detaches the server (`server/autodetect.rs:194-241`, called from `auto_detect_launch` `:295-320`):
  - It spawns `current_exe() server`, with stdin, stdout and stderr set to `/dev/null` (`:216-222`).
  - It calls `setsid()` in `pre_exec` (`platform/mod.rs:142-154`).
  - It passes the cwd as `HERDR_STARTUP_CWD`, which seeds the first workspace when the restored session is empty (`:225-232`, `server/headless/bootstrap.rs:89-117`).
  - When `--session` was explicit, it removes `HERDR_SOCKET_PATH` and `HERDR_CLIENT_SOCKET_PATH` (`:234-238`).
  - It then polls the client socket every 50 ms, for up to 15 s (`:18-23`, `:252-279`).
- **Command hall would run** (the same as herdr's own spawn):
  `env -u HERDR_SOCKET_PATH -u HERDR_CLIENT_SOCKET_PATH -u HERDR_SESSION <pinned>/herdr server --session <name>`, with `setsid`, stdio to `/dev/null`, and the cwd or `HERDR_STARTUP_CWD` set to wherever hall wants the first workspace. Then wait until both sockets accept a connection.
- Bind order: the API socket binds first (`bootstrap.rs:26`), then the client socket (`server/headless.rs:318-322`). `observe` and `control` need the client socket, so hall waits for that one, as herdr does (`autodetect.rs:309`).
- Two `hall` launched at once: the second server finds the socket live and exits 1 with "herdr server is already running" (`bootstrap.rs:32-36`, `ipc.rs:93-96`). Harmless.
- Pitfall (trigger): without the explicit `--session`, an inherited `HERDR_SOCKET_PATH` wins (`session.rs:82`, `:177-179`). `hall` typed inside a live herdr pane would then try to bind default's socket.
  - Masking condition: if default is running, the bind fails as AddrInUse and the mistake looks like "already running". If default is not running, the hall server binds default's socket path.
  - A socket override with no session name also leaves `data_dir` at the root (`session.rs:157-167`), which is default's `session.json` (`persist/io.rs:10-15`).
  - So the name must always go on argv. A socket env alone is never enough.
- Where things go, for session `<n>`, under the config root `R` (section 3):
  - `R/sessions/<n>/herdr.sock` (JSON API) and `R/sessions/<n>/herdr-client.sock` (binary client) (`session.rs:161-185`). Both are chmod 0600 (`socket_paths.rs:12`, `:73-75`).
  - `R/sessions/<n>/session.json` and `session-history.json` (`persist/io.rs:10-15`).
  - Logs: `R/sessions/<n>/herdr-server.log` (`bootstrap.rs:216-218`, `logging.rs:12-31`) and `herdr-client.log` for observe/control (`client/config_reload.rs:3-5`). 5 MB rotation (`logging.rs:9`). Filter set by `HERDR_LOG`.
  - **No pid file** (grep for pid files: no hit). The pid comes from the spawn. The ping capability `detached_server_daemon` (true when `getsid==getpid`, `platform/mod.rs:157-159`, `api/server.rs:70`) confirms the server is detached.
  - Stdout and stderr go to `/dev/null`, so the "ready" lines (`bootstrap.rs:202-213`) are lost. Use the log.
- Headless size:
  - 120x40 by default (`config.rs:79-80`), set by `[server] headless_cols/headless_rows` (`main.rs:221-225`, `config.rs:129-135`).
  - The server uses it whenever no herdr TUI client is in the foreground (`server/headless.rs:818-830`). Pane sizes estimate from it before the first view (`app/state.rs:928`).
  - `observe` renders at the observer's size without resizing the pane (research section 2).
  - Terminal attach/control resizes that pane's terminal to the controller's size and locks it (`server/headless.rs:1866-1877`, `direct_attach_resize_locks`).
- Other routes (not needed):
  - Homebrew ships a launchd agent running `herdr server` with KeepAlive (`/opt/homebrew/Cellar/herdr/0.9.0/sh.brew.herdr.plist`). It is not loaded (no `~/Library/LaunchAgents` entry). This is the pattern for a supervised hall server (a hall-owned plist with `--session`), if KeepAlive is wanted.
  - Spawning a client under a pty and detaching it is not needed.
- `herdr session` has no `start` verb, only `list|attach|stop|delete` (`cli.rs:420-438`). `attach` is just `--session` plus a client (`session.rs:35-52`).
- Nested guard: `herdr server` does not check `HERDR_ENV`. Only the TUI and client paths do (`main.rs:550-553`, `:770-771`).

## 2. Reattach / show on second run

- `hall` is not a herdr client, so it has nothing to "reattach" in herdr's sense. A second run reconnects to sockets that already exist and draws the reeve screen.
- Detecting a running server:
  1. Is the socket path there? `connect()` ok means live. ConnectionRefused, NotFound or TimedOut means stale (`autodetect.rs:49-89`, `session.rs:388-390`).
  2. A stale socket needs no cleanup: the new server removes it before binding (`ipc.rs:81-108`).
- Version check:
  - Send one JSON line `{"id":"hall:ping","method":"ping","params":{}}` to `herdr.sock`.
  - The reply is `{"id":..,"result":{"type":"pong","version":"0.9.0","protocol":22,"capabilities":{live_handoff,detached_server_daemon,endpoint_protocol_generation:1,surface_interest,health_check}}}`. The server builds it at `api/server.rs:342-350`; the type is tagged snake_case at `api/schema/response.rs:43-50`, the capabilities at `api/server.rs:67-75`.
  - The CLI equivalent is `herdr status server --json` (`cli/status.rs:320-330`, which reports `compatible`).
- Where the protocol number lives:
  - `src/protocol/wire.rs:20` `PROTOCOL_VERSION: u32 = 22`. This guards the CLI and the direct-terminal paths.
  - `src/protocol/endpoint.rs:15` `ENDPOINT_PROTOCOL_GENERATION: u32 = 1`. Only the TUI client uses it (`autodetect.rs:145-178`).
  - Hall cannot read either number from a binary statically. It learns the server's from `ping`, and its own pinned value from `<pinned> status --json` with no server running (client side, `cli/status.rs:100,140`) or by hardcoding it.
- What a mismatch does:
  - JSON CLI: every `send_request` pings first and refuses unless the protocols are equal. It prints a `protocol_mismatch` envelope on stderr and exits 1 (`cli.rs:764-799`, `cli/protocol_guard.rs:16-43`, `main.rs:528`).
  - Binary client socket (`terminal session observe|control`, `terminal attach`): `TerminalHello.version` must equal the server's, otherwise the Welcome carries an error (`protocol/wire.rs:1694-1710`, `server/client_transport.rs:700-711`). `observe` and `control` use this socket (`client/terminal_sessions.rs:72-116`).
  - So hall must call observe and control through a herdr binary of the **same protocol** as the server: the pinned one.
- Second-run flow for hall:
  1. Live on `herdr.sock` and `herdr-client.sock`?
  2. If not live: spawn as in section 1.
  3. Ping. If `protocol != 22` (or version != pinned), refuse with a message (decision 30).
  4. `session.snapshot`, `events.subscribe`, draw.
- Stopping hall's server: `herdr session stop <name>` connects to `api_socket_path_for(name)` directly and sends `server.stop` (`session.rs:247-258`, `:260-297`). It never goes through the default server. This settles `hall:docs/test-setup.md:127` too. Never use `herdr server stop`, which stops whichever server it resolves (`session.rs:236-245`).

## 3. Pinned copy: paths, overrides, sharing

Getting a binary:
- Copy `/opt/homebrew/Cellar/herdr/0.9.0/bin/herdr` (22 MB), or `cargo build --release` at tag `v0.9.0`.
- A debug build changes the app dir name to `herdr-dev` (`config/io.rs:22-28`), so `~/.config/herdr-dev` is separate from the live one with no env at all. Debug builds also disable background update checks (`app/mod.rs:166-168`). Speed was not measured.

Roots:
- `CFG` = `$XDG_CONFIG_HOME/herdr`, else `$HOME/.config/herdr` (`config/io.rs:30-35`, `:61-68`).
- `STATE` = `$XDG_STATE_HOME/herdr`, else `$HOME/.local/state/herdr` (`:37-42`, `:87-94`).
- There is **no other herdr root override**: no `HERDR_HOME`, no data-dir flag.

| What | Path | Override |
|---|---|---|
| config file | `CFG/config.toml` | `HERDR_CONFIG_PATH` (`config/io.rs:169-174`) |
| pending release notes | next to the config file, `release-notes.json` | moves with `HERDR_CONFIG_PATH` (`release_notes.rs:27-31`) |
| session dir | `CFG/sessions/<n>/` (default: `CFG/`) | `XDG_CONFIG_HOME`, session name (`session.rs:161-167`) |
| API socket | `<session dir>/herdr.sock` | `HERDR_SOCKET_PATH` unless `--session` (`session.rs:169-181`) |
| client socket | `<session dir>/herdr-client.sock` | derived from `HERDR_SOCKET_PATH`; `HERDR_CLIENT_SOCKET_PATH` (`socket_paths.rs:23-58`) |
| session.json, session-history.json | session dir | as session dir (`persist/io.rs:10-15`) |
| logs herdr-server/-client/.log | session dir | as session dir (`logging.rs:12-38`) |
| plugin registry | `CFG/plugins.json` | `XDG_CONFIG_HOME` only (`persist/plugin_registry.rs:11-12`) |
| plugin checkouts and config | `CFG/plugins/{github,config}/` | `XDG_CONFIG_HOME` only (`plugin_paths.rs:5-19`) |
| plugin state | `STATE/plugins/<id>` | `XDG_STATE_HOME` (`plugin_paths.rs:21-25`) |
| agent detection overrides | `CFG/agent-detection/<agent>.toml` | `XDG_CONFIG_HOME` (`detect/manifest.rs:1129-1135`) |
| agent manifest cache (fetched from net) | `STATE/agent-detection/` | `XDG_STATE_HOME`; fetch off with `[update] manifest_check=false` (`detect/manifest_update.rs:497-499`, `app/mod.rs:540-556`) |
| product announcements | `STATE/product-announcements.json` | `XDG_STATE_HOME` (`product_announcements.rs:8,77-79`) |
| machines / endpoint catalog | `STATE/client/endpoints.json`, `endpoint-selection.json` | `XDG_STATE_HOME` (`client/endpoint/catalog.rs:369-379`) |
| pane graphics | `$XDG_RUNTIME_DIR` or `/var/tmp` `/herdr-pane-graphics-<uid>` | `XDG_RUNTIME_DIR` (`pane_graphics_files.rs:200-208`); per uid, so shared |
| sound temp | `$TMPDIR/herdr-sound-<pid>-<id>.mp3` | per pid, so no clash (`sound.rs:110`) |
| worktrees | `~/.herdr/worktrees` | `[worktrees] directory` in config (`worktree.rs:722`, `main.rs` default config) |
| agent integrations | `~/.claude`, `~/.codex`, ... | `CLAUDE_CONFIG_DIR`, `CODEX_HOME`, ... (`integration/env.rs:58-63`, `integration/targets.rs`) |
| update check | network, herdr.dev / brew | `[update] version_check=false` (`app/mod.rs:540-549`) |

- The update check never installs: a non-Homebrew copy only saves notes ("waiting for explicit install", `update.rs:2297-2320`).
- Integrations are written only on an explicit `herdr integration install` (`app/api/integrations.rs:38`). Their hooks route by the pane's `HERDR_SOCKET_PATH` (`integration/assets/claude/herdr-agent-state.sh:20-35`), so one installed hook serves either server. They are shared regardless. Integration asset version: 9 at v0.9.0, 10 at v0.9.3 (line 6 of that file).

**Pane env inheritance (trigger for every "separate root" option):**
- Each pane gets `HERDR_ENV=1`, `HERDR_SOCKET_PATH=<this server's API socket>`, `HERDR_BIN_PATH=<this server's exe>` and the pane, tab and workspace ids (`pane.rs:137-160`, `integration/env.rs:28-33`).
- **Everything else is inherited from the server process.** Only `WT_SESSION` and `CODEX_THREAD_ID` are removed (`pane.rs:89`, `:139`).
- So any `XDG_CONFIG_HOME`, `XDG_STATE_HOME`, `HERDR_CONFIG_PATH`, `PATH` or `REEVE_*` that hall sets on its server reaches every shell, claude, git and so on in it.

Can the pinned server share nothing?
- **Route S (same root, own session name, `HERDR_CONFIG_PATH`)**:
  - Separate: sockets, session.json, logs, config.
  - Shared with live:
    - `plugins.json` and checkouts: the liege's `layouts` plugin is loaded, and plugin `[[startup]]` hooks run in the hall server (`bootstrap.rs:79`).
    - `agent-detection/` overrides.
    - All of `STATE`: the manifest cache is written by both versions; also endpoints and plugin state.
    - `~/.herdr/worktrees`, integrations.
  - The session appears in the live `herdr session list` (`session.rs:187-212`).
  - The env leak is limited to `HERDR_CONFIG_PATH`.
- **Route X (`XDG_CONFIG_HOME` + `XDG_STATE_HOME` set to hall dirs on the server)**: shares nothing herdr-side except integrations (in `~/.claude` etc., untouched unless installed), the pane-graphics dir, and worktrees (set its dir in hall's config).
  - Cost: both XDG vars leak into every pane.
  - Guess, not checked: XDG-aware tools in those panes (for example `gh`, which reads `$XDG_CONFIG_HOME/gh`) would lose their config.
  - Possible mitigation: `[terminal] default_shell` (`main.rs:103-106`) set to a wrapper that unsets them. Not tried. It only covers panes started through the shell.
- **Debug-build route**: `herdr-dev` dirs with no env leak. Same sharing as X, minus the leak. Speed unknown.

What breaks when reeve's dispatch (Homebrew `herdr` on PATH, `reeve:backends/herdr.sh:19-28`) talks to a pinned server:
- Different protocol: every JSON call fails with `protocol_mismatch`, exit 1 (`cli.rs:779-799`).
  - Masking condition: `_h` sends stderr to `/dev/null` (`reeve:backends/herdr.sh:27`), so the reason never shows. Calls just come back empty or failed. `reeve_backend_herdr_available` would report only "herdr server not running" (`:116`).
- Same protocol (22) but different version: calls pass the guard. Per-method drift is unchecked: new or removed params or fields (e.g. `pane.graphics.*` removed in the 0.9.3 docs, research section 1). Guess: low risk at 0.9.0 to 0.9.3.
- `herdr session list` is filesystem-only, with no protocol check (`session.rs:187-226`). Under route X, Homebrew herdr only sees hall's root if `XDG_CONFIG_HOME` is in its env. Inside hall panes it is, by inheritance.
- `observe`/`control`/`attach` from a mismatched binary are refused at the handshake (`protocol/wire.rs:1694-1710`).
- Lever: hall can prepend the pinned bin dir to `PATH` in the server env. Then every pane's `herdr` is the pinned one and matches by construction. `HERDR_BIN_PATH` already points there (`integration/env.rs:30-32`), but reeve does not read it (grep: no hit in reeve).

## 4. Session placement in real use

Facts:
- Reeve chooses the herdr session from `REEVE_HERDR_SESSION`, else the `default:true` row of `herdr session list --json` (`reeve:backends/herdr.sh:10-17`). That row is always the root's `default` (`session.rs:214-226`).
- Every call adds `--session` (`:19-28`), which beats the pane's `HERDR_SOCKET_PATH` (`session.rs:80-83`).
- Socket-identity checks compare `_h_sock`, the socket of the named session as `herdr session list` reports it, with `HERDR_SOCKET_PATH` (`reeve:backends/herdr.sh:127-133`, `:187`, `:601`).
- So a reeve inside a hall server, with no `REEVE_HERDR_SESSION`, dispatches its hands into `default`. Its nesting and tab-label checks then fail, because the sockets differ.

| Option | What it is | Needs from reeve | Needs from hall / herdr | Facts against |
|---|---|---|---|---|
| A. Hall attaches to live `default` | Hall's own server is unused or empty; hall reads the Homebrew server | Nothing for placement (the listing, ADR 0002, still pending) | Lifting decisions 9/28 at go-live; hall's observe/control must match the protocol of the **live** server | No pinned copy in effect: a brew upgrade plus a restart of default can change the protocol under hall (research section 6) |
| B. Reeves started inside hall's server | New reeves are launched in hall's pinned server | `REEVE_HERDR_SESSION=<hall session>` in each reeve's env (it exists, `:14`); `herdr` on PATH matching hall's protocol; reeve start/launch aimed at hall's server; under route X, Homebrew herdr resolving hall's root (inherited `XDG_CONFIG_HOME`) | Hall's server env can carry `REEVE_HERDR_SESSION`, `PATH` and `REEVE_HOME` to every pane by inheritance (`pane.rs:137-160`) | Reeves already in `default` do not move: no cross-server pane move exists (live handoff only replaces the same server, `server/handoff.rs:249-254`); a cut-over is needed |
| C. Both, during transition | Hall watches `default` and its own server (decision 6 already covers several herdr sessions) | B for new reeves | Per-server protocol check; two herdr binaries, or one protocol for both | Holds A's coupling for as long as default is used |

- Route X plus option B: making hall's server the **default** session of hall's own root would let reeve's unchanged `_h_session` resolve it inside hall panes. But decision 28's wording ("no herdr call ever names the session default") would then need reading as "the live default". That is a wording question, not a mechanism.
- Judgement (one line): B is the only option where "updating Homebrew never breaks hall" holds, and it costs a reeve launch change plus a one-time cut-over. A is today's reality and keeps hall coupled to the live install.

## 5. Licence

- Herdr: Apache-2.0 (`LICENSE` at root; `Cargo.toml:7`; `brew info herdr`: "License: Apache-2.0").
- **No NOTICE file** at v0.9.0 or v0.9.3 (`git ls-tree -r` matching notice, licen or copying: only `LICENSE`, `vendor/*/LICENSE*`, the Windows ConPTY files, and `src/client/shell/endpoint_notices.rs`, which is code). The `LICENSE` appendix is the unfilled template ("Copyright [yyyy] [name of copyright owner]").
- The binary also includes:
  - `vendor/libghostty-vt`, MIT, "(c) 2024 Mitchell Hashimoto, Ghostty contributors".
  - `vendor/portable-pty`, MIT, "(c) 2018 Wez Furlong", patched (`vendor/portable-pty.patches.md`).
  - Crate dependencies, whose licences were not enumerated. Homebrew ships `sbom.spdx.json` with only 4 `name` entries, so it is not a full list.
- Apache-2.0 section 4 applies **on redistribution**: give a copy of the licence (4a), mark modified files (4b), keep the source's attribution notices (4c), pass on NOTICE if present (4d: none). Section 6: no trademark rights, so hall must not be branded "herdr". MIT: keep the copyright and permission notice with copies.
- While hall stays private and local (decision 18), running a pinned copy is use, not redistribution, so section 4 is not triggered. Keeping `LICENSE` beside the pinned binary costs nothing. Shipping hall later with herdr bundled means shipping the herdr `LICENSE`, the two vendor MIT licences, and the Cargo dependency licences (e.g. `cargo about`; not run).
- Not legal advice. This reading is from the licence text and repo files only.

## 6. v0.9.3 against 1 to 3

- Protocol unchanged: `v0.9.3:src/protocol/wire.rs:20` is still 22, endpoint generation still 1 (`v0.9.3:src/protocol/endpoint.rs:15`). Snapshot format still 3 (`persist/snapshot.rs:12` at both tags). So 0.9.0 and 0.9.3 interoperate on the CLI and binary paths. Unchanged as well: the session, socket, config-dir and protocol-guard code (`git diff v0.9.0 v0.9.3` of `session.rs` changes only `delete_session`; `config/io.rs` adds only BOM handling; `socket_paths.rs`, `cli/protocol_guard.rs`, `api/status.rs` and `plugin_paths.rs` have no diff).
- **macOS server context (affects section 1)**:
  - When herdr spawns the daemon it now sets the private env `HERDR_MACOS_SERVER_CONTEXT=user` (`v0.9.3:src/platform/mod.rs:197-201`, `v0.9.3:src/platform/macos/bootstrap.rs:23-32`).
  - At start, the server moves itself to the per-user Mach bootstrap namespace (`bootstrap.rs:35-45`, called from `v0.9.3:src/server/headless/bootstrap.rs:7`). The source comment says "setsid() does not detach from Aqua's Mach bootstrap namespace".
  - A server hall spawns directly on 0.9.3 does not get this unless hall sets that env too. It is private, so it can change without notice. At 0.9.0 the code does not exist.
  - Guess: this is about servers outliving a GUI logout, and is irrelevant while the liege stays logged in.
- **Client needs a terminal before it spawns** (`v0.9.3:src/server/autodetect.rs:296-303`): `herdr --session X` with no tty now fails before starting a daemon. Hall does not rely on the client route, so no effect. It does rule out "run the client without a tty to spawn the server" as a trick.
- Unchanged for section 3: no new root overrides; the plugin registry is still at `CFG/plugins.json`. The changes in `persist/plugin_registry.rs` and `persist/writer.rs` add code, not paths (diffstat only; `registry_path` was not re-read at v0.9.3).
- Integration hooks: v9 to v10. Not version-locked to the server: they speak JSON to whatever `HERDR_SOCKET_PATH` names.

## Not checked

- Nothing was run against a server. `herdr server --session <n>` honouring the name is read from source (`session.rs:54-81`), not observed.
- Payload of `ping` read from source, not captured.
- No per-method JSON drift between 0.9.0 and 0.9.3.
- Speed of a debug build.
- The `default_shell` wrapper as an XDG-leak fix.
- Behaviour of other tools under an inherited `XDG_CONFIG_HOME`.
- The full dependency licence list.
- Whether `run_plugin_startup_hooks` of the liege's `layouts` plugin has side effects in a second server.

## Open for the liege

1. **Where reeves live (section 4)**: A (hall reads the live `default`; no real pinning), B (new reeves start inside hall's own server; needs a reeve launch change and a cut-over), or C (both, during a transition).
2. **How separate the pinned server is (section 3)**:
   - S: own session, own config file, shares plugins, detection overrides and state with live.
   - X: own XDG roots, shares nothing herdr-side, but every pane inherits the XDG vars.
   - Debug build: own `herdr-dev` dirs, no env leak, unmeasured speed.
3. **Pinned binary source**: copy the Homebrew 0.9.0 binary, or build `v0.9.0` from source (needed for the debug-build route).
