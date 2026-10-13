# Test setup

Hall is built and tested only against its test setup, never the live one (ADR [0003](adr/0003-separate-from-live-setup.md), decisions 10, 26, 33, 38, 47 and 48). This page is how to bring it up and down, drive a test reeve to a dialog or the end of a turn, and what is checked on the way. Plan step 2 moved it from herdr to hall: it makes no herdr call at all.

- One hall session, `hall-dev`, run by the debug hall binary (`target/debug/hall`), whose folders are `~/.config/hall-dev` and `~/.local/state/hall-dev`. The session sits at `~/.config/hall-dev/sessions/hall-dev/`.
- A test reeve home, `~/.reeve-hall-dev`.
- Test reeves and test hands: interactive `claude` sessions on `claude-haiku-4-5-20251001`, one per roster line, each started in `<home>/work/<name>`. Each test reeve sits in a workspace of its own; each test hand is a tab in its test reeve's workspace (reeve = workspace, hand = tab, decision 34), placed by the scripts, since test reeves do not dispatch through hall before go-live (decision 36).
- The stand-in, serving the draft listing from `<home>/standin/world.toml` ([listing-draft.md](listing-draft.md)). Its `herdr_session` field holds the hall session's name.

## Inputs

`scripts/test-setup` and `scripts/test-drive` take two inputs: the reeve home and the hall session name. They come from `scripts/test-setup.conf`, which flags override:

| Input | Config | Flag | Value in the config |
|---|---|---|---|
| reeve home | `HALL_DEV_HOME` | `--home` | `~/.reeve-hall-dev` |
| hall session | `HALL_DEV_SESSION` | `--session` | `hall-dev` |

`--config <file>` names another config of the same shape. The scripts have no built-in values: an input missing from both is a usage error. Every run prints the inputs, and the hall binary it calls, before doing anything.

The config also holds the roster, one line per test reeve or test hand (`reeve <name>` or `errand <id> <owner reeve> <office>`). As shipped: test reeves `ansel` and `bertil`; test hands `ansel-fix` and `ansel-review` (tabs in `ansel`'s workspace) and `bertil-probe` (a tab in `bertil`'s). A line in the old two-session form, with a session number, is refused.

Every subcommand takes `--dry-run`: it prints every command it would run, each on a line starting `+ `, and runs none. Plumbing that only parses output already captured (`jq` over a hall reply) is not printed, and neither is the home check below, which runs in a dry run too, since it reads only where the path leads and writes nothing.

The scripts need the stand-in built before their first run, because it checks the reeve home:

```sh
PATH=/opt/homebrew/opt/rustup/bin:$PATH cargo build -p hall-standin
```

Without it every subcommand refuses; there is no weaker check to fall back on. `HALL_STANDIN_BIN` and `HALL_DEBUG_BIN` name a prebuilt stand-in and hall instead (the tests use them); then `up` builds neither.

## What the scripts refuse

