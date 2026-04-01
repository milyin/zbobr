The `update_task_branches` function in env.rs accepts `_dest_repo` and `_dest_branch` parameters that are ignored (prefixed with `_`). All callers in abstract_test_helpers.rs compute `dest_repo = env.dest_repo(&repo_path)` and pass it, but the value is thrown away. This is dead code contradicting the single-repo design.

Fix:
1. Remove `_dest_repo: &str` and `_dest_branch: &str` params from `update_task_branches` in env.rs
2. Remove the `dest_repo()` method from IntegrationTestEnv in env.rs
3. Update all callers in abstract_test_helpers.rs to remove the dest_repo computation and pass only task_id and work_branch