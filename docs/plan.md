# Hall: version one plan

Terms are the ones in [GLOSSARY.md](../GLOSSARY.md). The decisions behind this plan are in [decisions.md](decisions.md) and the [decision records](adr/). Facts about herdr come from the herdr-surface report (`~/.reeve/errands/herdr-surface/report.md`, cited below as "report", with its own `full:<line>` references to the herdr docs).

## Goal

Hall is the liege's audience hall: one screen over every live reeve, each reeve at the top level of the reeve tree with its hands in flight beneath it, a hand state on every hand and a waiting-on-you badge on every reeve. Picking a reeve or a hand shows its session live, and the liege can type into that session from hall. When a hand becomes stuck or a reeve needs the liege, hall raises an alert. Herdr stays the backend that keeps every session alive; reeve stays the only source of errand state.

## Version one scope

In:

- See and talk: the reeve tree, and any session shown and typed into from hall (decision 8).
- Reeve tree: every reeve on the machine, whichever herdr session it sits in (decision 6), hands nested beneath their reeve, errands in flight only (decision 16), a hand state mark per hand (decision 12).
- Live session view: the picked session fills the screen to the right of the reeve tree, with one line of key help at the bottom (decision 12).
- Typing, under control rules: hall takes a session's input only while that session is shown and focused in hall, and hands it back on moving away (decision 15).
- Badges: a count per reeve of what is on disk: a hand's unanswered question, a hand waiting, blocked, idle or wedged (decision 17).
- Alerts with sound: a sound (configurable, can be muted) and a highlight on the reeve when a hand becomes stuck or asks a question, or when a reeve's own session stands at a dialog or ends its turn (decision 19).
- Prefix key: ctrl+b then a key for hall's own commands; every other key goes to the session. The prefix is configurable (decision 14).

Out:

- Household actions (dispatching, answering, cleaning up errands): these stay with the reeve (decision 8).
- Starting, stopping or reconfiguring herdr: hall attaches to a backend the liege already has up (decision 13).
- The reeve's full waiting list: the badge counts only what is on disk until a reeve change the liege approves (decision 17).
- Publishing: the repository stays private and local, with no remote, until version one works (decision 18).
- Alerts beyond the sound and the highlight: no desktop messages and no use of herdr's own toasts or sounds.

## Architecture sketch

No code and no library choices yet. This is the shape the steps below build toward.

- **Hall process.** One full-screen program the liege starts from their own terminal (decision 13). It holds the reeve tree, the session view and the key handling, and draws them itself.
- **Herdr, through its documented socket API only** (ADR [0001](adr/0001-own-app-on-herdr.md)). Newline-delimited JSON over a Unix socket per herdr session; named sessions live at `~/.config/herdr/sessions/<name>/herdr.sock` (report section 1, full:4964, full:4991-4992). Hall uses:
  - `session.snapshot` to learn the sessions and their states at start and after any gap (report section 1, full:4610-4612).
  - `events.subscribe` for pushed changes, treated as invalidations of the snapshot, never as a sequence of their own (full:5128-5148).
  - `herdr terminal session observe` for a read-only live stream of a session: NDJSON `terminal.frame` records carrying base64 ANSI, a full frame first and partial frames after, rendered at the size the observer asks for (report section 2, full:3988-3994).
  - `herdr terminal session control` for the same stream plus input, resize and release, with one controller at a time (report section 2, full:3996-4002).
- **Off limits:** undocumented herdr methods. The report names `client_shell.surface.set`, `command.invoke`, `server.live_handoff`, `pane.copy_*`, `pane.scroll`, `pane.selection.read`, `pane.link.activate`, `pane.edit_scrollback`, `integration.list` and the `*.dismiss` methods as present in the 0.9.0 schema but absent from the documented method table (report section 1, full:4589-4604). Herdr's internal client protocol is undocumented too (report section 2, full:5264-5266). Hall uses none of them.
- **Reeves and errands from reeve's listing** (ADR [0002](adr/0002-state-through-reeve.md)). Hall never reads status files. It asks for a machine-readable listing; during development a stand-in serves that listing from the test setup (decision 11, ADR [0003](adr/0003-separate-from-live-setup.md)).
- **Reeve's own session state from herdr, for alerts only** (decision 19). Whether a reeve's session stands at a dialog or has ended its turn comes from herdr's agent state, and is never shown or used as errand state.
- **ANSI frames to the screen.** Herdr sends ready-to-paint ANSI; hall must turn it into a grid of cells itself (ADR 0001, consequences). Candidates only, none chosen: `vt100` or `alacritty_terminal` for the terminal emulation, `ratatui` for drawing hall's own screen. Rust is the language (decision 5).
- **Sound.** Hall plays the alert sound itself, because herdr's own sounds need a herdr client in the foreground (decision 19).

## Listing format hall needs from reeve

**Draft.** These are the fields hall needs, not a settled format. The format will be owned by a reeve change that goes to the liege for approval when hall is ready for the live setup (decision 11, ADR 0002). Until then the stand-in serves it.

Per reeve:

| Field | Meaning |
|---|---|
| name | The reeve's name, as shown in the reeve tree. |
| session location | Which herdr session and which session inside it hold the reeve. |
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
| session location | Which herdr session and which session inside it hold the hand. |

## Test setup

Hall is built and tested only against the test setup (decision 10, ADR 0003):

- A herdr session named `hall-dev`, separate from the liege's, with its own socket (report section 5, full:3857-3881).
- A reeve home at `~/.reeve-hall-dev`.
- Stand-in reeves: sessions in `hall-dev` that stand for reeves and hands, described by the listing the stand-in serves from `~/.reeve-hall-dev`.