- The reeve home `~/.reeve`, anything inside it, and any home whose directory name does not start with `.reeve-hall-dev`. Before any other step, every subcommand asks the stand-in, the one judge of a reeve home, first for the root check (`hall-standin check-root --home <path>`), and from then on uses the path it prints: the home as resolved on disk. It follows symlinks, and matches the live reeve home by the directory on disk, not by its spelling, so a case variant (`~/.REEVE`) or another route to it (`/System/Volumes/Data/Users/...`) is refused too. It rules out `~/.reeve` both for `HOME` and for the home the user database records, so a wrong `HOME` does not open it. A symlink whose target does not exist is refused, since where it leads cannot be checked. A `HOME` or user database home with `..` in it is refused as well, since where its `.reeve` sits then depends on who reads the path. The root check always blocks, for every subcommand.
- A home with anything inside it that could lead a read or a write out of it, or hang one. Next, `hall-standin check-home --home <path>` walks the whole home without following links, since the world file, the throwaway config folder, each claude's working directory and what reeve and claude write there (`state`, `errands`, a project `.claude`) all sit inside it. It refuses a symlink anywhere inside; a regular file with more than one name (a hard link), since reeve's hooks append to files in `state` and an append through a hard link lands in the other name's file too; and anything that is neither a regular file nor a directory (a fifo, a socket, a device), since reading a fifo hangs. It refuses whatever it cannot list or look at (a directory with no read permission, say), and a home of more than 20000 entries or 32 directories deep rather than check it in part. Inside `<home>/xdg-config` and `<home>/xdg-state` it walks only herdr's own folder, `herdr` in any letter case (on the Mac's case-insensitive disk `Herdr` is the same folder), in full, and skips the rest (decision 48's amendments): those are the test claudes' `XDG_CONFIG_HOME` and `XDG_STATE_HOME`, where third-party tools write (the liege's shell init runs `fnm` in every claude Bash call, and it makes a symlink per shell under `xdg-state/fnm_multishells`). Herdr's folder is walked because herdr, and reeve's herdr backend through it, finds its config, sessions and state there, so a link at or inside `xdg-config/herdr` or `xdg-state/herdr` would lead a herdr call from a test pane to the live herdr. Each of the two folders, and `herdr` in each, must still be a real directory, not a link. `up` and `test-drive`, which write and launch, refuse such a home. `down` and `status` go on with a `WARNING` on stderr and in the inputs, and read and write nothing inside the home: `down` still stops (and with `--delete` deletes) the test hall session by its checked name but leaves the recorded panes in the world file, and `status` skips the recorded panes and the listing, and says so for each. The stand-in's own commands (`listing`, `set`, `place`, `locate`, `sessions`) refuse such a home outright. So a home a test claude dirtied (`npm install` in its working directory makes symlinks, a Python venv too) can always be brought down; then remove it by hand and start fresh. Keep build trees and local git clones out of a test reeve home: `cargo build` and a `git clone` from a local path make hard links, so the home would then be refused. Reeve also makes a hard link for an instant in two places a test reeve writes: the wake spool (`ln` then `rm` under `state/sessions/<sid>/wake`) and the sentry lock (`state/.sentry.lock.new.<pid>` linked to `state/.sentry.lock`). So `up` or `test-drive` can refuse once, naming either name of the pair: a file under `state/sessions/*/wake/` (`.new.*` or the spool entry beside it), or `state/.sentry.lock` or `state/.sentry.lock.new.<pid>`. Run it again and it passes. If a writer was killed mid-write, its `.new.*` name stays linked: remove only the `.new.*` name, never the spool entry or the lock it is linked to. For the wake spool you can instead wait: once the wake is delivered the refusal clears on its own. The refusal is fail-safe (decision 29).
- Writes through a symlink. The scripts refuse, on their own, to write a file or make a directory where that path or a directory between the home and it is a symlink, dangling or not.
- A reeve home with `.` or `..` in it, since `..` is only undone after the symlinks before it are followed. The stand-in refuses `..` on its own too.
- A reeve home with anything but letters, digits, `.`, `_`, `-` and `/` in it, as given or as resolved, since it goes into the command line each pane runs.
- The hall session `default` in any letter case, and any hall session not named `hall-dev*`.

## Rules every command keeps

