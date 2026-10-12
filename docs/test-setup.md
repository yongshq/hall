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

Every subcommand takes `--dry-run`: it prints every command it would run, each on a line starting `+ `, and runs none. Plumbing that only parses output already captured (`jq` over a herdr reply) is not printed, and neither is the home check below, which runs in a dry run too, since it reads only where the path leads and writes nothing.

The scripts need the stand-in built before their first run, because it checks the reeve home:

```sh
PATH=/opt/homebrew/opt/rustup/bin:$PATH cargo build -p hall-standin
```

Without it every subcommand refuses; there is no weaker check to fall back on.

## What the scripts refuse

- The reeve home `~/.reeve`, anything inside it, and any home whose directory name does not start with `.reeve-hall-dev`. Before any other step, every subcommand asks the stand-in, the one judge of a reeve home, first for the root check (`hall-standin check-root --home <path>`), and from then on uses the path it prints: the home as resolved on disk. It follows symlinks, and matches the live reeve home by the directory on disk, not by its spelling, so a case variant (`~/.REEVE`) or another route to it (`/System/Volumes/Data/Users/...`) is refused too. It rules out `~/.reeve` both for `HOME` and for the home the user database records, so a wrong `HOME` does not open it. A symlink whose target does not exist is refused, since where it leads cannot be checked. A `HOME` or user database home with `..` in it is refused as well, since where its `.reeve` sits then depends on who reads the path. The root check always blocks, for every subcommand.
- A home with anything inside it that could lead a read or a write out of it, or hang one. Next, `hall-standin check-home --home <path>` walks the whole home without following links, since the world file, the live baseline, each claude's working directory and what reeve and claude write there (`state`, `errands`, a project `.claude`) all sit inside it. It refuses a symlink anywhere inside; a regular file with more than one name (a hard link), since reeve's hooks append to files in `state` and an append through a hard link lands in the other name's file too; and anything that is neither a regular file nor a directory (a fifo, a socket, a device), since reading a fifo hangs. It refuses whatever it cannot list or look at (a directory with no read permission, say), and a home of more than 20000 entries or 32 directories deep rather than check it in part. `up` and `test-drive`, which write and launch, refuse such a home. `down` and `status` go on with a `WARNING` on stderr and in the inputs, and read and write nothing inside the home: `down` still stops (and with `--delete` deletes) the two test herdr sessions by their checked names but leaves the recorded panes in the world file, and `status` skips the baseline, the recorded panes and the listing, and says so for each. The stand-in's own commands (`listing`, `set`, `place`, `locate`, `sessions`) refuse such a home outright. So a home a test claude dirtied (`npm install` in its working directory makes symlinks, a Python venv too) can always be brought down; then remove it by hand and start fresh. Keep build trees and local git clones out of a test reeve home: `cargo build` and a `git clone` from a local path make hard links, so the home would then be refused.
- Writes through a symlink. The scripts refuse, on their own, to write a file or make a directory where that path or a directory between the home and it is a symlink, dangling or not.
- A reeve home with `.` or `..` in it, since `..` is only undone after the symlinks before it are followed. The stand-in refuses `..` on its own too.
- A reeve home with anything but letters, digits, `.`, `_`, `-` and `/` in it, as given or as resolved, since it goes into the command line each pane runs.
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

