# Hall: version one plan

Terms are the ones in [GLOSSARY.md](../GLOSSARY.md). The decisions behind this plan are in [decisions.md](decisions.md) and the [decision records](adr/). Facts about herdr come from two places: the herdr-surface report, kept in this repository at [research/herdr-surface.md](research/herdr-surface.md) and cited below as "research section N"; and herdr's own documentation for version 0.9.3, cited by page and section, with the page links collected [at the end](#herdr-documentation-cited).

## Goal

Hall is the liege's audience hall: one screen over every live reeve, each reeve at the top level of the reeve tree with its hands in flight beneath it, a hand state on every hand and a waiting-on-you badge on every reeve. Picking a reeve or a hand shows its session live, and the liege can type into that session from hall. When a hand becomes stuck or a reeve needs the liege, hall raises an alert. Herdr stays the backend that keeps every session alive; reeve stays the only source of errand state.

## Version one scope

In:

- See and talk: the reeve tree, and any session shown and typed into from hall (decision 8).
- Reeve tree: every reeve on the machine, whichever herdr session it sits in (decision 6), hands nested beneath their reeve, errands in flight only (decision 16), a hand state mark per hand (decision 12).
- Live session view: the picked session fills the screen to the right of the reeve tree, with one line of key help at the bottom (decision 12).
- Typing, under control rules: hall takes a session's input only while that session is shown and focused in hall, and hands it back on moving away (decision 15).
- Badges: a count per reeve of what is on disk: a hand's unanswered question, a hand waiting, blocked, idle or wedged (decision 17).
- Alerts with sound: a sound (configurable, can be muted) and a highlight on the reeve when a hand becomes stuck or asks a question, or when a reeve's own session stands at a dialog or ends its turn (decision 19). Turn end means the end of every reeve reply, not only replies that ask the liege something (decision 22).
- Prefix key: ctrl+b then a key for hall's own commands; every other key goes to the session. The prefix is configurable (decision 14).

Out:

- Household actions (dispatching, answering, cleaning up errands): these stay with the reeve (decision 8).
- Starting, stopping or reconfiguring herdr: hall attaches to a backend the liege already has up (decision 13).
- The reeve's full waiting list: the badge counts only what is on disk until a reeve change the liege approves (decision 17).
- Narrowing the turn-end alert to replies that ask the liege something: later, if the alert proves noisy, and it needs a signal reeve does not record today (decision 22).
- Publishing: the repository stays private and local, with no remote, until version one works (decision 18).
- Alerts beyond the sound and the highlight: no desktop messages and no use of herdr's own toasts or sounds.

## Architecture sketch

No code and no library choices yet. This is the shape the steps below build toward.

- **Hall process.** One full-screen program the liege starts from their own terminal (decision 13). It holds the reeve tree, the session view and the key handling, and draws them itself.
- **Herdr, through its documented API only (socket methods and the documented CLI)** (ADR [0001](adr/0001-own-app-on-herdr.md)). Newline-delimited JSON over a Unix socket per herdr session; named herdr sessions live at `~/.config/herdr/sessions/<name>/herdr.sock` (research section 1; [Socket API][socket-api], Socket transport and Socket paths). Hall uses:
  - `session.snapshot` to learn the sessions and their states at start and after any gap (research section 1; [Socket API][socket-api], Raw methods).
  - `events.subscribe` for pushed changes, treated as invalidations of the snapshot, never as a sequence of their own ([Socket API][socket-api], Event subscriptions).
  - `herdr terminal session observe` for a read-only live stream of a session: NDJSON `terminal.frame` records carrying base64 ANSI, a full frame first and partial frames after, rendered at the size the observer asks for (research section 2; [Persistence and remote access][persistence], Direct terminal attach).
  - `herdr terminal session control` for the same stream plus input, resize and release, with one controller at a time (research section 2; [CLI reference][cli], Direct terminal attach).
