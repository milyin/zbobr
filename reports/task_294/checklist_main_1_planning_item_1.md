In `zbobr/src/init.rs`, add a new `"linter_worker"` stage to the `main_stages` IndexMap, positioned between `linting` and `testing`.

Stage configuration:
- `role: "linter_worker"`
- `on_success` → `"linting"` (loop back to verify the fix)
- `on_failure` → `"working"` (escalate to general worker if linter_worker can't fix it)

**Why**: This creates the tight loop: `linting → linter_worker → linting`. On success, we re-run the linter to verify all issues are fixed. On failure (can't fix), escalate to the general working stage.

**Pattern to follow**: Look at `test_worker` stage definition as the closest analog — it also loops back (via `on_intermediate → "working"`) and escalates on failure. Note that `linter_worker` uses `on_success → "linting"` (not `on_intermediate`) since the fix-and-verify cycle is represented as success/failure, not intermediate progress.

**Important**: Use `IndexMap` insertion order to ensure `linter_worker` is between `linting` and `testing` in the stage list.