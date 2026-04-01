Overall, the core production changes are moving in the right direction and mostly follow the existing backend/config patterns, but the simplification is still incomplete. I found three issues that should be fixed before this is accepted.

1. GitHub repo normalization is still too permissive and can silently target the wrong host.

   In `zbobr-repo-backend-github/src/github.rs:110-129`, `parse_github_repo()` accepts any string containing `://` and normalizes it by taking the last two path segments. The resulting `owner/repo` is then used by `from_config()` (`:154-159`) and later turned back into `https://github.com/{owner}/{repo}.git` for cloning (`:269`). That means a non-GitHub URL like `https://gitlab.example.com/acme/service.git` would be silently reinterpreted as GitHub repo `acme/service`, which is incorrect and can point the backend at the wrong repository.

   This is a correctness issue introduced by the new normalization logic. The parser should either:
   - accept only `owner/repo`, `https://github.com/owner/repo(.git)`, and `git@github.com:owner/repo(.git)`, or
   - explicitly validate that URL/SSH inputs are actually for GitHub before normalizing.

   Please add a negative test for non-GitHub URLs/SSH refs so this cannot regress.

2. A removed public interface is still exposed in CLI help.

   The task says the preparator stage is fully removed, including public interfaces. But `zbobr/src/commands.rs:109` still documents the `--signal` argument as `New signal (go_preparation, go_planning, etc.)`. That help text is user-visible, so it still advertises a removed stage.

   Please remove the `go_preparation` example (or reword the help to avoid hardcoding obsolete signal names). This is exactly the kind of literal drift the project rules warn about.

3. Documentation and test surface are still inconsistent with the single-repo model.

   README still describes the old multi-repo/task-selected routing model in several places. Examples:
   - `README.md:12` says the dispatcher can manage “any set of repositories”.
   - `README.md:20-29` describes a task project plus a separate per-task “Target Repository”.
   - `README.md:103` uses `task_repo = "owner/repo"`, but the actual task-backend config field is `github_repo`.
   - `README.md:119` says to create an issue and “reference a target repo”, which contradicts the new single configured repo model.

   Separately, the GitHub integration helpers still preserve the removed multi-repo/fork model instead of being cleaned up:
   - `zbobr-dispatcher/tests/mcp_integration/env.rs:324-346` keeps `update_task_branches(task_id, _dest_repo, _dest_branch, work_branch)`, but the destination repo/branch parameters are ignored.
   - `zbobr-dispatcher/tests/mcp_integration/test_helpers.rs:785-1092` still contains cross-org/fork-oriented helpers (`CROSS_ORG_DEST_REPO`, fork-remote assertions, `fork_owner()` gating, etc.).

   Even if some of these helpers are currently unused, they now encode removed behavior and are misleading: callers still appear to set per-task target repo/branch, but the helper discards them. Since the task explicitly included updating tests/docs, these leftovers should be removed or rewritten to reflect the single configured repository model.

Because of the above, I recommend another fixing pass before acceptance.