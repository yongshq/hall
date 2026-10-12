**Superseded by [0004](0004-fork-of-herdr.md).**

# Hall is its own app, with herdr as the backend

Hall is a full-screen app of its own whose top level is a reeve. Herdr keeps running every session unseen underneath; hall draws the reeve tree itself and shows the chosen session through herdr's live terminal stream, sending keys back through the same route. Chosen because the liege wants reeves, not herdr workspaces, at the top level, and herdr's documented socket API (events pushed, live frames to any external client, input with one controller at a time) makes that possible without writing terminal handling of our own.

## Considered Options

- A panel inside herdr (a plugin pane): cheaper, but herdr plugins cannot add a grouping level or draw their own sidebar, so the top level would stay a herdr workspace.
- Our own terminal handling from scratch: months of work herdr already does (keeping sessions alive, reattaching, scrollback).

## Consequences

- Hall must turn herdr's ANSI frames into a picture itself, with a terminal-emulator library.
- Hall depends only on herdr's documented API; undocumented methods (client-shell, command.invoke) are off limits.