- **Off limits: anything not documented.** Hall uses only methods in the documented method table or named in documented prose ([Socket API][socket-api], Raw methods), and the documented CLI. Present in the 0.9.0 schema but not documented: `client_shell.surface.set`, `command.invoke`, `server.live_handoff`, `pane.focus`, `pane.copy_*`, `pane.scroll`, `pane.selection.read`, `pane.link.activate`, `pane.edit_scrollback`, `integration.list` and the `*.dismiss` methods (research section 1). `pane.graphics.*` is in the 0.9.0 schema but documented as removed, with no replacement ([Socket API][socket-api], Raw methods). Herdr's internal client protocol is undocumented too (research section 2; [Socket API][socket-api], Protocol stability). Hall uses none of them.
- **Reeves and errands from reeve's listing** (ADR [0002](adr/0002-state-through-reeve.md)). Hall never reads status files. It asks for a machine-readable listing; during development the stand-in serves that listing from the test setup (decision 11, ADR [0003](adr/0003-separate-from-live-setup.md)).
- **Reeve's own session state from herdr, for alerts only** (decision 19). Whether a reeve's session stands at a dialog or has ended its turn comes from herdr's agent state, and is never shown or used as errand state. Herdr has agent state only for sessions it detects as agents (claude, codex and others); anything else is a plain terminal to it (research section 7; [Agents][agents], Supported agents).
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

Hall is built and tested only against the test setup (decisions 10 and 23, ADR 0003):

- Two herdr sessions, `hall-dev` and `hall-dev-2`, each a separate herdr server with its own socket (research section 5; [Persistence and remote access][persistence], Named sessions). Two, so that reeves across herdr sessions (decision 6) are shown before live use.
- A reeve home at `~/.reeve-hall-dev`.
- Test reeves and test hands: agent sessions herdr detects (claude, or another detected agent), spread across `hall-dev` and `hall-dev-2`, described by the listing the stand-in serves from `~/.reeve-hall-dev`. A plain shell will not do: herdr gives it no agent state, so it can never stand at a dialog or end a turn (research section 7; [Agents][agents], Supported agents). A test reeve can be driven to a dialog by prompting it to do something its permission settings ask about, and to the end of a turn by prompting it with something it answers directly; `herdr agent prompt` submits a prompt and `herdr agent wait` waits for the settled state ([CLI reference][cli], Agents).
- The stand-in: serves the draft listing for the test reeves and test hands from `~/.reeve-hall-dev`.

**Hard rule: never the herdr session `default`, never `~/.reeve`.** Those belong to the live setup, which the liege uses for the household's real work and which stays untouched while hall is developed (decision 9). Hall takes the herdr sessions and the reeve home it works against explicitly, and has no built-in fallback to either live one.

How "explicitly" is met (decision 24, ADR 0003 consequences):

- **Every herdr call names its herdr session.** Hall's own calls, the stand-in's and every script that sets up or drives the test setup either pass `--session hall-dev` (or `--session hall-dev-2`) on the command line, or connect to `~/.config/herdr/sessions/<name>/herdr.sock` directly. Never `HERDR_SESSION`, never an inherited `HERDR_SOCKET_PATH`. Herdr resolves the session in this order: explicit `--session <name>`, then `HERDR_SOCKET_PATH`, then `HERDR_SESSION`, then the default session ([Socket API][socket-api], Socket paths). Every process in a herdr pane inherits a `HERDR_SOCKET_PATH` pointing at its own server ([Add Herdr support to your agent][add-support], How it works), so a hand working inside the liege's `default` session that sets only `HERDR_SESSION=hall-dev` still lands on `default`.
- **Herdr's global config and plugins are shared.** A named herdr session still shares the same global config file ([Persistence and remote access][persistence], Named sessions), and installed plugins are global to the user across herdr sessions ([Plugins][plugins], Install and link). Never edit `~/.config/herdr/config.toml` or the plugin registry for the test setup; if a test config is needed, point `HERDR_CONFIG_PATH` at it ([CLI reference][cli], Environment variables).

## Steps

Each step works end to end and can be shown to the liege on its own. Every step that touches herdr or reeve works only in `hall-dev`, `hall-dev-2` and `~/.reeve-hall-dev`, and ends by checking the live setup is unchanged.

### 1. Test setup and stand-in

- **Outcome:** the `hall-dev` and `hall-dev-2` herdr sessions running, with test reeves and test hands in both; `~/.reeve-hall-dev`; and the stand-in printing the draft listing for them. Any hand state, question, or waiting, idle or wedged flag can be set by changing `~/.reeve-hall-dev`, and any test reeve can be driven to a dialog or to the end of a turn, so later steps can show every state and every alert on demand.
- **Shown working:** the stand-in's listing names every test reeve and errand, with session locations that match the sessions in `hall-dev` and `hall-dev-2`; a test reeve driven to a dialog reads as blocked in herdr, and after a turn reads as ready for input (`herdr agent explain`, with `--session`); the live setup is unchanged (the `default` session and `~/.reeve` as they were).
- **Needs:** nothing earlier.

