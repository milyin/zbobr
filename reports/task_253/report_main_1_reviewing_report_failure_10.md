Overall assessment: the single-repo simplification is largely implemented correctly. The analog choice is reasonable: the new single-repo routing consistently moves repository/branch ownership into the repo backend, `TaskIdentity` is simplified to work-branch-centric state, and the dispatcher/worktree wiring generally follows existing backend patterns well. I did not find unrelated code changes that look extraneous to the task.

Must-fix issue:

1. `parse_github_repo()` still accepts non-GitHub SSH remotes and silently normalizes them as if they were GitHub repositories.

   Evidence:
   - `zbobr-repo-backend-github/src/github.rs:118-140`
   - HTTPS input is validated strictly with `parts[2] != "github.com"`.
   - SSH input only checks the path after `:` has exactly `owner/repo`; it never validates the host part.

   Example bad inputs currently accepted:
   - `git@gitlab.com:owner/repo`
   - `ssh://example.com/owner/repo` is rejected by the HTTPS branch, but the SSH-like `git@notgithub.com:owner/repo` path is not.

   Why this matters:
   - This breaks the new configuration contract for the GitHub repo backend: invalid repository references should be rejected early and consistently.
   - `from_config()` immediately rewrites the configured repository to normalized `owner/repo` form (`github.rs:167-172`), so a non-GitHub SSH remote is silently converted into a GitHub target instead of failing validation. That is a correctness bug, not just leniency.
   - It is also inconsistent with the stricter HTTPS validation logic in the same function, so the two accepted URL forms do not follow the same standard.

   Suggested fix:
   - In the SSH branch of `parse_github_repo()`, validate the prefix/host as well, not just the `owner/repo` suffix. At minimum, require the SSH remote to target GitHub (`git@github.com:` or another explicitly supported GitHub SSH form if intended).
   - Add tests rejecting non-GitHub SSH hosts, e.g. `git@gitlab.com:owner/repo` and `git@notgithub.com:owner/repo`.

Everything else I spot-checked looks aligned with the plan: preparator removal is mostly complete, repo/branch ownership is centered in `[repo]`, and the FS/GitHub backends follow similar single-repo patterns.