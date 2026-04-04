## Test Plan Analysis: Append Instance Name for Work Directories

### Change Summary
- **`commands.rs`**: 4 lines appending `instance` to `workspaces` and `repos_dir` via `PathBuf::join()` at the wiring point
- **`env.rs`**: Test environment updated to mirror the same pattern in both `init_fs_fs()` and `init_github_github()`

### Existing Test Coverage
- **~14 integration tests** in `integration_fs_fs.rs` exercise the full dispatch pipeline through the test environment, which now applies instance-scoped paths. These tests cover task creation, stage transitions, MCP tool invocations, conflict handling, pause/resume cycles, and more — all running under the new directory layout.
- **4 `TaskDir` unit tests** verify path construction independently of the wiring point.
- **GitHub integration tests** (`integration_github_github.rs`) use the same pattern via `init_github_github()`.

### Assessment: No Additional Tests Needed

1. **No new behavioral contract**: The change is an unconditional `PathBuf::join(instance)` — there is no branching, error handling, or edge cases to exercise.
2. **Integration tests already validate the layout**: Since `env.rs` mirrors the production wiring, every integration test implicitly runs with instance-scoped directories and would fail if path construction were broken.
3. **Avoid testing static values**: A unit test asserting the exact path string would snapshot a config literal, which violates the test plan guidelines.
4. **Avoid testing standard library**: `PathBuf::join()` is well-tested Rust stdlib; wrapping it in a test adds no value.

### Conclusion
The existing integration test suite provides sufficient coverage for this change. No new tests are recommended.