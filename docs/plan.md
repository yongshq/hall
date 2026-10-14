# Hall: version one plan

Terms are the ones in [GLOSSARY.md](../GLOSSARY.md). The decisions behind this plan are in [decisions.md](decisions.md) and the [decision records](adr/); the one that shapes it is decision 31, ADR [0004](adr/0004-fork-of-herdr.md): hall is a fork of herdr. Facts come from three reports kept in this repository: [research/herdr-launch.md](research/herdr-launch.md) (cited as "launch section N"), [research/fork-map.md](research/fork-map.md) ("fork-map section N") and [research/herdr-surface.md](research/herdr-surface.md) ("research section N"). Paths into herdr's code are at v0.9.0.

## Goal

Hall is the liege's audience hall: one screen over every live reeve, each reeve at the top level of the sidebar with its hands in flight beneath it, a hand state on every hand and a waiting-on-you badge on every reeve. The liege types `hall` and nothing else: it starts and attaches like herdr, detaches, and a second `hall` reattaches (decision 30). Picking a reeve or a hand shows its session, and the liege types into it as in herdr. When a hand becomes stuck or a reeve needs the liege, hall raises an alert. Hall is herdr, forked from v0.9.0 and renamed, sharing nothing with the liege's installed herdr (decision 31); reeve stays the only source of errand state (ADR [0002](adr/0002-state-through-reeve.md)).

## Version one scope

In:

- Hall starts like herdr: `hall` starts its server out of sight if none is running, attaches, detaches and reattaches (decision 30). Sessions, panes, input, the prefix key ctrl+b (decision 14) and scrollback come with herdr.
- Separate from the live herdr: own config and state folders, binary, inbound environment names, protocol number and client endpoint generation (decisions 33 and 44).
- Reeve tree in the sidebar: every reeve in hall's one session (decisions 6 and 38), reeve = workspace and hand = tab (decision 34), errands in flight only (decision 16), a hand state mark per hand (decision 12). Workspaces that are not reeves are kept apart from the reeve list.
- Badges: a count per reeve of what is on disk: a hand's unanswered question, a hand waiting, blocked, idle or wedged (decision 17).
- Alerts with sound: herdr's own request and done sounds for a session at a dialog or ending its turn, every reeve reply included (decisions 19, 22, 35); hall adds a sound and a highlight on the reeve when a hand becomes blocked, idle, wedged or asks a question, which herdr cannot see. The sound can be changed or muted.

Out:

- Household actions (dispatching, answering, cleaning up errands): these stay with the reeve (decision 8).
- The reeve's full waiting list: the badge counts only what is on disk until a reeve change the liege approves (decision 17).
- The reeve side of the bridge (a hall backend in reeve, and whether reeve pushes hand state to hall or hall reads the listing): decided with the liege at go-live, step 5 (decision 36). Decided by decisions 67 and 68: a `hall` backend in reeve, and hall reads reeve's listing; reeve's side is built in reeve's repository.
- Narrowing the turn-end alert to replies that ask the liege something: later, if the alert proves noisy, and it needs a signal reeve does not record today (decision 22).
- Herdr upstream changes past v0.9.0: ported only when chosen, by hand (decision 31).
- Publishing: the repository stays private and local, with no remote to push to, until version one works (decisions 18, 32).

## Architecture sketch