### 2. Reeve tree from the stand-in

- **Outcome:** hall starts against `hall-dev`, `hall-dev-2` and `~/.reeve-hall-dev`, each named explicitly, reads the stand-in's listing and draws the reeve tree: every test reeve from both herdr sessions, its hands in flight beneath it, a hand state mark per hand. Moving through the tree with keys works; the key help line shows at the bottom.
- **Shown working:** test reeves in `hall-dev` and in `hall-dev-2` appear side by side in one reeve tree; changing a test hand's state in `~/.reeve-hall-dev` changes its mark in hall; finishing and cleaning up an errand drops the hand from the tree; the `default` session and `~/.reeve` as they were.
- **Needs:** step 1.

### 3. Watching a session live

- **Outcome:** picking a reeve or a hand shows its session in the right of the screen, live, read-only, through `herdr terminal session observe` with `--session` naming the herdr session the listing gives, sized to the part of the screen hall gives it.
- **Shown working:** output in a test reeve's session in `hall-dev` appears in hall as it happens; resizing hall's terminal redraws the session at the new size; moving to a test hand in `hall-dev-2` switches the view to it; the `default` session and `~/.reeve` as they were.
- **Needs:** step 2 for picking, step 1 for sessions to watch.

### 4. Typing into it

- **Outcome:** with a session shown and focused, keys go to it through `herdr terminal session control` with `--session` naming its herdr session; ctrl+b (or the configured prefix) then a key runs hall's own commands instead. Moving away releases control.
- **Shown working:** typing a command into a test hand's session in `hall-dev` from hall runs it there, and the same in `hall-dev-2`; the prefix moves focus back to the reeve tree without sending anything to the session; after moving away, the session takes input from elsewhere again; the `default` session and `~/.reeve` as they were, with nothing typed into any session there.
- **Needs:** step 3.

### 5. Badges, highlight and alert sound

- **Outcome:** each reeve carries a waiting-on-you badge counting its hands' unanswered questions and its hands waiting, blocked, idle or wedged. When a hand becomes stuck or asks a question, or a reeve's own session stands at a dialog or ends its turn (every reply, decision 22), hall plays the alert sound and highlights the reeve. Hall reads each reeve session's herdr agent state for this, in `hall-dev` and `hall-dev-2`. The sound can be changed or muted.
- **Shown working:** with no herdr client in the foreground, each trigger in decision 19 in turn raises the highlight and plays the sound:
  - a test hand set to waiting in `~/.reeve-hall-dev`;
  - a test hand set to blocked;
  - a test hand set to idle;
  - a test hand set to wedged;
  - a test hand with a new unanswered question;
  - a test reeve driven to a dialog;
  - a test reeve ending a turn, alerting on every reply.

  The hand triggers also raise their reeve's badge count; the reeve triggers come from a test reeve in `hall-dev` and from one in `hall-dev-2`. With the sound muted, a repeat of any trigger shows only the highlight. The `default` session, `~/.reeve` and herdr's global config as they were.
- **Needs:** step 2 for the listing and the tree; step 1 for setting states and driving test reeves on demand; the herdr agent-state reading, built in this step (`session.snapshot` and `events.subscribe` per herdr session, with one `pane.agent_status_changed` entry per reeve session). The question of how a reeve's turn ending is read (below) is settled before this step starts. Independent of steps 3 and 4.

## Questions left for build time

