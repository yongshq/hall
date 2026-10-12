# Hall: version one plan

Terms are the ones in [GLOSSARY.md](../GLOSSARY.md). The decisions behind this plan are in [decisions.md](decisions.md) and the [decision records](adr/); the one that shapes it is decision 31, ADR [0004](adr/0004-fork-of-herdr.md): hall is a fork of herdr. Facts come from three reports kept in this repository: [research/herdr-launch.md](research/herdr-launch.md) (cited as "launch section N"), [research/fork-map.md](research/fork-map.md) ("fork-map section N") and [research/herdr-surface.md](research/herdr-surface.md) ("research section N"). Paths into herdr's code are at v0.9.0.

## Goal

Hall is the liege's audience hall: one screen over every live reeve, each reeve at the top level of the sidebar with its hands in flight beneath it, a hand state on every hand and a waiting-on-you badge on every reeve. The liege types `hall` and nothing else: it starts and attaches like herdr, detaches, and a second `hall` reattaches (decision 30). Picking a reeve or a hand shows its session, and the liege types into it as in herdr. When a hand becomes stuck or a reeve needs the liege, hall raises an alert. Hall is herdr, forked from v0.9.0 and renamed, sharing nothing with the liege's installed herdr (decision 31); reeve stays the only source of errand state (ADR [0002](adr/0002-state-through-reeve.md)).

## Version one scope

In:

- Hall starts like herdr: `hall` starts its server out of sight if none is running, attaches, detaches and reattaches (decision 30). Sessions, panes, input, the prefix key ctrl+b (decision 14) and scrollback come with herdr.
- Separate from the live herdr: own config and state folders, binary, inbound environment names and protocol number (decision 33).
- Reeve tree in the sidebar: every reeve in hall's one session (decisions 6 and 38), reeve = workspace and hand = tab (decision 34), errands in flight only (decision 16), a hand state mark per hand (decision 12). Workspaces that are not reeves are kept apart from the reeve list.
- Badges: a count per reeve of what is on disk: a hand's unanswered question, a hand waiting, blocked, idle or wedged (decision 17).
- Alerts with sound: herdr's own request and done sounds for a session at a dialog or ending its turn, every reeve reply included (decisions 19, 22, 35); hall adds a sound and a highlight on the reeve when a hand becomes blocked, idle, wedged or asks a question, which herdr cannot see. The sound can be changed or muted.

Out:

