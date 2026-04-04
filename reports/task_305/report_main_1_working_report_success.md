## Summary

Implemented directory isolation between multiple zbobr instances by appending `{instance}` to workspace and repository directory paths.

## Changes

### 1. `zbobr/src/commands.rs` (commit 235abb47)
- After task config setup and before backend construction, append instance name to both:
  - `dispatcher_config.workspaces` → `{workspaces}/{instance}`
  - `repo_config.repos_dir` → `{repos_dir}/{instance}`
- Follows the existing wiring-point pattern (analogous to `tasks_config.instance = dispatcher_config.instance.clone()`)

### 2. `zbobr-dispatcher/tests/mcp_integration/env.rs` (commit 5d56546a)
- Updated `init_fs_fs()`: workspaces and repos_dir now include instance suffix
- Updated `init_github_github()`: same pattern applied
- `workspaces_dir` field in `IntegrationTestEnv` reflects the instance-scoped path

## Verification
- `cargo clippy --all-targets --all-features` — clean
- `cargo test --workspace --exclude zbobr-task-backend-github` — all pass
- 3 failures in `zbobr-task-backend-github` are pre-existing (CryptoProvider issue), confirmed they also fail on `main`