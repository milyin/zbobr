In `zbobr-dispatcher/tests/mcp_integration/env.rs`, apply the same instance-scoping pattern in both test environment setup functions.

There are two setup functions that construct configs:
1. The local/mock test setup (~line 78-195) 
2. The GitHub integration test setup (~line 218-299)

In each function, after `dispatcher_config` is created, append instance to paths:
- `workspaces_dir` (and `dispatcher_config.workspaces`) should include the instance suffix
- `repos_dir` in `ZbobrRepoBackendFsConfig` or `ZbobrRepoBackendGithubConfig` should also include instance

The `workspaces_dir` field stored in `IntegrationTestEnv` is used by tests to locate task directories, so it must reflect the instance-scoped path. Update it accordingly.

Check if `IntegrationTestEnv.workspaces_dir` is used elsewhere in tests and verify those usages still work with the instance-scoped path.