- **Frames and input: socket or CLI.** `observe` and `control` are documented as `herdr terminal session` commands printing NDJSON ([CLI reference][cli], Direct terminal attach); the research's method list has no socket method for them (research section 1). Whether hall drives the CLI for each shown session or a socket route exists is not established.
- **Which herdr sessions to attach to.** The reeve tree covers every reeve whichever herdr session it sits in (decision 6), and each herdr session has its own socket. Whether hall follows the session locations in the listing to every socket, or is told the set of herdr sessions, is not settled. Either way each one is named explicitly, as the test setup requires.
- **How a reeve's turn ending is read.** Reeve's backend avoids herdr's `agent_status` because it latches, and uses `agent explain` instead (research section 1, `backends/herdr.sh:539-542`). The alert for a reeve ending its turn, on every reply (decision 22), needs a reliable signal from herdr; which one is not settled. `pane.agent_status_changed` also needs one subscription entry per session, and one bad id rejects the whole request ([Socket API][socket-api], Event subscriptions). Step 5 waits on this.
- **How often to read the listing.** The listing is something hall asks for, not something pushed; the refresh interval, and whether herdr events should trigger a re-read, is not settled.
- **Control and size.** With `control`, hall owns resize; herdr says the last client to interact sets sizes on a shared view ([Concepts][concepts], Client and server). What the liege sees in a herdr client on the same session while hall holds control has not been tried (the research probed only `observe`, research Gaps).
- **Sending a literal ctrl+b.** Every key but the prefix goes to the session (decision 14), so ctrl+b itself can never reach one, and a session running readline or vim may need it. Herdr's own answer for direct attach is ctrl+b twice ([CLI reference][cli], Direct terminal attach); hall's is not chosen.
- **Synchronised output.** Frames start with `ESC[?2026h` (research section 2). Whether each candidate emulator handles it is to be checked when choosing one.
- **How the sound is played.** No mechanism has been looked at yet.
- **Herdr version.** Build and verify against the installed herdr 0.9.0, though the docs read are for 0.9.3. Upgrading is a Homebrew upgrade of the live setup's backend, after which the `default` server runs the old version until it restarts (research section 6), and decision 9 keeps the live setup untouched. So an upgrade is the liege's call, still open (the note after decision 25).

## Risks

- **Herdr's API is young.** Protocol 22, herdr 0.9.0 installed against 0.9.3 docs, with known drift: `pane.graphics.*` is in the 0.9.0 schema but removed in the 0.9.3 docs, and herdr's token style rules are documented but absent from 0.9.0 (research sections 1 and 4, Gaps). Hall keeps to documented methods and checks each one it uses against the installed version.
- **Event loss and resubscribe.** Event history is not durable; a slow reader gets `events_lost` and its connection is closed, and events share no sequence with snapshots ([Socket API][socket-api], Event subscriptions). Hall must resubscribe, take a fresh `session.snapshot`, and treat every event as a prompt to re-check, never as the state itself. Event payloads come from the schema and docs only; none were captured live (research Gaps).
- **One controller at a time.** `control` takes input and resize ownership for one client ([CLI reference][cli], Direct terminal attach). Hall holding control while the liege types into the same session elsewhere can surprise; the release on moving away (decision 15) is what keeps this small. `control` itself was not probed (research Gaps).
- **Sound without a herdr client in the foreground.** Herdr's sounds and `notification.show` need a foreground client (`no_foreground_client`, [Socket API][socket-api], Raw methods), so hall cannot lean on them and plays the sound itself (decision 19). If hall's own sound fails, a stuck hand goes unnoticed; the highlight is the fallback.

## Herdr documentation cited

All pages at herdr 0.9.3, the stable release when the research was done. Each link is the raw source of the page at that release, from herdr's documentation index.

- [Add Herdr support to your agent][add-support]
- [Agents][agents]
- [CLI reference][cli]
- [Concepts][concepts]
- [Persistence and remote access][persistence]
- [Plugins][plugins]
- [Socket API][socket-api]

[add-support]: https://raw.githubusercontent.com/herdrdev/herdr/v0.9.3/docs/next/website/src/content/docs/add-herdr-support.mdx
[agents]: https://raw.githubusercontent.com/herdrdev/herdr/v0.9.3/docs/next/website/src/content/docs/agents.mdx
[cli]: https://raw.githubusercontent.com/herdrdev/herdr/v0.9.3/docs/next/website/src/content/docs/cli-reference.mdx
[concepts]: https://raw.githubusercontent.com/herdrdev/herdr/v0.9.3/docs/next/website/src/content/docs/concepts.mdx
[persistence]: https://raw.githubusercontent.com/herdrdev/herdr/v0.9.3/docs/next/website/src/content/docs/persistence-remote.mdx
[plugins]: https://raw.githubusercontent.com/herdrdev/herdr/v0.9.3/docs/next/website/src/content/docs/plugins.mdx
[socket-api]: https://raw.githubusercontent.com/herdrdev/herdr/v0.9.3/docs/next/website/src/content/docs/socket-api.mdx
