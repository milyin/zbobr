Simplify the GitHub repo backend to work directly on the target repository without forking:

1. Remove `fork_owner` from `ZbobrRepoBackendGithubConfig` and its validation logic.

2. Remove fork-related methods from `github.rs`: `ensure_fork()` and `ensure_fork_remote()`.

3. Remove the "fork" remote from bare clone setup — there is now only one remote (`origin`) pointing to the target repository.

4. Update `ensure_worktree_github()` and any PR creation logic to push directly to the target repo's work branch instead of to a fork.

5. Remove the `merge-upstream` GitHub API call that was used to sync the fork.

The analog for the simplified backend is the filesystem backend (`zbobr-repo-backend-fs/src/fs.rs`), which already has no fork concept and operates directly on a single bare clone.

Why: In single-repo operation, there is no need for forking. The agent has direct write access to the target repository (via the configured token), so work branches are pushed directly to origin.