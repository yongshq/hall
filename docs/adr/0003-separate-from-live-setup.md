# Hall is built against its own test setup, never the live one

While hall is developed, the liege's live reeves and herdr session keep running the household's real work, so hall never touches them: it is built and tested against its own herdr session (`hall-dev`, a separate server with its own socket) and its own reeve home (`~/.reeve-hall-dev`) holding stand-in reeves. A stand-in serves the reeve listing format hall needs, so reeve itself does not change until the liege approves that change at the point hall is ready for the live setup.
