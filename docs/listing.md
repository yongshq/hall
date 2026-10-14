# Listing

The listing is how hall learns from reeve which reeves and hands there are, where each sits and what each waits on (ADR [0002](adr/0002-state-through-reeve.md)). Hall never reads reeve's files: it runs the command its client config names, `[reeve] listing_command`, once a second (`listing_interval_ms`, decisions 49 and 67b), and reads the JSON it prints. This page is the format, version 1, settled in step 5 (decision 68) from the draft step 2 served (decision 11), and the stand-in that serves it for hall's tests.

Two commands print it:

- **Reeve's own**, `bin/reeve-listing [--home <path>]` in reeve's repository (reeve's side of step 5). It reads the reeve home (`--home`, else `REEVE_HOME`, else `~/.reeve`) and writes nothing, delivers no wake and calls no backend; the flags come from the sentry's own judgements on disk. At go-live the liege's hall reads it: `~/.config/hall/config.toml` gets `[reeve] listing_command = ["/Users/yong/yongshq/reeve/bin/reeve-listing"]` (the cut-over, the reeve's). The test setup's `--listing reeve` mode runs it on the test home ([test-setup.md](test-setup.md)).
- **The stand-in**, `crates/hall-standin`, from a world file in a test reeve home, so hall's tests and the test setup can set any state on demand, which a real reeve cannot (The stand-in, below).

## The format, version 1

JSON on stdout, exit status 0:

```json
{
  "version": 1,
  "reeves": [
    {
      "name": "ansel",
      "session": { "backend": "hall", "name": "hall-dev", "pane_id": "w1:p1" },
      "live": true
    }
  ],
  "errands": [
    {
      "id": "ansel-fix",
      "office": "artificer",
      "owner_reeve": "ansel",
      "hand_state": "needs-decision",
      "question": "Which base branch?",
      "open": 1,
      "waiting": false,
      "idle": false,
      "wedged": false,
      "since": { "waiting": null, "idle": null, "wedged": null },
      "session": { "backend": "hall", "name": "hall-dev", "pane_id": "w3:p1" }
    }
  ]
}
```