- **One program, `hall`.** Herdr's server and client, renamed. `hall` with no server running spawns `hall server` detached (setsid, stdio to `/dev/null`) and waits for its sockets, as herdr does (launch section 1). Herdr's client draws the chrome from the server's snapshot, which already carries every tab with its own agent status (fork-map section 2).
- **Repository.** Herdr's history merged at hall's root (`git merge --allow-unrelated-histories v0.9.0`); herdr's crate is the root package and `crates/hall-standin` sits beside it in one cargo workspace. Remote `upstream` is github.com/herdrdev/herdr, fetch only, push URL disabled (decision 32; fork-map section 5).
- **Separation patches** (decision 33; fork-map section 1, P1 to P8):
  - config and state folders named `hall` (`app_dir_name`, `src/config/io.rs:22-28`; a release build uses `~/.config/hall`, a debug build `~/.config/hall-dev`), which moves sessions, sockets, logs, plugins and detection overrides in one change;
  - binary `hall`;
  - inbound environment renamed `HALL_*`: socket path, client socket path, session, config path, nested-guard marker;
  - protocol number bumped off 22 (to 1022), so a herdr CLI or API call that reaches a hall socket, or the reverse, is refused at the protocol guard; and the client endpoint generation bumped off herdr's 1 (to 1001, `ENDPOINT_PROTOCOL_GENERATION`, `src/protocol/endpoint.rs:18`), so a herdr client on a hall client socket, or the reverse, is refused at the endpoint handshake. The client path rests on the endpoint generation alone: a port that resets `endpoint.rs` reopens it, and a frozen hall welcome at 1001 (`tests/fixtures/hall-endpoint-welcome-1001.json`) is there to fail first. On macOS the herdr client's refusal does not read as a protocol mismatch: it fails with "Invalid argument (os error 22)" (decision 44d);
  - `update` and self-replace, the agent-manifest fetch, `--remote` install and `integration install` refused (`hall integration install` would overwrite the liege's `~/.claude/hooks/herdr-agent-state.sh`);
  - worktree folder and pane-graphics paths renamed.
- **Reeve tree.** A client-side sidebar change: tab rows under each workspace row, plus selection and hit testing (fork-map section 2, option A). Kept in few, new functions, because the client shell is herdr's busiest `src/` area (fork-map section 7).
- **Reeves and errands from reeve's listing** (ADR 0002). Hall never reads status files. During development the stand-in serves the listing from the test setup (decision 11).
- **Sound.** Herdr's own sounds, played by the client driving the session, which is the hall window (decision 35; fork-map section 4). Hall adds the triggers that come from reeve's state.

## Listing format hall needs from reeve

**Settled** in step 5 as version 1 ([listing.md](listing.md), decision 68), from the draft the stand-in served (decisions 11 and 36, ADR 0002). Reeve prints it (`bin/reeve-listing`); the stand-in serves the same format for hall's tests. These are the fields hall reads.

Per reeve:

| Field | Meaning |
|---|---|
| name | The reeve's name, as shown in the reeve tree. |
| session location | Which hall session and which pane inside it hold the reeve. |
| live | Whether the reeve is alive now. |

Per errand in flight:

| Field | Meaning |
|---|---|
| id | The errand id. |
| office | The office the hand holds. |
| owner reeve | The reeve that dispatched it, so hall nests the hand beneath it. |
| hand state | Working, needs a decision, blocked, done or failed, as the reeve reconciles it. |
| question | A question the hand has asked that is still unanswered, if any. |
| waiting, idle, wedged | One flag each, as the reeve judges them. |
| session location | Which hall session and which pane inside it hold the hand. |

A session location is `{ backend, name, pane_id }` since version 1: hall shows only `backend` `hall`, so herdr's reeves and hands, in herdr's own `default`, never land on hall's panes. The stand-in's world file still names the session `herdr_session`; from step 2 it holds a hall session name (fork-map section 8).

## Test setup

Hall is built and tested only against the test setup (decisions 10 and 38, ADR [0003](adr/0003-separate-from-live-setup.md)):

- One hall session, `hall-dev`: a hall server with its own socket, run by the debug build (`target/debug/hall`). The debug build's folder is `~/.config/hall-dev`, so the session sits at `~/.config/hall-dev/sessions/hall-dev/`. Live use is the release build, whose folder is `~/.config/hall`; a release `hall` does not list the test session. One session is enough, since once hall is in use every reeve lives in hall's own session (decision 38).
- A reeve home at `~/.reeve-hall-dev`. Every test reeve and test hand starts with `REEVE_HOME=~/.reeve-hall-dev` on its own command line (decision 26): the liege's reeve plugin hooks run in every claude session, in hall panes too.
- Test reeves and test hands: claude sessions, each test reeve in its own workspace and its test hands as tabs in that workspace, placed by the scripts, since test reeves do not dispatch through hall before go-live (decision 36).
- The stand-in: serves the listing (version 1) for them from `~/.reeve-hall-dev`. Since step 5 the test setup can instead read reeve's own listing of that home and start one test reeve that dispatches its own hands through reeve's hall backend (`--listing reeve`, [test-setup.md](test-setup.md)).

Step 2 moved the test setup built for herdr (decision 24) to hall ([test-setup.md](test-setup.md)): the one session `hall-dev`, `hall-dev-2` dropped (decision 38). It makes no herdr call at all and creates no herdr session (decision 33); its own check of the live herdr's session list is gone, and `status` runs `scripts/live-check` (decision 40). Every test reeve and test hand starts with `XDG_CONFIG_HOME=~/.reeve-hall-dev/xdg-config` and `XDG_STATE_HOME=~/.reeve-hall-dev/xdg-state` beside `REEVE_HOME` (decisions 47 and 48), so no herdr call from a test pane can reach the live `default` or write the live herdr's state, and with nothing of who ran `up` (decision 48). The herdr-built version was never brought up live, so no herdr session of its exists and none needs deleting.

**Hard rule: never the live herdr, never `~/.reeve`.** The liege's herdr (its session `default`, its folders and sockets) and `~/.reeve` belong to the live setup, which stays untouched while hall is developed (decision 9).

**The "live setup unchanged" check** (decisions 28, 40 and 41, reworded for the fork). Once hall has its own folders, hall never sees herdr's sessions and hall's own `default` is not the live one, so decision 28's guards are partly moot (fork-map section 8). The live herdr rewrites files in its own folders whenever the liege uses it (its `default` session keeps its data in the config folder itself, `src/session.rs:162-166`), so the check never compares those folders as a whole. The check makes no herdr call at all (decision 40): it reads the live herdr's folders from the filesystem and the process table, nothing else. What is checked (decision 41):

- **No hall-made entry in the live herdr's four places**, judged against a baseline listing of each that the check records on its first run, each listed to its own depth (an absent folder is an empty listing, recorded as such):
  - `~/.config/herdr`, the config folder: entry names at every level (on this Mac the folder is a link into the liege's dotfiles; the check follows that one link and no link below it);
  - `~/.local/state/herdr`, the state folder: entry names at every level;
  - `~/.herdr/worktrees`, the worktree folder (`WorktreesConfig`'s default, `src/config/model.rs:1153`), where a worktree lands at `<repo>/<branch>`: two levels, `<repo>` and `<repo>/<branch>`;
  - `/var/tmp/herdr-pane-graphics-<uid>`, the pane-graphics folder (`runtime_base`, `src/pane_graphics_files.rs:200-208`; `XDG_RUNTIME_DIR` is unset on macOS, so `/var/tmp` it is): one level. Each server makes a `server-<pid>-<nonce>` folder there (`:188`), so a new entry names the process that made it.

  Hall's names here come from the repo, the branch or the pid, not from `hall*`, so the check lists every entry that is not in the baseline. The liege's own herdr use adds entries too, so a new entry fails the check until it is accepted (decision 42): `scripts/live-check accept <place> <entry> --by <who> --reason <why>` adds it to the baseline with who accepted it, why and when. Each new entry is printed with what the filesystem and the process table tell about it: its birth time; for a worktree, the repo its `.git` file points to; for a pane-graphics folder, the command of its pid if that process is alive. An entry gone from the baseline is printed and does not fail: the liege's herdr deletes its own.
- **No hall process connected to a live herdr socket**, found by following each of hall's unix socket peers. A path-only `lsof` (`lsof ~/.config/herdr/herdr.sock`) cannot see a connecting client on macOS: the connecting side of a unix socket carries no path, only its peer's address, so the path lists the live server's own fds and nothing else. The mend-fork-plan-2 review found this with a throwaway server and client on a socket of its own, no herdr involved. Rerun as one python process holding the listener, a client and the accepted connection, `lsof -nP -U -a -p <pid>` prints:

  ```
  COMMAND   PID USER   FD   TYPE             DEVICE SIZE/OFF NODE NAME
  Python  69948 yong    3u  unix 0x47fc7ca2015fb503      0t0      s.sock
  Python  69948 yong    4u  unix 0x6144d70e1e13a59a      0t0      ->0xf712f070ab9808fc
  Python  69948 yong    5u  unix 0xf712f070ab9808fc      0t0      s.sock
  ```

  Fd 4 is the client: no name, only `->0xf712...`, the address of the server's accepted fd 5, which carries the socket's name. So the check lists hall's unix fds (`lsof -nP -a -U -c hall`), takes each `->0x<addr>` peer, and fails when that address is one of the live herdr's socket fds whose name is under `~/.config/herdr` (from `lsof -nP -a -U -c herdr`, every herdr process, not one server), naming the hall pid and the herdr socket. Both sides need `-a`: without it lsof ORs `-U` with `-c` and prints every process's unix fds and every fd of the named command; the check also keeps only `unix` lines of that command, so lines a missing `-a` would add cannot trip it. If lsof is missing or errors, this part fails; it never passes on no output. lsof exits 1 with no output at all when nothing matches, which the check reads as an empty list.

  This sees only the connections open at the instant lsof runs: a hall CLI call that opens and closes a connection between two runs is not seen. That case is covered by a test instead (`tests/hall_separation.rs`): hall's CLI and client run with `HERDR_SOCKET_PATH` and `HERDR_SESSION` pointing at throwaway listener sockets, under a temporary HOME, `XDG_CONFIG_HOME` and `XDG_STATE_HOME`, and no listener may see a connection; as a control, `HALL_SOCKET_PATH` and `HALL_SESSION` at the same kind of listener do connect.
- **No file under `~/.reeve/state` names the test home's path or a test reeve or errand name**, read only, in a file's contents or its own path under `~/.reeve/state`. The home matches as a path component (`/.reeve-hall-dev` followed by a character that cannot continue the name), the names from `scripts/test-setup.conf`'s roster as whole words (not inside a longer run of letters, digits, `_` or `-`), so `ansel` matches neither `anselm` nor `ansel-fix`. Only regular files are read: no link is followed and no fifo or socket opened.

Nothing about default's socket file or folder is compared (decision 41): a clean herdr stop deletes the socket file (`ServerHandle`'s `Drop`, `src/api/server.rs:41-50`, and `cleanup_sockets`, `src/server/headless/lifecycle.rs:359-367`), so its presence changes whenever the liege stops or starts herdr and says nothing about hall.

The check does not use `herdr session list`, which earlier drafts and fork-map section 3 took for a folder read: it fills each entry's `running` by connecting to that session's socket (`src/session.rs:214-222`, `is_running_at` at `:388-390`), so it would reach the live `default` server.

The check is its own tool, `scripts/live-check`, separate from the test setup scripts; step 1 built it. It prints one line per part and exits 0 only when every part passes. It keeps its baseline at `~/.local/state/hall-live-check/baseline.json`, a folder of its own, recorded on its first run, before anything of herdr's was merged, built, tested or run in hall (finding 2 of review-mend-fork-plan-3). No flag rewrites the baseline and `accept` only adds to it; a baseline deleted after it existed cannot be told from a first run, so deleting it is the only reset, and it re-baselines over anything new since. Not in `~/.local/state/hall-dev`: that is the debug hall's own state (agent detection, announcements, plugin and client state, `src/config/io.rs:37-42`), and deleting it is a plausible way to reset hall-dev, which would silently re-baseline the check over any entry hall made meanwhile. It is also outside the test home, so resetting `~/.reeve-hall-dev` cannot re-baseline it; and unlike the repo's gitignored `target/`, it is not wiped by `cargo clean` and is the same file from every worktree.

## Steps

Each step works end to end and can be shown to the liege on its own. Steps 1 to 4 work only in `hall-dev` and `~/.reeve-hall-dev` (or, for step 1, hall sessions of its own and a throwaway herdr server under temporary folders), and each ends with the "live setup unchanged" check. Step 5 is exempt: it is the cut-over, which changes reeve and starts real reeves in hall, with the liege's approval (decision 9).

### 1. Fork and separate

- **Outcome:** herdr v0.9.0 merged at hall's root (decision 32), with the stand-in beside it in one cargo workspace; the separation patches (decision 33); herdr's `LICENSE` and both vendored MIT licences kept, a `NOTICE` line naming hall as a modified herdr, `MODIFICATIONS.md` listing the changed files; herdr's `AGENTS.md` replaced by hall's own (herdr's kept as a reference file) and `build.rs`'s external-contributor warning dropped; the build working on this Mac (decision 37); herdr's own test suite passing under the new names; the "live setup unchanged" check built as `scripts/live-check` (above).
- **Shown working:** `hall` opens, detaches and reattaches exactly like herdr; a second `hall` reattaches to the same server; a herdr CLI pointed at a hall socket is refused with a protocol mismatch, and so is hall pointed at a herdr socket; a herdr client on a hall client socket is refused at the endpoint handshake, which on macOS reads "Invalid argument (os error 22)" (decision 44d). Both legs run the Homebrew `herdr` binary under a throwaway temporary `XDG_CONFIG_HOME` and `XDG_STATE_HOME` (decision 40). For the forward leg only `HERDR_SOCKET_PATH` points at the hall socket: the herdr client opens `herdr-client.log` in its data folder before it connects (`src/client/mod.rs:142`, `src/client/config_reload.rs:3-4`), and without a temporary `XDG_CONFIG_HOME` that is `~/.config/herdr` itself, the live `default` client's log. The herdr CLI writes no log file (only the client and the server, `src/server/headless/bootstrap.rs:217`, call `init_file_logging`). For the reverse leg the herdr socket is a throwaway herdr server, started under those temporary folders with `HERDR_SOCKET_PATH` and `HERDR_SESSION` unset, and stopped after. Never the live `default`'s socket: every CLI request pings the server before the protocol check (`src/cli.rs:764-799`), so hall pointed there would reach the live server (decisions 9 and 28); `hall update` and `hall integration install` are refused; `cargo test --workspace` passes, herdr's tests and the stand-in's together; `scripts/live-check` passes, making no herdr call: no hall-made entry in the live herdr's four places, no hall process connected to a live herdr socket, and nothing in `~/.reeve/state` naming the test setup.
- **Needs:** Rust 1.96.1 (herdr's pin) and zig 0.15.2, both installed (decision 39); the zig build question below.

### 2. Test setup on hall

- **Outcome:** the test setup scripts and the stand-in retargeted from herdr to the debug hall binary and the one hall session `hall-dev` (fork-map section 8, decision 38): the one `HALL_HERDR` array in `scripts/test-setup-lib.sh` and the `command -v herdr` check in `scripts/test-setup`, the fake binary in `tests/scripts_fake_herdr.rs`, and `env -u` of the `HALL_*` names instead of the `HERDR_*` ones. The scripts' own live check is dropped in favour of `scripts/live-check` (decision 40): `up` no longer takes a baseline of the session list's live entry (`scripts/test-setup:127-132`) and `status` no longer compares it (`scripts/test-setup:199-212`); `status` runs `scripts/live-check` instead. The second session is dropped: `HALL_DEV_SESSION_2`, `--session-2` and the session number in the roster. Test hands are placed by the scripts as tabs in their test reeve's workspace.
- **Shown working:** `scripts/test-setup up` brings up test reeves and test hands in `hall-dev`, each test hand a tab beside its test reeve; the stand-in's listing names every test reeve and errand with session locations that match; a test reeve driven to a dialog and to the end of a turn reads so in hall; `--dry-run` and the fake-binary tests show the test setup making no herdr call at all, and `scripts/live-check` makes none either; the "live setup unchanged" check passes.
- **Needs:** step 1; and, settled before this step starts, a guard that keeps a test reeve or test hand in a hall pane from reaching the live `default` through Homebrew `herdr --session` or `herdr session list`, which reeve's herdr backend and plugin hooks call (open question "Environment names set into panes"; decision 44f), with the liege if it touches reeve. Settled by decision 47: `XDG_CONFIG_HOME` pointed at a throwaway folder on every test command line.

### 3. Reeve tree in the sidebar

- **Outcome:** hall's sidebar shows every test reeve in `hall-dev` at the top level with its hands in flight beneath it (reeve = workspace, hand = tab, decision 34), a hand state mark per hand and a waiting-on-you badge per reeve, from the stand-in's listing (decision 17). Workspaces that are not reeves are kept apart from the reeve list. Moving through the tree with keys and picking a hand shows its pane. One line of key help at the bottom (decision 12): herdr v0.9.0 has none, only a help overlay (`src/client/shell/overlays.rs:917`) and, with mouse capture, `new` and `menu` in the sidebar footer (`src/client/shell/sidebar.rs:368-420`).
- **Shown working:** several test reeves appear side by side in one reeve tree, each with its hands beneath; the key help line shows at the bottom; changing a test hand's state in `~/.reeve-hall-dev` changes its mark and its reeve's badge; finishing and cleaning up an errand drops the hand from the tree; a plain workspace stays out of the reeve list; the "live setup unchanged" check passes.
- **Needs:** step 2.

### 4. Alerts

- **Outcome:** herdr's request and done sounds play in the hall window for a session at a dialog or ending its turn (decision 35). Hall adds a sound and a highlight on the reeve when a hand becomes blocked, idle, wedged or asks a question, from the listing (decision 19). Every reeve reply alerts (decision 22); if herdr's suppression for the focused active tab hides it, a setting turns that off. The sound can be changed or muted.
- **Shown working:** each trigger in turn raises the highlight and plays the sound:
  - a test hand at a dialog (waiting);
  - a test hand set to blocked in `~/.reeve-hall-dev`;
  - a test hand set to idle;
  - a test hand set to wedged;
  - a test hand with a new unanswered question;
  - a test reeve driven to a dialog;
  - a test reeve ending a turn, alerting on every reply;
  - a test reeve ending a turn while its tab is active and the hall window has focus, with the setting that turns herdr's suppression off (decisions 22 and 35).

  With the sound muted, a repeat of any trigger shows only the highlight. The "live setup unchanged" check passes.
- **Needs:** step 3 for the highlight on the reeve; step 2 for setting states and driving test reeves on demand. The turn-end question below is settled before this step starts. Settled by decision 62; built in step 4 with decision 63 (the highlight, its clearing, the listing alert's sound), amended by decisions 64 and 65 after its reviews, and `scripts/test-drive` drives a test hand too, for the hand at a dialog.

### 5. Go-live bridge

- **Outcome:** the liege decides the reeve side of the bridge (decision 36): a `hall` backend in reeve or a smaller change to reeve's herdr backend (fork-map section 3), and whether reeve pushes hand state to hall as metadata tokens or hall reads the listing; with it, the listing format itself (decision 11). Then the cut-over: new reeves start inside hall. Reeves already in the live herdr's `default` finish there, since no pane moves between servers (launch section 4); until they do they stay visible in herdr, not in hall (decision 38).
- **Shown working:** a reeve started inside hall dispatches a hand, which opens as a tab in that reeve's workspace in hall and nowhere in the live herdr; its state, badge and alerts show in hall from the real reeve, not the stand-in; reeves still in the live herdr keep running until they finish.
- **Needs:** steps 1 to 4, and the liege's approval of the reeve change, which touches the live setup (decision 9). Approved by decision 67 ("yes to all", 2026-10-14): a `hall` backend in reeve selected by `REEVE_BACKEND=hall`, hall reading a listing from reeve each second, and changing the reeve setup for go-live. Designed in errand step5-bridge-design, adopted as written by decision 68.
- **Hall's side (built):** hall reads the listing at version 1 only, refusing a missing or other version as a failed read, with session locations `{ backend, name, pane_id }`, and drops every entry whose backend is not `hall` before it matches anything. Into every pane, plugin pane and plugin command it sets `HALL_SOCKET_PATH` (its own socket), `HALL_BIN_PATH` (the running hall) and `REEVE_BACKEND=hall`, after a launch's own env so nothing overrides them, and keeps herdr's outbound names (Environment names set into panes, below). Every herdr request the client rings for is remembered while its pane reads blocked, listed or not, so a hand's dialog in its first seconds, before reeve records its pane, does not ring again as `waiting` (decision 66's window B). The stand-in serves version 1. The test setup gains `--listing reeve`, which runs reeve's own listing on the test home and starts one test reeve with no hands placed; it runs end to end once reeve's side lands on reeve's main. Cut-over (the release build, `~/.config/hall/config.toml` naming reeve's listing, the liege launching it) is the reeve's, with the liege.

## Questions left for build time

- **How a reeve's turn ending is read.** Herdr's done sound already fires on turn end (fork-map section 4). Whether its idle and done reads are good enough is not settled: reeve's backend avoids herdr's `agent_status` because it latches, and uses `agent explain` instead (`reeve:backends/herdr.sh:539-542`); and a dialog claude paints full screen can read idle with no blocked rule, so it plays done or nothing instead of request. Step 4 waits on this. Settled before step 4 by decision 62: herdr's own transitions read every measured turn end and dialog once (the latch is only `agent_status` holding done until seen), and claude's manifest gains one rule for a full-screen consent with no known footer.
- **Building zig on this Mac.** Settled in step 1: Homebrew's `zig@0.15` (0.15.2, keg-only, at `/opt/homebrew/opt/zig@0.15/bin`) builds the vendored libghostty-vt and hall links against it with Rust 1.96.1, with no SDK workaround: the patched `.tbd` copy of fork-map section 6 was not needed and hall's build has none. Zig 0.15.2 from Homebrew and Rust 1.96.1 through rustup are installed beside the existing toolchain (decision 39).
- **Telling reeve workspaces from the rest.** The liege's other workspaces sit in the same list as reeves (fork-map section 2). Whether hall keeps them apart by the listing's session locations, by a token reeve pushes, or by something else is not settled. Settled in step 3 by decision 50: by the listing's session locations.
- **Environment names set into panes.** Decision 33 renames the inbound names. What hall sets into every pane now:
  - herdr's outbound names, unchanged: `HERDR_SOCKET_PATH` (hall's own socket), `HERDR_ENV=1`, `HERDR_PANE_ID`, `HERDR_TAB_ID`, `HERDR_WORKSPACE_ID`, `HERDR_BIN_PATH` (the hall binary) and the rest;
  - beside them, since step 1's repair (decision 44b): `HALL_ENV=1`, `HALL_PANE_ID`, `HALL_TAB_ID` and `HALL_WORKSPACE_ID`, in plugin panes and plugin commands too. Hall reads only these: `hall` typed in a hall pane is refused as nested, and a pane command's default target is `HALL_PANE_ID`, never a live herdr pane's `HERDR_PANE_ID`;
  - `HALL_SESSION`, inherited: `hall --session <name>` sets it in its own process environment (`apply_explicit_name`, `src/session.rs:448-455`), and the server and its panes inherit it, so a `hall` CLI call in a pane reaches that pane's own server.

  - since step 5 (decision 68): `HALL_SOCKET_PATH` (hall's own socket), `HALL_BIN_PATH` (the hall binary) and `REEVE_BACKEND=hall`, set after any launch's own env, in plugin panes and plugin commands too, so a reeve started in a hall pane uses reeve's hall backend and reaches its own hall by socket, whoever started the server.

  Whether the outbound `HERDR_*` names are renamed was open: renamed, the liege's installed claude hook goes silent in hall panes, so claude's state there rests on herdr's screen detection alone, and whether step 2's "a test reeve driven to a dialog reads so in hall" still holds then is not checked; kept, a Homebrew `herdr` call in a hall pane without `--session` reaches hall and is refused by the protocol bump (fork-map sections 1 and 3). Settled in step 5 (decision 68): kept. Reeve's hall backend reads only the `HALL_*` names, so the kept `HERDR_*` names steer nothing of reeve's.

  The gap, either way, for a process in a hall pane without step 2's guard: one that calls Homebrew `herdr` with `--session`, or runs `herdr session list`, reaches the live herdr. An explicit session beats `HERDR_SOCKET_PATH` (`active_api_socket_path`, `src/session.rs:173-180`), and `session list` reads `~/.config/herdr/sessions` and connects to each session's socket. Reeve's herdr backend does both on every call (`reeve:backends/herdr.sh:15-27`: `herdr session list --json`, then `herdr <sub> ... --session <default>`), so a test reeve or test hand running in a hall pane would drive the live `default`. Step 2 needed a guard against this settled before it started (decision 44f). Decision 47 chose it for the test setup: every test reeve and test hand runs with `XDG_CONFIG_HOME` pointed at a throwaway folder, the only input besides `HOME` to every Homebrew herdr socket path, so no herdr call from a test pane resolves to the live `~/.config/herdr`. A live reeve in hall at go-live needs its own answer, with the bridge (step 5). Step 5's answer: reeve's hall backend, selected by `REEVE_BACKEND=hall` in every hall pane, addresses hall by `HALL_SOCKET_PATH` and never runs `herdr` (decision 68). A Homebrew `herdr --session ...` or `herdr session list` typed by a model in a hall pane still reaches the live herdr; that gap is not closable from hall without a PATH shim, which login shells defeat (fork-map section 3).
- **How often to read the listing.** The listing is something hall asks for, not something pushed; the refresh interval is not settled, and it may lapse if reeve pushes state at go-live. Settled for development in step 3 by decision 49: the client runs the configured listing command once a second by default (`[reeve] listing_interval_ms`).

## Risks

- **A codebase hall did not write.** About 241k lines of Rust (decision 31). The client shell, where the reeve tree goes, is herdr's busiest `src/` area (`src/client` +15707 -2239 from v0.9.0 to v0.9.3), so every upstream port risks conflicts there; vendor and the crate split are the large part of any port past v0.9.0 (fork-map section 7).
- **Herdr's CLI tests do not run on a Mac.** `tests/cli.rs` and `tests/auto_detect.rs` are Linux-only, so on this Mac `cargo test` runs 3105 tests but none of the CLI integration suite (fork-map section 6). Separation patches to CLI paths need checking by hand here.
- **Tests that fall back to the real config.** Herdr's integration tests start real servers with a temporary `XDG_CONFIG_HOME` and an explicit socket path; the `HALL_*` rename must rename them in the same change, or they fall back to the real config folder (fork-map section 6).
- **A separation patch missed.** Any place that still names `herdr` (a folder, an inbound variable, a network path) shares state with the live install. The protocol bump turns a hall CLI call on a herdr socket into a refusal, and the endpoint generation does the same for a hall client on a herdr client socket (decision 44d).
