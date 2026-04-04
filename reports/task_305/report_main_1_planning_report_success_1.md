## Plan: Append instance name for work directories

### Approach
Append `/{instance}` to `workspaces` and `repos_dir` paths at the single wiring point in `commands.rs::run()`, before backends are constructed. This achieves full filesystem isolation between instances with minimal code change — all downstream code (TaskDir, cleanup, bare clone) automatically operates within the instance-scoped directory.

### Key design decisions
- **Wiring-point modification** chosen over threading `instance` through every struct — minimal change, same result
- **Analog**: follows existing pattern of `tasks_config.instance = dispatcher_config.instance.clone()` at the same location
- **No changes needed** to TaskDir, repo backends, cleanup logic — they all read from the already-adjusted config paths

### Files to modify
1. `zbobr/src/commands.rs` — append instance to both paths after config assembly
2. `zbobr-dispatcher/tests/mcp_integration/env.rs` — apply same pattern in test setups
3. Verify with `cargo test` and `cargo clippy`