1. Rebuilds the stand-in (`cargo build -p hall-standin`, with keg-only rustup's bin put first on PATH), unless `HALL_STANDIN_BIN` names one. The first build is by hand (Inputs, above), since the home check needs it.
2. Reads `herdr session list --json`. If `hall-dev` or `hall-dev-2` is not running, it prints the command to start it and stops with exit status 3 (see "Starting a herdr session" below). Run it, detach, and run `up` again.
3. Seeds `<home>/standin/world.toml` from the roster if it does not exist yet; an existing world file is kept as it is.
4. Writes `<home>/standin/live-baseline.json` once: the live herdr session's entry in the session list (name, running, socket path, session dir), for `status` to compare against later.
5. Per roster line: creates `<home>/work/<name>`, prints claude's folder trust for it (below), and, unless the world file already records a pane that herdr still has in the right session, creates a workspace (`herdr workspace create --cwd <work dir> --label <name> --no-focus --session <s>`), runs claude in its root pane (`herdr pane run`), and records the pane with `hall-standin place <name> <s> <pane>`. A pane id herdr hands out that is not the stand-in's pane id form (letters, digits, `:`, `_`, `-`, not starting with `-`) stops `up` before `pane run`, so it never reaches herdr's command line as an option. So the listing's session locations are the panes herdr handed out, and a second `up` starts nothing twice.
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
- The live setup check: the live herdr session's entry in the session list, compared with up's baseline (`live session entry (name, socket, dir, running) unchanged since up's baseline; its panes are not checked`, or a `CHECK:` line), and that neither test session shares its socket.
- Per roster line: the recorded pane, confirmed with `herdr pane get` in its test herdr session, and herdr's reading of it from `herdr agent explain --json`.
- `~/.reeve/state`: any file mentioning the test home's path or a roster name (`grep -rlF`, `grep -rlwF`). `none` is the expected answer for both.
- Folder trust per work dir, and the listing.

Anything wrong is a line starting `CHECK:`. On a home that fails the check inside it (What the scripts refuse, above), `status` still runs, reading nothing inside the home.

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

Stops each test herdr session that is running (`herdr session stop <name> --json`), which ends every test reeve and test hand in it, and forgets every recorded pane in the world file (unless the home fails the check inside it, above: then the sessions are still stopped and the world file is left alone). Without `--delete`, herdr keeps each session's saved shape and restores it on the next start. The test reeve home and its world file are kept; remove `~/.reeve-hall-dev` by hand to start fresh.

`herdr session stop|delete <name>` carries no `--session`: the name is its argument, and herdr resolves its own connection for it, which under `env -u` is the default session's. Whether herdr then asks the default server to stop the named session, or reaches the named one directly, is not documented and was not tried. Check it once in the first supervised live run: after `down`, `herdr session list --json` should show `default` still running with the same entry as before.

## Setting states

Any hand state, question or flag is set by editing `<home>/standin/world.toml`, or with `hall-standin set` (see [listing-draft.md](listing-draft.md)):

```sh
target/debug/hall-standin --home ~/.reeve-hall-dev set ansel-fix state=blocked
target/debug/hall-standin --home ~/.reeve-hall-dev set bertil-probe question="Which base?" wedged=true
```

## Tests

`cargo test --workspace` (with `PATH=/opt/homebrew/opt/rustup/bin:$PATH`) runs:

- the stand-in's unit tests, and `tests/standin_cli.rs`: the listing's shape, every hand state and flag, a question present and absent, errands not in flight left out, `check-home`, and the refusals (a missing `--home`, `~/.reeve` directly, inside it, through a symlink and in another letter case, the user database's `~/.reeve` under a wrong `HOME`, a home not named `.reeve-hall-dev*`, a `..` in the home or in `HOME`, a symlink anywhere inside the home, from `standin` and `work` to a sibling `state` or `errands` and a deep `work/<name>/.claude` (with no write landing in the fake live home), a hard link to a file in the fake live home, a fifo for the world file (refused, not hung on), `check-root` passing a home the walk refuses, a `standin` or `work` it cannot list, a home past the walk's depth limit, the herdr session `default`, a pane id that could read as an option);
- `tests/scripts_dry_run.rs`: every subcommand of both scripts in `--dry-run`. Every herdr line runs under `env -u HERDR_SOCKET_PATH -u HERDR_SESSION` and ends `--session hall-dev` or `--session hall-dev-2` (or is `session list --json`, or `session stop|delete hall-dev*`); nothing printed mentions `default`; every claude line has `REEVE_HOME="<home>"` and the haiku model; nothing is created; `status` runs only reads; the only key ever sent is Esc; the refusals fire before any command is printed, including `~/.reeve` in another letter case, through a symlink, and from the user database under a wrong `HOME`, a home that resolves to a path with a quote or `$(...)` in it, a `..` in the home, the root check refusing `down` and `status` for a live home a claude dirtied, and a missing stand-in; a symlinked `standin`, `work`, `work/<name>`, `state`, `errands` or `work/<name>/.claude`, a `standin` or `work` it cannot list, and an npm-style link in `work/<name>` refusing `up` and `test-drive` while `down` (still stopping both test sessions by name) and `status` go on with a warning and nothing run inside the home; and, behind a stand-in that passes any home, `up` refusing to write or make a directory through a symlink, dangling or not;
- `tests/scripts_fake_herdr.rs`: both scripts run for real against a fake `herdr` on PATH and a fake HOME, all under `target/`, with `HERDR_SOCKET_PATH` and `HERDR_SESSION` set as inside a live pane. It checks that neither variable reaches herdr, that the listing's session locations are the panes herdr handed out, that a second `up` starts nothing, that `status` reports the live setup clean and flags a `~/.reeve/state` file mentioning the test setup, that `up` stops when a test session is not running, that `up` refuses a pane id like `--session=default` from `workspace create` before any `pane run`, that `up` with a symlinked `standin` or `work` makes no herdr call and writes nothing into the fake live home, that on a home with an npm-style link `up` refuses while `status` and `down` run, `down` stopping both test sessions and leaving the recorded panes alone, and that `up` behind a stand-in that passes any home still refuses to write the world file or the baseline through a dangling link, or to make a working directory through one, starting no claude.

The tests never start herdr or claude and never read or write the real `~/.reeve-hall-dev`.