| Field | Meaning |
|---|---|
| `version` | Required, `1`. A listing with no `version`, or another, is refused: the tree keeps its last good read, marked `stale:` with the reason, as for a failed read. The draft's `draft: true` is no version. |
| `reeves[].name` | The reeve's name, as shown in the reeve tree. |
| `reeves[].session` | Its session location (below). |
| `reeves[].live` | Whether the reeve is alive now; default `true`. |
| `errands[]` | Errands in flight only (decision 16): one torn down, or never dispatched, is left out; a hand reported done shows `✓` until its teardown. |
| `errands[].id` | The errand id. |
| `errands[].office` | The office the hand holds, or `null` when its record names none (read as none). |
| `errands[].owner_reeve` | The reeve that dispatched it, or `null` when reeve cannot tell (the record names no valid reeve and its owning session has no name): the hand then nests under no reeve, and raises no alert, since no row could show its highlight. Its pane keeps herdr's own notifications, as a pane the listing does not name. |
| `errands[].hand_state` | `working`, `needs-decision`, `blocked`, `done` or `failed`; any other (reeve's `unknown`, say) shows `·` and counts toward nothing. |
| `errands[].question` | The hand's newest unanswered question, or `null`. |
| `errands[].open` | Reeve's count of the hand's open questions. Hall does not read it. |
| `errands[].waiting`, `idle`, `wedged` | One flag each, as the reeve's watch judges them; default `false`. |
| `errands[].since` | When each flag's present spell began: `{ "waiting": N, "idle": N, "wedged": N }`, each `N` the epoch seconds in the first field of the latch file behind that flag (`state/.attn-<id>`, `.stale-<id>`, `.wedged-<id>`), or `null` while that flag is `false`. Optional: an older reeve prints none. A `since` that does not read (not an object, or a value that is not a whole number of seconds) is taken as absent, for that flag or for all three, and never drops the errand. Hall reads it only to ring each spell once (Ringing a flag, below). |
| `errands[].session` | The hand's session location (below). |

A session location is `{ "backend": <name>, "name": <session or null>, "pane_id": <id or null> }`: the backend that holds it (`hall` or `herdr`), the session there, and the pane in it. `name` is `null` for every herdr entry, since reeve's herdr backend cannot say, and for a hall record with no `place`; `pane_id` is `null` until the pane is known. Hall drops every reeve and errand whose `backend` is not `hall` before it matches anything (design finding F1): herdr's live session is `default`, as hall's is at go-live, and a herdr reeve at `w1:p1` would otherwise land on hall's own `w1:p1`. Of the rest, hall shows those whose `name` is its own session (`default` for the liege's hall, `hall-dev` in the test setup) at the pane named.

Reeve's listing prints `null` for a value it does not have: `office`, `owner_reeve`, `question` and a location's `name` and `pane_id`, as above; every other field always has a value. Hall reads each reeve and errand on its own, after the `backend` check: one that does not read (a `null` where the table allows none, a field of the wrong type, a missing `name`, `id` or `session`) is dropped alone, and the rest of the listing still reads. An entry with no `session.backend` to tell is not taken for another backend's: it does not read either. Each reason an entry is dropped for is logged once in the client log, at warn on hall's target (`event="reeve.listing"`, with the entry's `id` or `name` and the reason, written by the default filter `hall=warn`, which writes no info line; set `HERDR_LOG` to change it), so a drift in the listing that drops every entry the same way, which the tree shows only as no reeve listed, leaves its cause there. Only a listing that is not JSON at all, or has the wrong `version`, fails the whole read.

Hall ignores fields it does not know. A command that fails shows its first non-empty stderr line as the reason; reeve's prints one line and exits 1 only when it cannot read the home.

### Ringing a flag

Reeve's listing shows a flag only while the reeve's watch runs, and the watch exits when it wakes the reeve and is started again: the flag drops for that gap and rises again on the same latch. Hall rings a hand's listing alerts on a rise only ([test-setup.md](test-setup.md), Alerts), so without `since` one spell of `idle`, say, rang once per watch (finding R2 of step5-shown-working). With it, a rise of `waiting`, `idle` or `wedged` rings only when its `since` differs from the one hall last rang for that errand and flag, and hall then keeps the new one:

- during a watch gap the flag drops, as reeve's listing has it by design, and with it the hand's count in its reeve's badge, the hand's note and its highlight;
- the same `since` again after a gap is the same spell: no sound, and the highlight comes back, unless the liege has shown the hand (enter, a click on its row, or the focus moving onto its pane) since that spell rang, which keeps it cleared for that `since`; a `waiting` spell absorbed by a dialog this client rang (below) counts as shown when the liege had shown the hand at that dialog, before reeve listed `waiting` too;
- another `since` is a new spell and rings, as does a spell older than the last one rung;
- a flag up with no `since` (absent, `null`, or one that does not read) rings on every rise, as it did before `since`, and leaves what was last rung as it was;
- the first good read after attaching rings nothing, counts each spell then up as rung and raises the hand's highlight without a sound, so a watch gap after it rings nothing either; showing the hand clears that highlight as above, and a hand whose pane is focused at that read counts as shown and is not highlighted;
- a client that starts during a watch gap does not see the spell, so when the flag comes back that spell rings once more; what was rung is kept per client process, not by the server, so a restarted or reattached client can ring a spell again, and two clients attached at once ring each spell once each (two sounds);
- a `waiting` rise left silent because this client rang herdr's request for the dialog the hand's pane stands at counts its spell as rung, so one dialog still rings once; a dialog counts as rung when its request was delivered, even if that agent's sound is off (`[ui.sound.agents]`), so the `waiting` stays silent too; while this client only holds that request (the toast delay, or the snapshot), the rise waits and counts nothing, and if herdr's rule then drops the request (the pane left the dialog first) the rise rings as any other;
- an errand that leaves the listing has its spells forgotten: back with the same `since`, it rings as a hand new to the listing.

## A hand's pane after a hall restart

A reeve hand's pane is not restored (the liege's choice A, 2026-10-14). Reeve opens each hand's pane with `--no-agent-resume` (`tab create`, `worktree open` or `workspace create`), and hall saves that mark with the pane. When the server starts again and restores its session, a marked pane is not recreated at all, a tab or workspace left with no pane goes with it, and their ids are not handed out again. So after a hall restart the hand's recorded target no longer resolves: reeve's sentry reports the errand `stale:` (session gone), and the reeve sends the hand out again from its brief. Work committed on the hand's branch is kept; its unfinished conversation is not. The reeve's own pane is unmarked and resumes as before (`[session] resume_agents_on_restore`).

Why: hall's native resume restarts claude as a bare `claude --resume <id>`, without the hand's launch line (`REEVE_HAND`, the office's `--settings`, `--add-dir`), so a resumed hand stalls at its next `reeve-say`. `pane get` and `pane list` show `"agent_resume": false` for a marked pane, so reeve can check the mark. A live handoff keeps marked panes, since their processes carry on.

## The stand-in

The stand-in prints version 1 with every location's `backend` `hall`, the session name from the world file's `herdr_session` key, and no `open` and no `since`, so its flags ring on every rise.

### Asking for it

```sh
hall-standin --home ~/.reeve-hall-dev
```

`--home` is required on every call. There is no default and no environment fallback. The stand-in refuses (exit status 1, a message on stderr, nothing on stdout):

- `~/.reeve`, or any path inside it, symlinks followed, for `HOME` and for the home the user database records; matched by the directory on disk, so another letter case (`~/.REEVE`) or another route to it is refused too;
- a path through a symlink whose target does not exist;
- a path with a `..` in it, since `..` is only undone after the symlinks before it are followed, so the spelled-out path and the one on disk could differ;
- a home with a symlink anywhere inside it (the world file, the live baseline, each claude's working directory, what reeve and claude write there), since it could carry a read, a write or a claude session out of the home; the home is walked without following links, and whatever cannot be listed or looked at is refused, as is a home of more than 20000 entries or 32 directories deep;
- a `HOME` or user database home with a `..` in it, since where its `.reeve` sits then depends on who reads the path;
- any home whose directory name does not start with `.reeve-hall-dev`;
- a world file that places a reeve or an errand in the session `default` (any letter case);
- a world file or `place` with a pane id that is not `^[A-Za-z0-9:_-]+$` or that starts with `-`, so it can never reach hall as an option.

A missing `--home` is a usage error (exit status 2).

### The world file

The stand-in reads `<home>/standin/world.toml`. Editing it changes the listing on the next read; nothing is cached. `scripts/test-setup up` seeds it from the roster in `scripts/test-setup.conf` and records each pane id. The key `herdr_session` keeps its draft name and holds the hall session's name, listed as `session.name`.

```toml
[[reeve]]
name = "ansel"              # required, unique
herdr_session = "hall-dev"  # required, never "default"
pane_id = "w1:p1"           # optional; recorded by scripts/test-setup up; A-Z a-z 0-9 : _ -, no leading -
live = true                 # optional, default true

[[errand]]
id = "ansel-fix"            # required, unique, not also a reeve name
office = "artificer"        # required
owner = "ansel"             # required, a reeve above
state = "working"           # optional, default working: working | needs-decision | blocked | done | failed
question = "Which base?"    # optional; absent or "" means no question
waiting = false             # optional, default false
idle = false                # optional, default false
wedged = false              # optional, default false
in_flight = true            # optional, default true; false leaves the errand out of the listing
herdr_session = "hall-dev"  # required, never "default"
pane_id = "w3:p1"           # optional
```

Unknown keys are refused, so a typo fails loudly instead of being ignored.

### Changing it without an editor

```sh
hall-standin --home ~/.reeve-hall-dev set ansel-fix state=blocked
hall-standin --home ~/.reeve-hall-dev set ansel-fix question="Which base?" waiting=true
hall-standin --home ~/.reeve-hall-dev set ansel-fix question=      # clears the question
hall-standin --home ~/.reeve-hall-dev set ansel-fix in_flight=false # finished and cleaned up
hall-standin --home ~/.reeve-hall-dev set ansel live=false
hall-standin --home ~/.reeve-hall-dev place ansel hall-dev w1:p1   # record a session location
hall-standin --home ~/.reeve-hall-dev locate ansel                 # prints: hall-dev w1:p1
hall-standin --home ~/.reeve-hall-dev sessions                     # one line per reeve and errand in flight
hall-standin --home ~/.reeve-hall-dev check-home                   # prints the home as resolved on disk, or refuses it
hall-standin --home ~/.reeve-hall-dev check-root                   # the same, without the walk inside the home
```

`check-home` reads no file inside the home, which need not exist yet; it only lists its directories and looks at each entry, to check that nothing inside is a symlink, a file with more than one name (a hard link), or neither a regular file nor a directory. Every other command makes the same check first. `check-root` makes only the check of the path itself (the live reeve home, `..`, the name). The test setup scripts run `check-root` before anything else and use the path it prints, then `check-home` (see [test-setup.md](test-setup.md) for which subcommands go on past that one with a warning).

`set` and `place` keep the file's comments and layout, and write nothing unless the result is still a valid world file.
