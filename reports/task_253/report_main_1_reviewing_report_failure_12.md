Overall assessment: the core single-repo refactor is largely consistent with the intended analogs. The repo backends now own repository/branch routing, `TaskIdentity` is simplified to `work_branch`, and the dispatcher flow auto-derives branches in a way that matches the new design. However, two issues still need fixing before this branch is ready.

1. `parse_github_repo()` is still too permissive for common malformed HTTPS inputs.

Why this matters:
The current parser rejects extra path segments and non-GitHub hosts, which is good, but it still accepts GitHub URLs that include query strings or fragments. Those are realistic copy-paste inputs from the browser, and they normalize to invalid API repository names such as `owner/repo?tab=readme` or `owner/repo#readme`. That means dry-run prompt preview and runtime normalization can still produce broken repository identifiers even though the input looks like a GitHub repo URL.

Evidence:
- `zbobr-repo-backend-github/src/github.rs:113-129` trims `/` and `.git`, then for HTTPS only checks `parts.len() == 5 && parts[2] == "github.com"` and returns `format!("{}/{}", parts[3], parts[4])`.
- With that logic, `https://github.com/owner/repo?tab=readme` becomes `owner/repo?tab=readme`, and `https://github.com/owner/repo#readme` becomes `owner/repo#readme`.
- Existing tests in `zbobr-repo-backend-github/src/github.rs:918-1039` cover extra path segments and non-GitHub hosts, but do not cover query/fragment rejection.

What to change:
Parse HTTPS URLs with URL semantics rather than plain string splitting, or at minimum explicitly reject `?` / `#` in the final repo component before accepting it. Add tests for query-string and fragment variants, including `.git?…` cases.

2. Docs/examples are still inconsistent with the actual current configuration and task schema.

Why this matters:
This task explicitly includes updating tests, config examples, and documentation. Several touched docs still describe interfaces or fields that no longer match the code. That will mislead users setting up the simplified single-repo workflow and it weakens confidence that the refactor is complete.

Concrete inconsistencies:
- `README.md:245-330` documents token names and requirements such as `ZBOBR_REPO_GITHUB_TOKEN`, `ZBOBR_TASK_GITHUB_TOKEN`, `ZBOBR_AGENT_GH_TOKEN`, and `ZBOBR_OWNER_GH_TOKEN` as if they were actual enforced interfaces.
- The code does not use those names as config interfaces. The dispatcher config only resolves `agent_github_token` (`zbobr-api/src/config.rs:572-583`), and the repo/task backends use `github_token` fields from config (`zbobr-repo-backend-github/src/config.rs:9-24`, `zbobr-task-backend-github/src/github.rs:186-193`).
- `zbobr_github_test.toml.sample:28-32` still shows `agent_token`, but the actual field is `agent_github_token`.
- `zbobr-task-backend-fs/README.md:40-53` still describes the old YAML schema (`plan`, `stage`, `tool`, `model`, `parameters`, `done`, `checklist`), while the implementation serializes the newer task structure with `state`, `work_branch`, `pr_url`, `context`, `signal`, `stack`, `status`, etc. (`zbobr-task-backend-fs/src/fs.rs:19-54`, `69-110`).
- `zbobr-task-backend-fs/example-task.yaml:1-22` likewise still uses the legacy `stage`-based layout instead of the current task file shape.

What to change:
Bring the README and sample files into alignment with the actual config/task model that ships in this branch. In particular:
- remove or rewrite the undocumented `ZBOBR_*` token-name claims unless the code really supports them;
- rename `agent_token` to `agent_github_token` in `zbobr_github_test.toml.sample`;
- update the FS backend README/example to describe the current serialized task format instead of the legacy schema.

Analog consistency assessment:
The implementation generally follows the right analogs: both repo backends now expose a single configured repo/branch, and dispatcher/runtime code uses backend-owned routing instead of per-task routing. The remaining gaps are not architectural mismatches; they are parser completeness and stale public-facing documentation/examples.

Result: report_failure.