- **No herdr call at all** (decision 33). Every call runs the hall binary, as `env -u HALL_SOCKET_PATH -u HALL_CLIENT_SOCKET_PATH -u HALL_SESSION -u HALL_CONFIG_PATH -u HALL_ENV -u HALL_PANE_ID <hall> ... --session hall-dev`. Inside a hall pane those names point at that pane's own server and pane, and hall resolves `--session` before them, so the flag wins and the inherited names are dropped as well. Four calls name the session differently, as hall's CLI requires: `hall session list --json` (a read of hall's own sessions folder, `~/.config/hall-dev/sessions`, used to check whether the session runs and to find its socket), `hall session stop|delete hall-dev --json`, whose session is the argument, and `hall --session hall-dev server`, which starts it. Hall never sees herdr's sessions: its folders are its own.
- **Never `hall server stop`**, which stops whichever server it resolves. `down` uses `hall session stop hall-dev`. Hall refuses to stop any server that is not hall's (decision 45).
- **Every test reeve and test hand names its reeve home and throwaway config and state folders** (decisions 26, 47 and 48). Each pane runs `cd "<home>/work/<name>" && env -u REEVE_HAND -u CLAUDECODE -u CLAUDE_PID -u CLAUDE_EFFORT $(<-u each CLAUDE_CODE_* and HERDR_* name the pane's shell holds, but hall's six>) REEVE_HOME="<home>" XDG_CONFIG_HOME="<home>/xdg-config" XDG_STATE_HOME="<home>/xdg-state" HALL_SOCKET_PATH="<hall-dev's socket>" claude --model claude-haiku-4-5-20251001`, and a test hand has `REEVE_HAND="<its errand id>"` before `REEVE_HOME`. Homebrew herdr finds its config, sessions and sockets only through `XDG_CONFIG_HOME` or `HOME`, so no herdr call from a test pane (reeve's herdr backend, the plugin hooks, a model's Bash call) can resolve to the live `~/.config/herdr`: with nothing under `<home>/xdg-config/herdr`, `herdr session list` connects to nothing and `--session default` finds no server. It writes its state (the client's endpoint files, preferences, plugin state, the agent-detection cache) under `XDG_STATE_HOME`, else `~/.local/state/herdr`, so that moves to `<home>/xdg-state` too. `HALL_SOCKET_PATH` keeps a `hall` call from the pane on `hall-dev`, whose socket the moved `XDG_CONFIG_HOME` would otherwise miss. `HOME` is never changed: that would move `~/.claude`. `GH_CONFIG_DIR` is not set: the test claudes do not use `gh`, which under the moved folder finds no login.
- **Nothing about a test claude's identity comes from whoever ran `up`** (decision 48). `up` may run in a plain terminal, in a live herdr pane, or from a claude (a reeve's hand, say), whose environment carries `REEVE_HAND`, claude's own session markers (`CLAUDECODE`, `CLAUDE_PID`, `CLAUDE_EFFORT`, the `CLAUDE_CODE_*` names, among them its messaging socket and token) and the live herdr's `HERDR_*` names. `up` starts the `hall-dev` server under `env -u` of every one of those names it carries, so no pane shell in `hall-dev` holds them. Each claude line drops them again, worked out in the pane's own shell when the line runs (decision 48's second amendment): `$(command env | command sed ...)` lists every `CLAUDE_CODE_*` and `HERDR_*` name that shell holds, beside the fixed `REEVE_HAND`, `CLAUDECODE`, `CLAUDE_PID` and `CLAUDE_EFFORT`. So a `hall-dev` someone else started, whose panes hold names the runner of `up` lacks (a claude's messaging socket, `CLAUDE_CODE_CHILD_SESSION`), passes none of them to a test claude either. A claude line keeps the six `HERDR_*` names hall sets into every pane with the session's own values (`HERDR_ENV`, `HERDR_SOCKET_PATH`, `HERDR_BIN_PATH` and the pane, tab and workspace ids), which claude's herdr agent-state hook uses to report to `hall-dev`. So a test reeve is a reeve, whoever ran `up`: reeve's hooks act on it and write under `REEVE_HOME`. A test hand is a hand because its claude line says so, with its errand id as `REEVE_HAND`.
- **A dialog is never answered with allow.** The only key the scripts ever send is Esc, which denies.

What the guard costs (decision 47): reeve's own herdr backend does nothing for a test reeve. `reeve-name` labels no workspace or tab, a test reeve cannot dispatch a hand, and reeve's status reads through the backend find nothing. The scripts place and label the test hands instead. Reeve's hooks still run in each test claude (decision 48), writing under `REEVE_HOME`; a herdr call one of them makes finds only the throwaway folders.

## Up

```sh
scripts/test-setup up            # or --dry-run first
```

1. Builds the debug hall and the stand-in (`cargo build -p hall -p hall-standin`, with keg-only rustup's and zig's bins put first on PATH and the `HALL_*` names dropped), each unless `HALL_DEBUG_BIN` or `HALL_STANDIN_BIN` names one. The stand-in's first build is by hand (Inputs, above), since the home check needs it.
2. Reads `hall session list --json`. If `hall-dev` is not running, starts it: `hall --session hall-dev server`, in a session of its own (`setsid`, through perl) with its input and output on `/dev/null`, as hall's own launch starts its server, so it outlives the script and its terminal. Then it waits up to 15 seconds (`HALL_START_WAIT`, in tenths of a second, overrides it) for the session list to show it running. If it does not, `up` stops and names the session folder, `~/.config/hall-dev/sessions/hall-dev`, where the server's log is: its own output goes to `/dev/null`. A server that comes up after the wait is left running, and the next `up` uses it.
3. Takes the session's socket path from the session list, for the test claudes' `HALL_SOCKET_PATH`. A path with anything but letters, digits, `.`, `_`, `-` and `/` in it stops `up`, since it goes into the command line each pane runs.
4. Creates `<home>/standin` and the throwaway config and state folders `<home>/xdg-config` and `<home>/xdg-state`, and seeds `<home>/standin/world.toml` from the roster if it does not exist yet; an existing world file is kept as it is. Then it writes `<home>/standin/hall.toml`, the hall client config that turns the reeve tree on (decision 49): `[reeve] listing_command` runs this checkout's stand-in on the home, and `onboarding = false` keeps herdr's first-run overlay off the test view. It is rewritten on every `up`; a stand-in or home path with anything but letters, digits, `.`, `_`, `-` and `/` in it stops `up`, since it goes into a TOML string as it is.
5. Per test reeve, then per test hand: creates `<home>/work/<name>`, prints claude's folder trust for it (below), and, unless the world file already records a pane that hall still has in `hall-dev`, opens it: a test reeve in a new workspace (`hall workspace create --cwd <work dir> --label <name> --no-focus --session hall-dev`), a test hand as a new tab in its reeve's workspace (`hall tab create --workspace <its reeve's workspace> --cwd <work dir> --label <id> --no-focus --session hall-dev`). It runs claude in the new root pane (`hall pane run`) and records the pane with `hall-standin place <name> hall-dev <pane>`. A pane id hall hands out that is not the stand-in's pane id form (letters, digits, `:`, `_`, `-`, not starting with `-`) stops `up` before `pane run`, so it never reaches hall's command line as an option. So the listing's session locations are the panes hall handed out, and a second `up` starts nothing twice. A test hand whose reeve is not up is refused.
6. Prints the listing.

To watch the session, attach to it from a terminal outside hall, with the `HALL_*` names unset and `HALL_CONFIG_PATH` naming the client config `up` wrote: `env HALL_CONFIG_PATH=~/.reeve-hall-dev/standin/hall.toml target/debug/hall --session hall-dev` (`up` prints the exact line). Detach with ctrl+b q; the server keeps running. Attached without that config, hall draws herdr's own sidebar.

## The reeve tree

Attached with the client config, the sidebar's top section is the reeve tree (plan step 3, decisions 49 to 54): each test reeve the listing places in `hall-dev`, its test hands beneath it with a mark for their state (`●` working, `?` needs-decision, `!` blocked, `✓` done, `✗` failed) and words for the rest (`asks`, `waiting`, `idle`, `wedged`), and a badge on the reeve counting its hands that wait on the liege. Workspaces that hold no test reeve sit under `other`. The client reads the listing once a second, so a change made with `hall-standin set` (Setting states, below) shows within a second; an errand set `in_flight=false` drops from the tree. ctrl+b w walks the tree: ↑ and ↓ move, enter shows the reeve's or the hand's own pane, esc leaves. The bottom row of the screen is the key help line.

## Folder trust (fact found)

Claude records folder trust in `~/.claude.json`, as `projects["<dir>"].hasTrustDialogAccepted`. A new `<home>/work/<name>` has no entry, so claude opens its trust dialog there first, and hall reads that as a dialog standing. `up` and `status` read the entry for each work dir and each of its parents, and print `accepted` or `NOT accepted`. Granting it is the liege's decision: answer the dialog in the session yourself (attach with `target/debug/hall --session hall-dev`), or trust the home once, which covers every work dir under it. The scripts never write `~/.claude.json` and never answer the dialog. Note that claude itself adds an entry for each new directory it runs in, and keeps its transcripts under `~/.claude/projects/`; those are claude's own files, shared with the live setup by design, not reeve's. A claude started with an inherited `CLAUDE_CODE_CHILD_SESSION` keeps no transcript; the claude lines drop it (decision 48), so test claudes keep theirs.

## Status

```sh
scripts/test-setup status
```

Read only. It prints:

- `hall-dev` from hall's session list: running, socket path.
- Per roster line: the recorded pane, confirmed with `hall pane get` in `hall-dev`, with its workspace and tab, and hall's reading of it from `hall agent explain --json`.
- Folder trust per work dir, and the listing.
- `scripts/live-check`, the "live setup unchanged" check (decisions 40 to 42): no new entry in the live herdr's four places, no hall process connected to a live herdr socket, no file under `~/.reeve/state` naming the test home or a roster name. It makes no herdr call; see the [plan](plan.md#test-setup).

Anything wrong is a line starting `CHECK:`; a failed live check also makes `status` exit 1. On a home that fails the check inside it (What the scripts refuse, above), `status` still runs, reading nothing inside the home.

## Driving a test reeve

```sh
scripts/test-drive ansel dialog     # to a dialog
scripts/test-drive ansel turn-end   # to the end of a turn
scripts/test-drive ansel deny       # deny a standing dialog with Esc
```

- `dialog` prompts the test reeve to create a file in its work dir (`hall agent prompt`), which claude's default permissions ask about, waits up to two minutes for hall to read it as blocked (`hall agent wait --until blocked`), and prints the reading. The dialog is left standing, so hall can alert on it; `deny` or `turn-end` clears it.
- `turn-end` reads the session first and, if a dialog stands, denies it as `deny` does. Then it prompts a direct question with `hall agent prompt --wait`, which waits for observed activity before it settles, so an agent already idle cannot end the wait before the turn starts. Then it prints the reading.
- `deny` sends Esc, waits up to ten seconds for hall to stop reading the dialog (`hall agent wait --until idle --until done --until working`; hall refuses a prompt to an agent it reads as blocked), and prints the reading.

How the reading is printed, from hall's `agent explain --json` (herdr's, unchanged), the same signal reeve's own backend relies on, because `agent_status` latches:

