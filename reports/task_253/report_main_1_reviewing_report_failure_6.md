Overall assessment: the core code refactor is much closer to the intended single-repo design, and the backend/TaskIdentity simplification is broadly consistent with the existing WorktreeBackend pattern. However, the task is still not complete against the stated specification.

Issue 1 — preparator stage is not fully removed from public interfaces
Severity: must-fix

The task description says: "preparator stage is fully removed." That is not yet true in the user-facing/config-facing surface:

- `zbobr/src/commands.rs:109` still documents signals as `go_preparation, go_planning, etc.`
- `zbobr/src/commands.rs:125-127` still exposes `--executor-mcp-tester-preparation`
- `zbobr/src/commands.rs:471-485` still wires the `preparation` scenario override into `ZbobrExecutorMcpTesterConfig`
- `zbobr-executor-mcp-tester/src/config.rs:12-20` still defines a legacy `preparation` field
- `zbobr-executor-mcp-tester/src/config.rs:77-79` still maps stage names `preparation` / `preparator`
- `zbobr-executor-mcp-tester/src/lib.rs:150-153` still tests execution against stage name `preparation`

Why this matters:
The new workflow no longer has a preparator stage, so keeping it in CLI help, config types, and stage-name resolution leaves a stale public API and contradicts the simplification goal. This is not just an internal compatibility shim; it is still discoverable and advertised behavior.

Suggested fix:
Remove the preparation/preparator-specific mcp-tester override field and aliases, or explicitly replace them with current stage names only. Also scrub the remaining help text and related tests/comments so the public interface no longer suggests that a preparator stage exists.

Issue 2 — documentation/examples are still materially inconsistent with the new single-repo model
Severity: must-fix

A large set of docs and examples still describe the pre-simplification model or invalid current config/schema:

- `README.md:12` says zbobr can "manage any set of repositories"
- `README.md:20-28` still presents a target-repo concept around obsolete CLI flags and mixes old/new naming
- `README.md:119` says issues should "reference a target repo", which should no longer be task-level routing
- `README.md:198` says config lives in the "task project" and later sections still reference old config/layout names
- `README.md:282-283`, `README.md:323`, `README.md:336`, `README.md:345-346` reference obsolete `[backend_github]`, `zbobr.toml.sample`, and `TASK_PROJECT.md`
- `README.md:315` claims `gh repo clone` is used from `zbobr-dispatcher/src/backend/github.rs`, but the implementation now lives in `zbobr-repo-backend-github` and uses different mechanics
- `docs/github-token-permissions.md:20` still documents `git push --force`
- `docs/github-token-permissions.md:39` still references `--tasks-github-task-repo`
- `zbobr-task-backend-fs/README.md:49` still documents `destination_repository` / `destination_branch` task parameters
- `zbobr-task-backend-fs/example-task.yaml:7-10` still serializes those removed task fields
- `zbobr_github_test.toml.sample:8-12` still refers to `[tasks.github]` / `[repo.github]`
- `zbobr_github_test.toml.sample:31-32` still uses `agent_token` instead of the current field name

Why this matters:
Updating docs/examples was an explicit checklist item for this task, and these are not minor wording nits. Several examples are now factually wrong or invalid for the current code, which will mislead users and future contributors.

Suggested fix:
Do a final documentation sweep for all public docs/examples/samples/help text and align them to the current schema and workflow:
- single configured repo backend (`[repo].repository` / `[repo].branch`)
- no per-task destination repo/branch fields
- no preparator stage
- no fork workflow
- current config section/field names only
- current repo backend implementation details only

Analog consistency:
The core implementation mostly follows the existing backend abstraction pattern correctly: repository/branch moved into the repo backend, and `TaskIdentity` is simplified accordingly. The main inconsistency is that the public/documentation surface was not simplified to the same extent, so the branch still presents mixed old/new behavior.

Conclusion:
Failing review until the remaining preparator-facing interface and stale docs/examples are cleaned up.