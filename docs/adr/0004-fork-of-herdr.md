# Hall is a fork of herdr

Hall is herdr v0.9.0, forked and renamed: its own program `hall`, its own sessions and its own config and state folders, sharing nothing with the liege's installed herdr (decision 31). The liege types `hall` and it starts, detaches and reattaches exactly as herdr does (decision 30), with reeves at the top level of the sidebar and their hands beneath. Chosen because the liege wants "our own session and multiplexer like maybe fork herdr so any breaking changes from it doesn't affect hall", and a fork is the only option where an update to the live herdr can never reach hall. Supersedes [0001](0001-own-app-on-herdr.md).

## Considered Options

- Own app on herdr ([0001](0001-own-app-on-herdr.md)): still depends on an installed herdr and its API, and re-renders herdr's ANSI frames into a picture of its own.
- Own app on a pinned copy of herdr (decision 30): protects hall from a Homebrew upgrade, but reeves dispatch through the Homebrew `herdr` on PATH, which on this Mac wins over any copy hall puts in a pane ([fork-map](../research/fork-map.md) section 3), and hall still re-renders frames.
- A panel inside herdr (a plugin pane): herdr plugins cannot add a grouping level or draw their own sidebar ([herdr-surface](../research/herdr-surface.md)).
- Our own terminal handling from scratch: months of work herdr already does.

## Consequences

- Hall owns about 241k lines of Rust it did not write. Herdr's undocumented internals are now hall's own code; the "documented API only" rule of 0001 lapses with it.
- Upstream changes come in only when chosen and ported by hand. Herdr moves fast: 229 commits between v0.9.0 and v0.9.3, with most lines changed in `vendor/` and, in `src/`, most in the client shell, where the reeve tree goes ([fork-map](../research/fork-map.md) section 7). Hall's own sidebar work stays in few, new functions to keep ports cheap.
- Separation from the live herdr comes from hall's own config and state folders, binary name, inbound `HALL_*` environment names and protocol number, so herdr and hall refuse each other loudly (decision 33, [0003](0003-separate-from-live-setup.md)).
- The reeve tree shows one hall session, since every reeve lives in hall's own session once it is in use; herdr's client has one local endpoint (`src/client/endpoint.rs:72-75` at v0.9.0), so nothing needs combining (decision 38).
- Herdr's own request and done sounds play in the hall window, so hall reuses them and adds only the triggers herdr cannot see (decision 35).
- Apache-2.0 duties: herdr's `LICENSE` and both vendored MIT licences kept, a `NOTICE` line naming hall as a modified herdr, modified files listed in `MODIFICATIONS.md`, no herdr branding (decision 32). They bind on redistribution; doing them from the first commit costs nothing.