| state | `live_prompt_box` | printed as |
|---|---|---|
| `blocked` | any | at a dialog (hall: blocked) |
| `working` | any | working |
| `idle` or `done` | matched | ready for input (turn ended) |
| `idle` or `done` | not matched | covered composer: a dialog is likely on screen |

The last row matters: a dialog claude paints over the whole screen can read as `idle` with no `blocked` rule matched (found in reeve's backend). Then `agent wait --until blocked` times out, and the reading still shows the dialog.

## Down

```sh
scripts/test-setup down             # stop the test hall session
scripts/test-setup down --delete    # and delete its saved state
```

Stops `hall-dev` if it is running (`hall session stop hall-dev --json`, which reaches that session's own socket and checks first that it is a hall server), which ends every test reeve and test hand in it, and forgets every recorded pane in the world file (unless the home fails the check inside it, above: then the session is still stopped and the world file is left alone). Without `--delete`, hall keeps the session's saved shape and restores it on the next start, as shells in the old workspaces; `up` then opens fresh ones beside them. With `--delete` the next `up` starts clean. The test reeve home and its world file are kept; remove `~/.reeve-hall-dev` by hand to start fresh.

## Setting states

Any hand state, question or flag is set by editing `<home>/standin/world.toml`, or with `hall-standin set` (see [listing-draft.md](listing-draft.md)):

```sh
target/debug/hall-standin --home ~/.reeve-hall-dev set ansel-fix state=blocked
target/debug/hall-standin --home ~/.reeve-hall-dev set bertil-probe question="Which base?" wedged=true
```

## Tests

`cargo test --workspace` (with `PATH=/opt/homebrew/opt/rustup/bin:/opt/homebrew/opt/zig@0.15/bin:$PATH`) runs:

- the stand-in's unit tests, and `tests/standin_cli.rs`: the listing's shape, every hand state and flag, a question present and absent, errands not in flight left out, `check-home`, and the refusals (a missing `--home`, `~/.reeve` directly, inside it, through a symlink and in another letter case, the user database's `~/.reeve` under a wrong `HOME`, a home not named `.reeve-hall-dev*`, a `..` in the home or in `HOME`, a symlink anywhere inside the home, from `standin` and `work` to a sibling `state` or `errands` and a deep `work/<name>/.claude` (with no write landing in the fake live home), a hard link to a file in the fake live home, a fifo for the world file (refused, not hung on), `check-root` passing a home the walk refuses, a `standin` or `work` it cannot list, a home past the walk's depth limit, a link inside `xdg-config` or `xdg-state` passing the walk while either folder itself a link, or a link under a folder of that name deeper down, is refused, a symlink or hard link at or inside `xdg-config/herdr` or `xdg-state/herdr`, in any letter case, refused, the session `default`, a pane id that could read as an option);
- `tests/scripts_dry_run.rs`: every subcommand of both scripts in `--dry-run`. No line is a herdr call; every hall line runs under `env -u` of the six `HALL_*` names and ends `--session hall-dev` (or is `session list --json`, `session stop|delete hall-dev --json`, or the detached `--session hall-dev server`); `up` builds the debug hall unless `HALL_DEBUG_BIN` names one, opens a workspace per test reeve and a tab per test hand in its reeve's workspace, makes `<home>/xdg-config` and `<home>/xdg-state`, and writes the world file and `<home>/standin/hall.toml`; run with a starter's `REEVE_HAND`, claude markers and `HERDR_*` names set, the server starts under `env -u` of every one of them, and every claude line drops `REEVE_HAND`, `CLAUDECODE`, `CLAUDE_PID` and `CLAUDE_EFFORT` and works out the rest in the pane's shell (`$(command env | command sed ...)`, every `CLAUDE_CODE_*` and `HERDR_*` name but hall's six), the same line whatever the runner carries, then has `REEVE_HAND="<id>"` for a test hand only, `REEVE_HOME="<home>" XDG_CONFIG_HOME="<home>/xdg-config" XDG_STATE_HOME="<home>/xdg-state" HALL_SOCKET_PATH=...` and the haiku model; nothing printed mentions `default`; nothing is created; `status` runs only reads and `scripts/live-check`; the only key ever sent is Esc; `--session-2` is gone; the refusals fire before any command is printed, including `default` as the session, `~/.reeve` in another letter case, through a symlink, and from the user database under a wrong `HOME`, a home that resolves to a path with a quote or `$(...)` in it, a `..` in the home, the root check refusing `down` and `status` for a live home a claude dirtied, and a missing stand-in; a symlinked `standin`, `work`, `work/<name>`, `state`, `errands` or `work/<name>/.claude`, a `standin` or `work` it cannot list, and an npm-style link in `work/<name>` refusing `up` and `test-drive` while `down` (still stopping the test session by name) and `status` go on with a warning and nothing run inside the home; and, behind a stand-in that passes any home, `up` refusing to write or make a directory (the world file, `xdg-config`, a work dir) through a symlink, dangling or not;
- `tests/scripts_fake_hall.rs`: both scripts run for real against a fake hall binary (`HALL_DEBUG_BIN`) and a fake HOME, all under `target/`, with every `HALL_*` name hall reads set as inside a hall pane, the identity names of a claude run from a live herdr pane set as well (decision 48), a fake `herdr` first on PATH that records any call, and a fake `claude` that records the environment each claude line, run as a pane's shell runs it, gives it. It checks that no herdr call is made; that no `HALL_*` name reaches hall; that `up` starts the test session when it is not running and leaves a running one alone; that each test reeve gets a workspace of its own and each test hand a tab in its reeve's workspace; that the listing's session locations are the panes hall handed out; that the test session's server is started with none of the starter's identity names; that `up` writes the client config naming the stand-in's listing of the home and prints the attach line with it; that every claude line carries the throwaway `XDG_CONFIG_HOME` and `XDG_STATE_HOME` and the session's socket as `HALL_SOCKET_PATH`, and gives claude none of the starter's identity names, a test hand its errand id as `REEVE_HAND` and a test reeve none; that a session that does not start stops `up` with exit status 1 and the session folder named, starting no claude; that a second `up` starts nothing; that `status` runs `scripts/live-check`, passing, and fails with a `CHECK:` line when a `~/.reeve/state` file names the test setup; that `up` refuses a pane id like `--session=default` before any `pane run`, and an old roster with session numbers before any hall call; that `up` with a symlinked `standin` or `work` makes no hall call and writes nothing into the fake live home; that on a home with an npm-style link `up` refuses while `status` and `down` run, `down` stopping the test session and leaving the recorded panes alone; that a claude line in a pane whose shell holds names the runner of `up` lacks (a hall-dev someone else started) gives claude none of them and keeps hall's six `HERDR_*` names; that a link inside `xdg-config` or `xdg-state` lets `up` and `test-drive` go on, and either folder itself a link stops both, as does a symlink at `xdg-config/herdr` or `xdg-state/herdr` (or `Herdr`) or a hard link inside either; and that `up` behind a stand-in that passes any home still refuses to write the world file, or make `xdg-config` or a working directory, through a dangling link, starting no claude.

The tests never start hall, herdr or claude and never read or write the real `~/.reeve-hall-dev`.