**Hard rule: never the herdr session `default`, never `~/.reeve`.** Those belong to the live setup, which the liege uses for the household's real work and which stays untouched while hall is developed (decision 9). Hall takes the herdr session and the reeve home it works against explicitly, and has no built-in fallback to either live one.

## Steps

Each step works end to end and can be shown to the liege on its own.

### 1. Test setup and stand-in

- **Outcome:** the `hall-dev` herdr session with a few stand-in reeves and hands in it, `~/.reeve-hall-dev`, and a stand-in that prints the draft listing for them. Any hand state can be set by changing the test home, so later steps can show every state on demand.
- **Shown working:** the stand-in's listing names every stand-in reeve and errand, with session locations that match the sessions in `hall-dev`; the live setup is unchanged (the `default` session and `~/.reeve` as they were).
- **Needs:** nothing earlier.

### 2. Reeve tree from the stand-in

- **Outcome:** hall starts, attaches to `hall-dev`, reads the stand-in's listing and draws the reeve tree: each reeve, its hands in flight beneath it, a hand state mark per hand. Moving through the tree with keys works; the key help line shows at the bottom.
- **Shown working:** changing a hand's state in the test home changes its mark in hall; finishing and cleaning up an errand drops the hand from the tree.
- **Needs:** step 1.

### 3. Watching a session live

- **Outcome:** picking a reeve or a hand shows its session in the right of the screen, live, read-only, through `herdr terminal session observe`, sized to the part of the screen hall gives it.
- **Shown working:** output in a stand-in session appears in hall as it happens; resizing hall's terminal redraws the session at the new size; moving to another entry in the tree switches the view.
- **Needs:** step 2 for picking, step 1 for sessions to watch.

### 4. Typing into it

- **Outcome:** with a session shown and focused, keys go to it through `herdr terminal session control`; ctrl+b (or the configured prefix) then a key runs hall's own commands instead. Moving away releases control.
- **Shown working:** typing a command into a stand-in session from hall runs it there; the prefix moves focus back to the reeve tree without sending anything to the session; after moving away, the session takes input from elsewhere again.
- **Needs:** step 3.

### 5. Badges, highlight and alert sound

- **Outcome:** each reeve carries a waiting-on-you badge counting its hands' unanswered questions and its hands waiting, blocked, idle or wedged. When a hand becomes stuck or asks a question, or a reeve's own session stands at a dialog or ends its turn, hall plays the alert sound and highlights the reeve. The sound can be changed or muted.
- **Shown working:** setting a stand-in hand to blocked in the test home raises the badge count, highlights its reeve and plays the sound, with no herdr client in the foreground; with the sound muted, only the highlight shows.
- **Needs:** step 2 for the listing and the tree; step 1 for setting states on demand. Independent of steps 3 and 4.

## Questions left for build time

- **Frames and input: socket or CLI.** `observe` and `control` are documented as `herdr terminal session` commands printing NDJSON (full:3988-4002); the report's method list has no socket method for them (report section 1). Whether hall drives the CLI for each shown session or a socket route exists is not established.
- **Which herdr sessions to attach to.** The reeve tree covers every reeve whichever herdr session it sits in (decision 6), and each herdr session has its own socket. Whether hall follows the session locations in the listing to every socket, or is told the set of sessions, is not settled.
- **How a reeve's turn ending is read.** Reeve's backend avoids herdr's `agent_status` because it latches, and uses `agent explain` instead (report section 1, `backends/herdr.sh:539-542`). The alert for a reeve ending its turn needs a reliable signal from herdr; which one is not settled. `pane.agent_status_changed` also needs one subscription entry per session, and one bad id rejects the whole request (full:5142).
- **How often to read the listing.** The listing is something hall asks for, not something pushed; the refresh interval, and whether herdr events should trigger a re-read, is not settled.
- **Control and size.** With `control`, hall owns resize; herdr says the last client to interact sets sizes on a shared view (full:1240-1244). What the liege sees in a herdr client on the same session while hall holds control has not been tried (the report probed only `observe`, report Gaps).
- **Synchronised output.** Frames start with `ESC[?2026h` (report section 2). Whether each candidate emulator handles it is to be checked when choosing one.
- **How the sound is played.** No mechanism has been looked at yet.
- **Herdr version.** The installed herdr is 0.9.0 and the docs read are for 0.9.3. Upgrading is a change to the live setup's backend (Homebrew, the old instance keeps going until restarted, report section 6), so whether and when to upgrade is the liege's call.

## Risks

- **Herdr's API is young.** Protocol 22, herdr 0.9.0 installed against 0.9.3 docs, with known drift: `pane.graphics.*` is in the 0.9.0 schema but removed in the 0.9.3 docs, and herdr's token style rules are documented but absent from 0.9.0 (report section 1, section 4, Gaps). Hall keeps to documented methods and checks each one it uses against the installed version.
- **Event loss and resubscribe.** Event history is not durable; a slow reader gets `events_lost` and its connection is closed, and events share no sequence with snapshots (full:5144-5148). Hall must resubscribe, take a fresh `session.snapshot`, and treat every event as a prompt to re-check, never as the state itself. Event payloads come from the schema and docs only; none were captured live (report Gaps).
- **One controller at a time.** `control` takes input and resize ownership for one client (full:3996-4002). Hall holding control while the liege types into the same session elsewhere can surprise; the release on moving away (decision 15) is what keeps this small. `control` itself was not probed (report Gaps).
- **Sound without a herdr client in the foreground.** Herdr's sounds and `notification.show` need a foreground client (`no_foreground_client`, full:4737-4757), so hall cannot lean on them and plays the sound itself (decision 19). If hall's own sound fails, a stuck hand goes unnoticed; the highlight is the fallback.
