# Listing draft

**DRAFT.** This is the shape the stand-in serves today, built from the fields in the plan's "Listing format hall needs from reeve". It is not a settled format: the real one is owned by a reeve change that goes to the liege for approval when hall is ready for the live setup (ADR [0002](adr/0002-state-through-reeve.md), decision 11). Until then the stand-in (`crates/hall-standin`) serves it from a test reeve home, and hall reads only this.

## Asking for it

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

## The listing

JSON on stdout:

```json
{
  "draft": true,
  "reeves": [
    {
      "name": "ansel",
      "session": { "herdr_session": "hall-dev", "pane_id": "w1:p1" },
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
      "waiting": false,
      "idle": false,
      "wedged": false,
      "session": { "herdr_session": "hall-dev", "pane_id": "w3:p1" }
    }
  ]
}
```

| Field | Meaning |
|---|---|
| `draft` | Always `true` while this is a draft. |
| `reeves[].name` | The reeve's name, as shown in the reeve tree. |
| `reeves[].session` | Its session location (below). |
| `reeves[].live` | Whether the reeve is alive now. |
| `errands[]` | Errands in flight only (decision 16). One not in flight is left out. |
| `errands[].id` | The errand id. |
| `errands[].office` | The office the hand holds. |
| `errands[].owner_reeve` | The reeve that dispatched it; always one of `reeves[].name`. |
| `errands[].hand_state` | One of `working`, `needs-decision`, `blocked`, `done`, `failed`. |
| `errands[].question` | The hand's unanswered question, or `null`. |
| `errands[].waiting`, `idle`, `wedged` | One flag each, as the reeve judges them. |
| `errands[].session` | The hand's session location (below). |

A session location is `{ "herdr_session": <name>, "pane_id": <id or null> }`: the hall session, and the pane inside it that holds the session. The field keeps its draft name; since step 2 it holds a hall session name, `hall-dev` in the test setup (plan, step 2). `pane_id` is `null` until the test setup has recorded one. `herdr_session` is never `default`.

## The world file

The stand-in reads `<home>/standin/world.toml`. Editing it changes the listing on the next read; nothing is cached. `scripts/test-setup up` seeds it from the roster in `scripts/test-setup.conf` and records each pane id.

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

## Changing it without an editor

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
