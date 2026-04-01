All fixes from review ctx_rec_65 are complete:

1. **Preparator removed from public interfaces**: Deleted `preparation` field from `ZbobrExecutorMcpTesterConfig`, removed the `--executor-mcp-tester-preparation` CLI flag, removed `run_preparation()` integration test helper, deleted the 367-line commented-out test block in config.rs, and updated stale comments in cli.rs and tool_executor.rs.

2. **Docs updated**: Removed `PREPARING`/`GO_PREPARE` from `transitions.dot` and `transitions.md`, fixed the obsolete CLI flag reference in `github-token-permissions.md`, and removed `destination_repository/destination_branch` from the task-backend-fs README.

46 tests pass, 1 pre-existing unrelated failure remains.