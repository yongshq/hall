# Hall learns errand state only through reeve's own commands

Hall never reads a reeve's status files, latches or records itself. It asks reeve, through a machine-readable mode of reeve's listing, for each errand's reconciled state. Reeve already owns that reconciliation (an appended line is a wake event, not the current state; an open decision outlives a later done), and a second reader in another language would drift from it silently.

## Consequences

- Reeve needs a machine-readable listing it does not have today; that change waits on the liege, because the live reeve setup stays untouched while hall is developed.
