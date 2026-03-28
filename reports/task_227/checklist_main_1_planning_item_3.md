## What

Update all call sites that invoke `list_tasks` to pass the appropriate `allowed_users` slice:

1. `zbobr-dispatcher/src/cli.rs:962` — this is the main dispatch loop. Pass `&self.config.allowed_users` (or however the config is accessed at that point). This is the primary call site where filtering matters.

2. `zbobr/src/commands.rs:391` and `:420` — these are likely admin/diagnostic CLI commands. Pass `&[]` (empty slice) to list all tasks without filtering, or pass the configured `allowed_users` if the config is available. Choose whichever is more consistent with the expected behavior of those commands (listing all tasks for admin purposes suggests `&[]`).

## Why

The dispatcher dispatch loop is the entry point for automated task processing and should respect `allowed_users`. The CLI commands are for human inspection and likely should show all tasks, so `&[]` is appropriate there unless the design intent differs.

## Note

If the dispatcher backend wrapper in `zbobr-dispatcher/src/backend.rs` delegates to another backend, make sure the `allowed_users` parameter is forwarded through the delegation chain.