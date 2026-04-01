# GitHub Token Permissions Reference

This document lists the exact GitHub API operations performed by each backend and the fine-grained PAT permissions they require.

## Repo Backend Token (`ZBOBR_REPO_GITHUB_TOKEN`)

Used by `zbobr-repo-backend-github` to manage branches and pull requests on the configured repository.

### Classic PAT scopes

- `repo` — full repository access
- `workflow` — required when pushing branches that contain `.github/workflows/` files (most real-world repos)

### Fine-grained PAT permissions

**On the configured target repository**:

| Permission | Level | Operations |
| --- | --- | --- |
| Contents | Read/Write | Clone via `gh repo clone` / `git fetch`; push work branches |
| Workflows | Read/Write | Push branches containing `.github/workflows/` files |
| Pull requests | Read/Write | Create and list PRs (`POST/GET /repos/{repo}/pulls`) |
| Metadata | Read-only | Repository info (`GET /repos/{owner}/{repo}`) |

> **Note on Workflows:** GitHub rejects a `git push` that modifies `.github/workflows/` unless the token has `Workflows: Write`. Since zbobr pushes the entire work branch — which may include workflow files — this permission is effectively required for all target repositories that have GitHub Actions.

---

## Task Backend Token (`ZBOBR_TASK_GITHUB_TOKEN`)

Used by `zbobr-task-backend-github` to manage the task project repository: issues (= tasks), milestones (= stages), labels, and comments.

### Classic PAT scopes

- `repo` — full repository access (covers issues, labels, and milestones on private repos)

### Fine-grained PAT permissions

**On the task repo** (configured via `github_repo` in `[tasks]` or `--tasks-github-repo`):

| Permission | Level | Operations |
| --- | --- | --- |
| Issues | Read/Write | Get, create, update, close issues; add/remove labels on issues; create/delete milestones; list/create/update repo labels; list/post issue comments |
| Metadata | Read-only | Check repo exists (`GET /repos/{owner}/{repo}`) — included by default |
| Administration | Read/Write | **`zbobr setup` only:** create the task repo (`POST /orgs/{owner}/repos` or `POST /user/repos`) |

> **Note on Administration:** `Administration: Write` is only needed during the initial `zbobr setup` run that creates the task repository. If the repo already exists, this permission can be omitted.

> **Note on Issues scope:** GitHub's `Issues: Read/Write` covers not only issues themselves but also repository-level labels, milestones, and issue comments — all operations the task backend performs.
