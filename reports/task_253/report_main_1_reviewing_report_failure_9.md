Overall assessment: the analog choice is reasonable. The simplification largely follows the existing backend split well: repository/branch now live on the repo backend, and `TaskIdentity` only carries per-task worktree data. The FS and GitHub backends are mostly consistent with that design.

However, I found 2 must-fix issues:

1. `parse_github_repo()` is still too permissive, so invalid/non-GitHub repository refs are silently normalized and then used for real GitHub API calls.

   Evidence:
   - `zbobr-repo-backend-github/src/github.rs:118-153` accepts any `...://.../owner/repo` shape without verifying the host is `github.com`, and the final `parts.len() == 2` check does not reject empty owner/repo segments.
   - `zbobr-repo-backend-github/src/github.rs:167-172` stores that normalized value back into `backend_config.repository`.
   - `zbobr-repo-backend-github/src/github.rs:786-809` then uses `self.backend_config.repository` directly in `/repos/{pr_repo}/pulls` API calls.

   Consequences:
   - Inputs like `https://gitlab.com/owner/repo`, `git@gitlab.com:owner/repo`, `https://github.com//repo`, or `owner/` are accepted when they should be rejected.
   - That can make the backend operate on the wrong GitHub repository name or produce malformed owner/repo values.

   Expected fix:
   - Reject non-GitHub hosts/prefixes for URL/SSH forms.
   - Reject empty owner or repo segments for plain `owner/repo`, HTTPS, and SSH inputs.
   - Add regression tests for those cases.

2. README examples are still inconsistent with the simplified single-repo configuration model.

   Evidence:
   - `README.md:20` says `tasks.task_repo`, but the actual task backend setting is `github_repo` under `[tasks]`.
   - `README.md:26` still refers to `--repo-github-repository`, which does not match the current root `[repo]` config shape.
   - `README.md:100-107` shows `[dispatcher] task_repo = "owner/repo"`; that should describe `[tasks] github_repo = "owner/repo"` instead.
   - `README.md:119` says users should “reference a target repo” in the issue, which contradicts the new single-configured-repository model.

   Consequences:
   - The public docs still teach the old multi-repo/per-task routing mental model.
   - A user following the README can produce an incorrect config.

   Expected fix:
   - Update the README terminology, sample config, and workflow description so they consistently describe one configured target repository and the current `[tasks]` / `[repo]` layout.

Checklist status assessment: the implementation covers the planned surfaces, but these defects mean the task should not be accepted yet.