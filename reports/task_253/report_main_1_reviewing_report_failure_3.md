Overall assessment: the main architectural direction is correct and mostly consistent with the FS backend analog — `TaskIdentity` is simplified, preparator handling is removed from the default workflow, and repo/branch ownership moved into the repo backend. However, the implementation is not fully complete yet. I found three issues that should be fixed.

1. GitHub repo backend still uses the raw `repository` string for API calls, so URL/SSH forms break PR creation and connectivity checks.

Why this matters:
- The new code explicitly added normalization helpers that accept repository refs beyond plain `owner/repo`: `repo_short_name()` strips `.git` / trailing `/` in `zbobr-repo-backend-github/src/config.rs:68-77`, and `parse_github_repo()` accepts HTTPS and `git@github.com:` forms in `zbobr-repo-backend-github/src/github.rs:110-137`.
- But that normalization is only used for git-side operations (`update_worktree()` / `fetch_refs()` at `zbobr-repo-backend-github/src/github.rs:649` and `:752`).
- `ensure_pr_url()` still builds REST endpoints from the raw config string (`let pr_repo = &self.backend_config.repository;` at `zbobr-repo-backend-github/src/github.rs:766-788`), and `validate_connectivity()` does the same (`repo_path = &self.backend_config.repository` at `:821-827`).

Impact:
- With `repository = "https://github.com/owner/repo.git"` (which the new helper/tests now imply is acceptable), cloning/worktree prep succeeds, but PR lookup/creation and connectivity validation hit `/repos/https://github.com/...`, which is invalid.

Suggested fix:
- Normalize once at config/backend construction time and store a canonical `owner/repo` string (or a parsed `GitHubRepo` type) as the single source of truth for all GitHub API paths.
- Then derive `repo_short_name()` / bare clone naming / endpoint paths from that canonical representation instead of mixing raw and parsed forms.

2. GitHub integration test environment wires `target_repo` to the task repo instead of the configured repo backend repository.

Why this matters:
- `IntegrationTestEnv` documents `target_repo` as the remote repo slug used by GitHub repo-backend tests (`zbobr-dispatcher/tests/mcp_integration/env.rs:36-38`).
- `init_github_github()` now correctly receives both `task_repo` and `repository` (`zbobr-dispatcher/tests/integration_github_github.rs:57-66`).
- But the constructed env still stores `target_repo: Some(task_repo)` instead of `Some(repository)` (`zbobr-dispatcher/tests/mcp_integration/env.rs:248-255`).

Impact:
- Helpers that compute expected repo URLs / workspace names / repo-backend behavior from `env.target_repo` are now pointed at the wrong repository.
- This weakens coverage exactly in the area this task changed: single configured target repository behavior.

Suggested fix:
- Store `target_repo: Some(repository)` in `init_github_github()`.
- Recheck any helper assumptions that previously relied on fork-based behavior.

3. Documentation/examples are still inconsistent with the new single-repository model and with the actual implementation.

Why this matters:
- The task explicitly required updating docs/examples, but several changed docs still describe the old multi-target/fork model or outdated command behavior.
- `README.md` still says zbobr can manage “any set of repositories” (`README.md:12`), still tells users to create an issue and “reference a target repo” (`README.md:122`), and still describes task-project `zbobr.toml` as containing “fork owner” (`README.md:163-168`). Those are all contrary to the new one-instance-per-repository design.
- `docs/github-token-permissions.md:20-25` still documents `gh repo clone` and `git push --force`, but the repo backend now uses bare `git` operations (`zbobr-repo-backend-github/src/github.rs:268-272`, `:511-516`) and explicitly pushes without force.

Impact:
- Users configuring the new single-repo flow will still be told outdated behavior and permissions, and some docs now describe capabilities the code intentionally removed.

Suggested fix:
- Update README sections that still mention multi-repo task-project routing / fork-owner concepts.
- Update permission docs to reflect current implementation details (no forks, no `gh repo clone`, no force-push in the repo backend).

Analog consistency:
- The FS and GitHub backends now mostly follow the same single-repo shape (`repository`, `branch`, `repo_short_name()`, backend-owned routing), which is the right analog.
- The two code issues above are exactly where that consistency breaks: GitHub keeps a partially-normalized repository representation, and the GitHub integration env still points helper expectations at the wrong repository.

Conclusion:
- Not ready to approve yet. Please fix the three issues above and rerun review.