- Household actions (dispatching, answering, cleaning up errands): these stay with the reeve (decision 8).
- The reeve's full waiting list: the badge counts only what is on disk until a reeve change the liege approves (decision 17).
- The reeve side of the bridge (a hall backend in reeve, and whether reeve pushes hand state to hall or hall reads the listing): decided with the liege at go-live, step 5 (decision 36).
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
  - protocol number bumped off 22, so a herdr CLI, client or observer that reaches a hall socket, or the reverse, fails loudly at the guard;
  - `update` and self-replace, the agent-manifest fetch, `--remote` install and `integration install` refused (`hall integration install` would overwrite the liege's `~/.claude/hooks/herdr-agent-state.sh`);
  - worktree folder and pane-graphics paths renamed.
- **Reeve tree.** A client-side sidebar change: tab rows under each workspace row, plus selection and hit testing (fork-map section 2, option A). Kept in few, new functions, because the client shell is herdr's busiest `src/` area (fork-map section 7).
- **Reeves and errands from reeve's listing** (ADR 0002). Hall never reads status files. During development the stand-in serves the listing from the test setup (decision 11).
- **Sound.** Herdr's own sounds, played by the client driving the session, which is the hall window (decision 35; fork-map section 4). Hall adds the triggers that come from reeve's state.

## Listing format hall needs from reeve

**Draft.** These are the fields hall needs, not a settled format. The format will be owned by a reeve change that goes to the liege for approval at go-live (decisions 11 and 36, ADR 0002). Until then the stand-in serves it ([listing-draft.md](listing-draft.md)).

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

The stand-in's field is still named `herdr_session`; from step 2 it holds a hall session name (fork-map section 8).

## Test setup

Hall is built and tested only against the test setup (decisions 10 and 38, ADR [0003](adr/0003-separate-from-live-setup.md)):

- One hall session, `hall-dev`: a hall server with its own socket, run by the debug build (`target/debug/hall`). The debug build's folder is `~/.config/hall-dev`, so the session sits at `~/.config/hall-dev/sessions/hall-dev/`. Live use is the release build, whose folder is `~/.config/hall`; a release `hall` does not list the test session. One session is enough, since once hall is in use every reeve lives in hall's own session (decision 38).
- A reeve home at `~/.reeve-hall-dev`. Every test reeve and test hand starts with `REEVE_HOME=~/.reeve-hall-dev` on its own command line (decision 26): the liege's reeve plugin hooks run in every claude session, in hall panes too.
- Test reeves and test hands: claude sessions, each test reeve in its own workspace and its test hands as tabs in that workspace, placed by the scripts, since test reeves do not dispatch through hall before go-live (decision 36).
- The stand-in: serves the draft listing for them from `~/.reeve-hall-dev`.

Until step 2 the test setup is the one built for herdr ([test-setup.md](test-setup.md)): herdr sessions `hall-dev` and `hall-dev-2`, every herdr call naming its session (decision 24). It was never brought up live, so no herdr session of its exists and none needs deleting. Step 2 moves it to hall with the one session `hall-dev` and drops `hall-dev-2` (decision 38). From step 2 on, the test setup makes no herdr call at all and creates no herdr session (decision 33).

**Hard rule: never the live herdr, never `~/.reeve`.** The liege's herdr (its session `default`, its folders and sockets) and `~/.reeve` belong to the live setup, which stays untouched while hall is developed (decision 9).

**The "live setup unchanged" check** (decision 28, reworded for the fork). Once hall has its own folders, hall never sees herdr's sessions and hall's own `default` is not the live one, so decision 28's guards are partly moot (fork-map section 8). The live herdr rewrites files in its own folders whenever the liege uses it (its `default` session keeps its data in the config folder itself, `src/session.rs:162-166`), so the check never compares those folders as a whole. What is checked:

- nothing hall created under herdr's folders: no path named `hall*` under `~/.config/herdr` or `~/.local/state/herdr`;
- the live `default` entry in `herdr session list --json` unchanged against a baseline the check records on its first run (decision 28). `herdr session list` reads herdr's config folder and talks to no server (fork-map section 3); nothing names `default`;
- no hall process holding a herdr socket;
- no file under `~/.reeve/state` names the test home's path or a test reeve or errand name.

The check is its own tool, `scripts/live-check`, separate from the test setup scripts; step 1 builds it. Its one herdr call is that `herdr session list --json` read, which is exempt from "the test setup makes no herdr call" above. Until step 1 lands, the check is the one [test-setup.md](test-setup.md) runs today: default's entry in the session list and `~/.reeve/state`.

## Steps

Each step works end to end and can be shown to the liege on its own. Steps 1 to 4 work only in `hall-dev` and `~/.reeve-hall-dev` (or, for step 1, hall sessions of its own), and each ends with the "live setup unchanged" check. Step 5 is exempt: it is the cut-over, which changes reeve and starts real reeves in hall, with the liege's approval (decision 9).

### 1. Fork and separate

- **Outcome:** herdr v0.9.0 merged at hall's root (decision 32), with the stand-in beside it in one cargo workspace; the separation patches (decision 33); herdr's `LICENSE` and both vendored MIT licences kept, a `NOTICE` line naming hall as a modified herdr, `MODIFICATIONS.md` listing the changed files; herdr's `AGENTS.md` replaced by hall's own (herdr's kept as a reference file) and `build.rs`'s external-contributor warning dropped; the build working on this Mac (decision 37); herdr's own test suite passing under the new names; the "live setup unchanged" check built as `scripts/live-check` (above).
- **Shown working:** `hall` opens, detaches and reattaches exactly like herdr; a second `hall` reattaches to the same server; a herdr client or CLI pointed at a hall socket is refused with a protocol mismatch, and so is hall pointed at a herdr socket. For the reverse the herdr socket is a throwaway herdr server, started from the Homebrew `herdr` binary with its own temporary `XDG_CONFIG_HOME` and `XDG_STATE_HOME` and with `HERDR_SOCKET_PATH` and `HERDR_SESSION` unset, and stopped after. Never the live `default`'s socket: every CLI request pings the server before the protocol check (`src/cli.rs:764-799`), so hall pointed there would reach the live server (decisions 9 and 28); `hall update` and `hall integration install` are refused; `cargo test --workspace` passes, herdr's tests and the stand-in's together; `scripts/live-check` passes: no `hall*` path under herdr's folders, default's entry in `herdr session list` unchanged, no hall process on a herdr socket, and nothing in `~/.reeve/state` naming the test setup.
- **Needs:** Rust 1.96.1 (herdr's pin) and zig 0.15.2, both installed (decision 39); the zig build question below.

### 2. Test setup on hall

- **Outcome:** the test setup scripts and the stand-in retargeted from herdr to the debug hall binary and the one hall session `hall-dev` (fork-map section 8, decision 38): the one `HALL_HERDR` array in `scripts/test-setup-lib.sh` and the `command -v herdr` check in `scripts/test-setup`, the fake binary in `tests/scripts_fake_herdr.rs`, and `env -u` of the `HALL_*` names instead of the `HERDR_*` ones. The second session is dropped: `HALL_DEV_SESSION_2`, `--session-2` and the session number in the roster. Test hands are placed by the scripts as tabs in their test reeve's workspace.
- **Shown working:** `scripts/test-setup up` brings up test reeves and test hands in `hall-dev`, each test hand a tab beside its test reeve; the stand-in's listing names every test reeve and errand with session locations that match; a test reeve driven to a dialog and to the end of a turn reads so in hall; `--dry-run` and the fake-binary tests show the test setup making no herdr call at all (`scripts/live-check` is a separate tool and keeps its one read); the "live setup unchanged" check passes.
- **Needs:** step 1.

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
- **Needs:** step 3 for the highlight on the reeve; step 2 for setting states and driving test reeves on demand. The turn-end question below is settled before this step starts.

### 5. Go-live bridge

- **Outcome:** the liege decides the reeve side of the bridge (decision 36): a `hall` backend in reeve or a smaller change to reeve's herdr backend (fork-map section 3), and whether reeve pushes hand state to hall as metadata tokens or hall reads the listing; with it, the listing format itself (decision 11). Then the cut-over: new reeves start inside hall. Reeves already in the live herdr's `default` finish there, since no pane moves between servers (launch section 4); until they do they stay visible in herdr, not in hall (decision 38).
- **Shown working:** a reeve started inside hall dispatches a hand, which opens as a tab in that reeve's workspace in hall and nowhere in the live herdr; its state, badge and alerts show in hall from the real reeve, not the stand-in; reeves still in the live herdr keep running until they finish.
- **Needs:** steps 1 to 4, and the liege's approval of the reeve change, which touches the live setup (decision 9).

## Questions left for build time

- **How a reeve's turn ending is read.** Herdr's done sound already fires on turn end (fork-map section 4). Whether its idle and done reads are good enough is not settled: reeve's backend avoids herdr's `agent_status` because it latches, and uses `agent explain` instead (`reeve:backends/herdr.sh:539-542`); and a dialog claude paints full screen can read idle with no blocked rule, so it plays done or nothing instead of request. Step 4 waits on this.
- **Building zig on this Mac.** Zig 0.15.2 from Homebrew's `zig@0.15` (keg-only, at `/opt/homebrew/opt/zig@0.15/bin`) and Rust 1.96.1 through rustup, beside the existing toolchain, are installed (decision 39). A hello world links with Homebrew's zig; building libghostty-vt with it has not been tried. If that fails to link, the SDK workaround (a patched copy of the SDK's `.tbd` files, fork-map section 6) lives in hall's own build scripts, with nothing further installed (decision 39). Step 1 waits on this.
- **Telling reeve workspaces from the rest.** The liege's other workspaces sit in the same list as reeves (fork-map section 2). Whether hall keeps them apart by the listing's session locations, by a token reeve pushes, or by something else is not settled.
- **Environment names set into panes.** Decision 33 renames the inbound names. Whether the names hall sets into every pane (`HERDR_SOCKET_PATH`, `HERDR_PANE_ID` and the rest) are renamed too is open: renamed, the liege's installed claude hook goes silent in hall panes, so claude's state there rests on herdr's screen detection alone, and whether step 2's "a test reeve driven to a dialog reads so in hall" still holds then is not checked; kept, a Homebrew `herdr` call in a hall pane without `--session` reaches hall and is refused only by the protocol bump (fork-map sections 1 and 3). Tied to the bridge (step 5).
- **How often to read the listing.** The listing is something hall asks for, not something pushed; the refresh interval is not settled, and it may lapse if reeve pushes state at go-live.

## Risks

- **A codebase hall did not write.** About 241k lines of Rust (decision 31). The client shell, where the reeve tree goes, is herdr's busiest `src/` area (`src/client` +15707 -2239 from v0.9.0 to v0.9.3), so every upstream port risks conflicts there; vendor and the crate split are the large part of any port past v0.9.0 (fork-map section 7).
- **Herdr's CLI tests do not run on a Mac.** `tests/cli.rs` and `tests/auto_detect.rs` are Linux-only, so on this Mac `cargo test` runs 3105 tests but none of the CLI integration suite (fork-map section 6). Separation patches to CLI paths need checking by hand here.
- **Tests that fall back to the real config.** Herdr's integration tests start real servers with a temporary `XDG_CONFIG_HOME` and an explicit socket path; the `HALL_*` rename must rename them in the same change, or they fall back to the real config folder (fork-map section 6).
- **A separation patch missed.** Any place that still names `herdr` (a folder, an inbound variable, a network path) shares state with the live install. The protocol bump turns the worst of these, a hall client on a herdr socket, into a loud refusal.
