# Test setup

Hall is built and tested only against its test setup, never the live one (ADR [0003](adr/0003-separate-from-live-setup.md), decisions 10, 23, 24 and 26). This page is how to bring it up and down, drive a test reeve to a dialog or the end of a turn, and what is checked on the way. Plan step 1.

- Two herdr sessions, `hall-dev` and `hall-dev-2`.
- A test reeve home, `~/.reeve-hall-dev`.
- Test reeves and test hands: interactive `claude` sessions on `claude-haiku-4-5-20251001`, one per roster line, each in its own workspace in one of the two herdr sessions, each started in `<home>/work/<name>`.
- The stand-in, serving the draft listing from `<home>/standin/world.toml` ([listing-draft.md](listing-draft.md)).

## Inputs

`scripts/test-setup` and `scripts/test-drive` take three inputs: the reeve home and the two herdr session names. They come from `scripts/test-setup.conf`, which flags override:

| Input | Config | Flag | Value in the config |
|---|---|---|---|
| reeve home | `HALL_DEV_HOME` | `--home` | `~/.reeve-hall-dev` |
| first herdr session | `HALL_DEV_SESSION_1` | `--session-1` | `hall-dev` |
| second herdr session | `HALL_DEV_SESSION_2` | `--session-2` | `hall-dev-2` |

`--config <file>` names another config of the same shape. The scripts have no built-in values: an input missing from both is a usage error. Every run prints the inputs before doing anything.

The config also holds the roster, one line per test reeve or test hand (`reeve <name> <1|2>` or `errand <id> <1|2> <owner> <office>`). As shipped: test reeves `ansel` (hall-dev) and `bertil` (hall-dev-2); test hands `ansel-fix` (hall-dev), `ansel-review` (hall-dev-2, so a reeve has a hand in the other herdr session) and `bertil-probe` (hall-dev-2).

Every subcommand takes `--dry-run`: it prints every command it would run, each on a line starting `+ `, and runs none. Plumbing that only parses output already captured (`jq` over a herdr reply) is not printed.

## What the scripts refuse

- The reeve home `~/.reeve`, anything inside it, and any home whose directory name does not start with `.reeve-hall-dev` (symlinks followed).
- The herdr session `default` in any letter case, and any herdr session not named `hall-dev*`.

## Rules every command keeps

- **Every herdr call names its herdr session.** Each runs as `env -u HERDR_SOCKET_PATH -u HERDR_SESSION herdr ... --session <hall-dev or hall-dev-2>`. Inside a live herdr pane `HERDR_SOCKET_PATH` points at `default`, and herdr resolves `--session` before it, so the flag wins and the inherited variables are dropped as well. Two calls name a session differently, as herdr's CLI requires: `herdr session list --json` (a read of every session, used to check which are running and to compare the live one against a baseline) and `herdr session stop|delete <hall-dev*> --json`, whose session is the argument.
- **Never `herdr server stop`**, which stops whichever server it resolves. `down` uses `herdr session stop` on the two test sessions only.
- **Every test reeve and test hand names its reeve home** (decision 26). Each pane runs `cd "<home>/work/<name>" && REEVE_HOME="<home>" claude --model claude-haiku-4-5-20251001`.
- **A dialog is never answered with allow.** The only key the scripts ever send is Esc, which denies.

## Up

```sh
scripts/test-setup up            # or --dry-run first
```

