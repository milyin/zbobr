# Verify the build and tests pass

After all code changes are complete:

## 1. Build the entire workspace

```bash
cd /data/home/skynet/zdam/zbobr-dev/workspaces/task-201/zbobr
cargo build
```

Fix any compiler errors (unused imports, missing items, etc).

## 2. Run tests

```bash
cargo test
```

Fix any test failures.

## Common issues to watch for

- Unused `PIPELINE_PREFIX`, `STAGE_PREFIX`, `SIGNAL_PREFIX` constants — remove them if they cause dead code warnings treated as errors
- Any remaining references to removed functions (`apply_signal_change`, `signal_to_label`, `label_to_signal`) — remove those references
- The `State` enum imports in `issue_to_task` — `Stage` and `Pipeline` should still be imported for use in `labels_to_state()`