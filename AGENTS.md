# hall

Instructions for any agent working in this repository. `CLAUDE.md` links here.

## What hall is

Hall is the liege's audience hall: herdr v0.9.0, forked and renamed, with every live reeve at the top level of the sidebar and its hands beneath it. Start with:

- [README.md](README.md): what hall is and where things stand.
- [docs/plan.md](docs/plan.md): the version one plan and its five steps.
- [docs/decisions.md](docs/decisions.md): every decision so far; the plan cites them by number.
- [docs/adr/0004-fork-of-herdr.md](docs/adr/0004-fork-of-herdr.md): why hall is a fork.
- [GLOSSARY.md](GLOSSARY.md): the terms (reeve, hand, errand, liege, test setup).

Herdr's own instructions for agents are kept, unchanged, at [docs/herdr/AGENTS.md](docs/herdr/AGENTS.md). Read them for how herdr's code is built and tested; their contributor, release, `just` and maintainer rules are herdr's, not hall's.

Hall is a modified herdr under Apache-2.0: keep `LICENSE`, `NOTICE` and the vendored MIT licences (`vendor/libghostty-vt/LICENSE`, `vendor/libghostty-vt/pkg/afl++/LICENSE`, `vendor/portable-pty/LICENSE.md`), and add every herdr file you change to [MODIFICATIONS.md](MODIFICATIONS.md) in the same change.

## Hard rule: the live setup is untouched

The liege's installed herdr and reeve are in daily use (decisions 9, 28, 40, 41). Never write, connect to, start or stop anything in:

- `~/.config/herdr`, `~/.local/state/herdr`, `~/.herdr`, `/var/tmp/herdr-pane-graphics-*`;
- the live herdr server or its session `default`, or the Homebrew `herdr` install;
- `~/.reeve`, or `~/.claude` (hooks, settings).

Never run the Homebrew `herdr` except as a throwaway server or CLI under fresh temporary `XDG_CONFIG_HOME` and `XDG_STATE_HOME`, never pointed at the live `default`'s socket. Reading the process table (`lsof`, `ps`) and listing those folders is fine. Hall's own places are `~/.config/hall*`, `~/.local/state/hall*`, `~/.hall*/worktrees` and `/var/tmp/hall-pane-graphics-*`.

## Environment: unset HERDR_* and HALL_* first

You probably run inside a live herdr pane, where `HERDR_SOCKET_PATH` and the rest point at the live server. Every `cargo build`, `cargo test`, `cargo run` and every `hall` or `herdr` invocation runs with all inherited `HERDR_*` and `HALL_*` unset, for example:

```sh
env $(env | sed -nE 's/^((HERDR|HALL)_[A-Z_]*)=.*/-u \1/p') cargo test --workspace
```

Use `sed -E` as written: macOS sed has no `\|` in a basic expression, so the `\(A\|B\)` form matches nothing and unsets nothing.

Hall reads `HALL_SOCKET_PATH`, `HALL_CLIENT_SOCKET_PATH`, `HALL_SESSION`, `HALL_CONFIG_PATH`, `HALL_ENV` and `HALL_PANE_ID`. Into its panes it sets `HALL_ENV=1`, `HALL_PANE_ID`, `HALL_TAB_ID` and `HALL_WORKSPACE_ID` (decision 44b), `HALL_SOCKET_PATH`, `HALL_BIN_PATH` and `REEVE_BACKEND=hall` (decision 68), and keeps herdr's names (`HERDR_SOCKET_PATH`, `HERDR_PANE_ID`, `HERDR_ENV` and the rest; plan, "Environment names set into panes"). Unsetting every `HALL_*` name, as above, covers the new ones; `scripts/test-setup` also drops `REEVE_BACKEND` from its hall calls.

## Toolchain

- Rust 1.96.1, pinned by `rust-toolchain.toml`, through rustup: `PATH=/opt/homebrew/opt/rustup/bin:$PATH`.
- Zig 0.15.2 from Homebrew's keg-only `zig@0.15`: `PATH=/opt/homebrew/opt/zig@0.15/bin:$PATH`, or `ZIG=/opt/homebrew/opt/zig@0.15/bin/zig`. `build.rs` builds the vendored libghostty-vt with it.

## Test

```sh
PATH=/opt/homebrew/opt/rustup/bin:/opt/homebrew/opt/zig@0.15/bin:$PATH cargo test --workspace
```

with `HERDR_*` and `HALL_*` unset as above. This runs herdr's tests and hall's (`crates/hall-standin`, which also drives the scripts in `scripts/` with a fake HOME). Herdr's CLI integration suite (`tests/cli.rs`, `tests/auto_detect.rs`) is Linux-only and does not run on a Mac: check CLI changes by hand.

Known test gaps on this Mac, where hall runs:

- `tests/cli.rs` and `tests/auto_detect.rs` are Linux-only, so their renamed cases (`tests/cli/*`) have never run.
- Two `tests/live_handoff.rs` tests are ignored on macOS (each says why): `live_handoff_preserves_pane_process_io` and `live_handoff_keeps_unmanaged_agent_name_bound_to_saved_session`. Their paths are untested here.

## Never build or test a commit before the separation

Never build, test or run a commit before `237d5cf` (feat: separate hall from herdr), the separation commit, `git bisect` included: bisect only within `237d5cf..` (decision 44c). The merge `fe0d686` and the test-harness commit `0f7bba1` (split from the same working tree as `237d5cf`, never built on its own) are unseparated herdr: they use herdr's folders (`herdr-dev`, `/var/tmp/herdr-pane-graphics-<uid>`, a live place) and follow the inherited `HERDR_*` names, including `HERDR_SOCKET_PATH`, which points at the live server. They stay in the history; this rule is what keeps them from running.

## At the end of every change

Run `scripts/live-check` (with `HERDR_*` and `HALL_*` unset) and include its output in your report. It must exit 0. It fails on any entry in the live herdr's places that is not in its baseline: never accept an entry yourself (`scripts/live-check accept ...`); report it with what the check printed and let the liege decide. Never delete its baseline (`~/.local/state/hall-live-check/baseline.json`): that is the only reset, and it re-baselines over anything new.

`scripts/test-setup` and `scripts/test-drive` run the debug hall and the one hall session `hall-dev` and make no herdr call ([docs/test-setup.md](docs/test-setup.md)). Run them with `HERDR_*` and `HALL_*` unset too. Every test claude they start carries `XDG_CONFIG_HOME=~/.reeve-hall-dev/xdg-config` and `XDG_STATE_HOME=~/.reeve-hall-dev/xdg-state` (decisions 47 and 48), so no herdr call from a test pane finds the live herdr, and nothing of who ran `up` (`REEVE_HAND`, claude's markers, `HERDR_*`) reaches a test claude (decision 48). Run `scripts/live-check` after `up` and after `down`.