1. Builds the stand-in (`cargo build -p hall-standin`, with keg-only rustup's bin put first on PATH).
2. Reads `herdr session list --json`. If `hall-dev` or `hall-dev-2` is not running, it prints the command to start it and stops with exit status 3 (see "Starting a herdr session" below). Run it, detach, and run `up` again.
3. Seeds `<home>/standin/world.toml` from the roster if it does not exist yet; an existing world file is kept as it is.
4. Writes `<home>/standin/live-baseline.json` once: the live herdr session's entry in the session list (name, running, socket path, session dir), for `status` to compare against later.
5. Per roster line: creates `<home>/work/<name>`, prints claude's folder trust for it (below), and, unless the world file already records a pane that herdr still has in the right session, creates a workspace (`herdr workspace create --cwd <work dir> --label <name> --no-focus --session <s>`), runs claude in its root pane (`herdr pane run`), and records the pane with `hall-standin place <name> <s> <pane>`. So the listing's session locations are the panes herdr handed out, and a second `up` starts nothing twice.
6. Prints the listing.

## Starting a herdr session (fact found)

Herdr 0.9.0 documents no way to start a named session without a client attached. `herdr --help` lists `herdr server` as "Run as headless server", and the CLI reference says it "runs the headless server explicitly ... for supervised or service-style setups", but nothing documents it together with `--session`, and it was not tried: if it ignored `--session` it would aim at the live server. The documented way is `--session <name>`, "Use or create a named persistent session", which attaches a client. So `up` prints, for each session not running:

```sh
env -u HERDR_SOCKET_PATH -u HERDR_SESSION herdr --session hall-dev
```

for the liege to run in a spare terminal (outside herdr is cleanest; the `env -u` keeps it right inside one too). Detach with ctrl+b q; the server keeps running with no client, at the headless size (`server.headless_cols`/`rows`, 120x40 by default). Everything after that goes through `--session`. Herdr's global config is shared with the live setup and is not touched; a test config would go through `HERDR_CONFIG_PATH`.

## Folder trust (fact found)

Claude records folder trust in `~/.claude.json`, as `projects["<dir>"].hasTrustDialogAccepted`. A new `<home>/work/<name>` has no entry, so claude opens its trust dialog there first, and herdr reads that as a dialog standing. `up` and `status` read the entry for each work dir and each of its parents, and print `accepted` or `NOT accepted`. Granting it is the liege's decision: answer the dialog in the session yourself (attach with `herdr --session hall-dev`). The scripts never write `~/.claude.json` and never answer the dialog. Note that claude itself adds an entry for each new directory it runs in, and keeps its transcripts under `~/.claude/projects/`; those are claude's own files, shared with the live setup by design, not reeve's.

## Status

```sh
scripts/test-setup status
```

Read only. It prints:

- `hall-dev` and `hall-dev-2` from the session list: running, socket path.
- The live setup check: the live herdr session's entry in the session list, compared with up's baseline (`unchanged since up's baseline`, or a `CHECK:` line), and that neither test session shares its socket.
- Per roster line: the recorded pane, confirmed with `herdr pane get` in its test herdr session, and herdr's reading of it from `herdr agent explain --json`.
- `~/.reeve/state`: any file mentioning the test home's path or a roster name (`grep -rlF`, `grep -rlwF`). `none` is the expected answer for both.
- Folder trust per work dir, and the listing.

Anything wrong is a line starting `CHECK:`.

What `status` cannot check: which panes the live herdr session holds. That needs a herdr call naming `default`, which the test setup never makes. Instead every pane the test setup made is recorded with its herdr session and confirmed there, and the live session's session-list entry is compared with the baseline.

## Driving a test reeve

```sh
scripts/test-drive ansel dialog     # to a dialog
scripts/test-drive ansel turn-end   # to the end of a turn
scripts/test-drive ansel deny       # deny a standing dialog with Esc
```

- `dialog` prompts the test reeve to create a file in its work dir (`herdr agent prompt`), which claude's default permissions ask about, waits up to two minutes for herdr to read it as blocked (`herdr agent wait --until blocked`), and prints the reading. The dialog is left standing, so hall can alert on it; `deny` or `turn-end` clears it.
- `turn-end` reads the session first and, if a dialog stands, denies it with Esc. Then it prompts a direct question with `herdr agent prompt --wait`, which waits for observed activity before it settles, so an agent already idle cannot end the wait before the turn starts. Then it prints the reading.
- `deny` sends Esc and prints the reading.

How the reading is printed, from `agent explain --json`, the same signal reeve's own backend relies on, because `agent_status` latches:

| state | `live_prompt_box` | printed as |
|---|---|---|
| `blocked` | any | at a dialog (herdr: blocked) |
| `working` | any | working |
| `idle` or `done` | matched | ready for input (turn ended) |
| `idle` or `done` | not matched | covered composer: a dialog is likely on screen |

The last row matters: a dialog claude paints over the whole screen can read as `idle` with no `blocked` rule matched (found in reeve's backend). Then `agent wait --until blocked` times out, and the reading still shows the dialog.

## Down

```sh
scripts/test-setup down             # stop both test herdr sessions
scripts/test-setup down --delete    # and delete their saved state
```

Stops each test herdr session that is running (`herdr session stop <name> --json`), which ends every test reeve and test hand in it, and forgets every recorded pane in the world file. Without `--delete`, herdr keeps each session's saved shape and restores it on the next start. The test reeve home and its world file are kept; remove `~/.reeve-hall-dev` by hand to start fresh.

## Setting states

Any hand state, question or flag is set by editing `<home>/standin/world.toml`, or with `hall-standin set` (see [listing-draft.md](listing-draft.md)):

```sh
target/debug/hall-standin --home ~/.reeve-hall-dev set ansel-fix state=blocked
target/debug/hall-standin --home ~/.reeve-hall-dev set bertil-probe question="Which base?" wedged=true
```

## Tests

`cargo test --workspace` (with `PATH=/opt/homebrew/opt/rustup/bin:$PATH`) runs:

- the stand-in's unit tests, and `tests/standin_cli.rs`: the listing's shape, every hand state and flag, a question present and absent, errands not in flight left out, and the refusals (a missing `--home`, `~/.reeve` directly, inside it and through a symlink, a home not named `.reeve-hall-dev*`, the herdr session `default`);
- `tests/scripts_dry_run.rs`: every subcommand of both scripts in `--dry-run`. Every herdr line runs under `env -u HERDR_SOCKET_PATH -u HERDR_SESSION` and ends `--session hall-dev` or `--session hall-dev-2` (or is `session list --json`, or `session stop|delete hall-dev*`); nothing printed mentions `default`; every claude line has `REEVE_HOME="<home>"` and the haiku model; nothing is created; `status` runs only reads; the only key ever sent is Esc; the refusals fire before any command is printed;
- `tests/scripts_fake_herdr.rs`: both scripts run for real against a fake `herdr` on PATH and a fake HOME, all under `target/`, with `HERDR_SOCKET_PATH` and `HERDR_SESSION` set as inside a live pane. It checks that neither variable reaches herdr, that the listing's session locations are the panes herdr handed out, that a second `up` starts nothing, that `status` reports the live setup clean and flags a `~/.reeve/state` file mentioning the test setup, and that `up` stops when a test session is not running.

The tests never start herdr or claude and never read or write the real `~/.reeve-hall-dev`.
