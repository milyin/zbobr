# zbobr

`zbobr` is a GitHub-backed AI task orchestrator.

It reads tasks from GitHub issues, runs configurable multi-stage pipelines with AI agents, and publishes work results as pull requests.

## What makes zbobr different

- **Custom pipeline configuration**: define your own roles, stages, transitions, pauses, retries, and cross-pipeline calls in `zbobr.toml`.
- **Multiple AI tools**: run roles with different tools (`copilot`, `claude`, more executors can be added).
- **Strong defaults from `init`**: `zbobr init` creates a working starter config, prompts, and directories.
- **GitHub as source of truth**:
  - task context and control live in issues (title/body/comments/labels/state),
  - implementation result lives in pull requests and branches.

## Core idea

One `zbobr` instance controls one target repository and one task repository.

- **Task repository**: issues are tasks and orchestration state.
- **Target repository**: where code changes are implemented and PRs are opened.

The orchestrator advances tasks stage-by-stage and stores progress in GitHub so humans and agents share the same control plane.

## Quick start

### 1. Install

```bash
cargo install --path zbobr
```

### 2. Create a workspace with defaults

```bash
zbobr init my-workspace
cd my-workspace
```

`init` creates:

- `zbobr.toml` with default dispatcher/task/repo/workflow sections
- `prompts/` with role prompt templates
- `repos/` and `workspaces/` directories for runtime data

### 3. Configure GitHub repositories and tokens

Edit `zbobr.toml`:

- `[tasks].github_repo` and `[tasks].github_token`
- `[repo].repository`, `[repo].branch`, `[repo].github_token`

### 4. Initialize labels/stages in GitHub

```bash
zbobr setup
```

### 5. Run task progression

```bash
zbobr task list --select
zbobr task process --select
# or move all eligible tasks one step:
zbobr task advance
```

## Pipeline and role configuration

`zbobr` workflow is fully declarative in `zbobr.toml`.

You define:

- `workflow.roles.*`: role prompt + allowed MCP tools
- `workflow.pipelines.*.stages.*`: stage behavior
- transitions: `on_success`, `on_failure`, `on_intermediate`, `on_no_report`
- execution controls: `pause`, `call` (jump into another pipeline)

Example (shortened):

```toml
[workflow.roles.worker]
prompt = "prompts/worker.md"
mcp = ["stop_with_error", "report_success", "report_failure", "get_ctx_rec"]

[workflow.pipelines.work]
stages.working = { role = "worker", prompts = ["prompts/task.md"], on_intermediate = "reviewing" }
stages.reviewing = { role = "reviewer", prompts = ["prompts/task.md"], on_failure = "working" }
```

This lets you tailor the process to your team without changing Rust code.

## Multi-tool AI execution

`zbobr` supports multiple executors. Today that includes at least:

- Copilot executor
- Claude executor

Default tool/model are configured in `[dispatcher]`, and executor-specific defaults are configured in `[executor.*]`.

This enables patterns such as:

- planning with one model/tool,
- implementation with another,
- review/testing with a different setup.

## GitHub as context, control, and result channel

### Context

Agents consume task context from the issue thread and configured prompts.

### Control

Humans control progression by updating issue state/signals and comments.

Useful commands:

```bash
zbobr task show 42
zbobr task update 42 --state "pending:main"
zbobr task update 42 --signal "go_working"
```

### Result

Workers implement changes in the target repository branch and produce a PR tied to the task.

Operational evidence stays in GitHub:

- issue discussion and reports,
- commits and branch,
- pull request conversation and checks.

## Frequently used commands

```bash
# Create and inspect tasks
zbobr task create "Add feature X" --description "..."
zbobr task list
zbobr task show 42

# Process tasks
zbobr task process 42
zbobr task process --select
zbobr task advance

# Maintenance
zbobr cleanup --dry-run
zbobr cleanup
```

## Configuration model

Runtime config is loaded from `zbobr.toml` (or override files via config flags).

Top-level sections:

- `dispatcher`: global runtime behavior (instance/tool/model/workspace settings)
- `tasks`: GitHub task backend configuration
- `repo`: GitHub repository backend configuration
- `executor`: per-tool executor defaults
- `workflow`: roles and pipelines

## Prerequisites

- Rust/Cargo (to build and install `zbobr`)
- GitHub access tokens for task and repo backends
- Installed AI CLIs/tools used by your configured executors (for example Copilot CLI and/or Claude Code)

## Additional docs

- Token scopes and permissions: `docs/github-token-permissions.md`
- Pipeline transitions reference: `docs/transitions.md`
