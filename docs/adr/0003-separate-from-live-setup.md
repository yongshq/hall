# Hall is built against its own test setup, never the live one

While hall is developed, the liege's live reeves and herdr session keep running the household's real work, so hall never touches them: it is built and tested against its own herdr sessions (`hall-dev` and `hall-dev-2`, separate servers with their own sockets) and its own reeve home (`~/.reeve-hall-dev`) holding test reeves and test hands. A stand-in serves the reeve listing format hall needs, so reeve itself does not change until the liege approves that change at the point hall is ready for the live setup.

## Consequences

- Every herdr call names its herdr session on the command line or connects to that session's socket directly. Herdr resolves `--session` first, then `HERDR_SOCKET_PATH`, then `HERDR_SESSION`, and every process inside a live pane inherits a `HERDR_SOCKET_PATH` pointing at `default`, so an environment variable alone can land on the live setup.
- Herdr's global config and plugin registry are shared by every herdr session, so the test setup never edits them; a test config goes through `HERDR_CONFIG_PATH`.
