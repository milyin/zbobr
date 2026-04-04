After making the changes, run:
1. `cargo test` — all existing tests should pass. The test paths will now include the instance subdirectory (e.g., `base_path/workspaces/default/task-123/`) but this should be transparent since TaskDir and all downstream code just use the configured paths.
2. `cargo clippy` — ensure no new warnings.

If any tests fail due to hardcoded path expectations, update those expectations to include the instance subdirectory.