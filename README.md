# Zbobr

AI-powered task dispatcher that manages GitHub issues through automated stages using pluggable AI tools (GitHub Copilot, Claude Code).

## Overview

This repository contains **zbobr** — a reusable automation system that:

- **Manager Agent**: Processes issues through stages (PENDING → GO_PLANNING → PLANNING → GO_WORKING → WORKING), creates implementation plans, and spawns Worker agents
- **Worker Agent**: Implements individual issues by creating branches and PRs in the configured repository

Each zbobr instance manages a single configured target repository.

### Concepts

1. **Zbobr** (this repo)
   - Universal automation system that processes issues through stages
   - Contains planner/worker agents and the `zbobr` CLI binary

2. **Task Project** (`tasks.github_repo`)
   - A GitHub repository whose issues the dispatcher manages
   - Example: `YoroolGui/copilot-zenoh`
   - Contains: project-specific guidance and `zbobr.toml` config
   - Created via `zbobr init <workspace>` then `zbobr setup`

3. **Target Repository** (`--repo-repository`)
   - The single GitHub repository that worker agents operate on
   - Configured in the `[repo]` section of `zbobr.toml` as `repository = "owner/repo"` and `branch = "main"`

## Installation

Install zbobr using Cargo:

```bash
# Install from local source (if you've cloned the repo)
cargo install --path zbobr

# Or install directly from GitHub
cargo install --git https://github.com/milyin/zbobr.git zbobr
```

Verify installation:
```bash
zbobr --help
```

The `zbobr` binary will be installed to `~/.cargo/bin/` (make sure this is in your `PATH`).

**Add to PATH (if needed):**
```bash
# Add to ~/.bashrc or ~/.zshrc
export PATH="$HOME/.cargo/bin:$PATH"
```

## Quick Start

**Prerequisites:**
- GitHub Copilot CLI (`copilot`)
- GitHub CLI (`gh`) authenticated
- Permission to create repos (for task projects)
- Rust and Cargo (for installation)

**Create a workspace:**
```bash
# Ensure your GitHub token is available
export GH_TOKEN=$(gh auth token)

# Create a new zbobr workspace with config and directory structure
zbobr init my-workspace
cd my-workspace
```

**Set up the task project (after editing zbobr.toml):**
```bash
# Create milestones and labels in the configured task repo
zbobr setup
```

**Launch the dispatcher:**
```bash
# Run the manager loop (polls every 60 seconds)
zbobr loop
```

**Using zbobr.toml:**

Run `zbobr` from a directory containing `zbobr.toml` — it automatically reads config from the current directory:

```bash
# Run zbobr commands (automatically uses zbobr.toml config)
zbobr loop                # Start the dispatcher loop
zbobr task list           # Show all tasks
zbobr task list --state READY    # Filter tasks by state
zbobr task process 42     # Process issue #42 (single step)
zbobr task create "Title" --description "desc" --confirm    # create new task that will pause on each stage change
```

Notes on TOML layout:

- Root `zbobr.toml` contains a `[dispatcher]` table with dispatcher-specific keys, a `[tasks]` table for the task backend, and a `[repo]` table with repository config. Example:

   [dispatcher]
   instance = "mybot"

   [tasks]
   github_repo = "owner/zbobr-test-tasks"

   [repo]
   repository = "owner/target-repo"
   branch = "main"

  Stage-specific settings (role, tool, model, prompts, and transitions) are defined in `[workflow.pipelines.*.stages.*]` tables. Global defaults (tool, model) are set in `[dispatcher]`.

Note: legacy top-level dispatcher-only TOML files are no longer supported; use the root `zbobr.toml` with `[dispatcher]`, `[tasks]`, and `[repo]` tables.

This is the recommended approach for task-project workflows, as it eliminates the need to manually specify `--repo-repository` and `--tasks-github-repo` on the command line.

## How It Works

### Workflow

1. **Create issue** in task project with milestone `GO_PLANNING`
2. **Manager researches** the issue (transitioning to `PLANNING` lock state) and creates an implementation plan → sets `PENDING`
3. **Human reviews** and sets milestone to `GO_WORKING` (optionally add `tool:<name>` and `model:<name>` labels)
4. **Manager spawns Worker** (transitioning to `WORKING` lock state)  
5. **Worker implements** by:
   - Creating a work branch in the configured repository
   - Implementing the fix
   - Creating a PR with link to the issue
6. **Worker completes** by setting `PENDING` + adding `done` label
7. **Human reviews** PR and merges

### Labels & Milestones

These are dispatcher-owned and universal—same across all task projects:

**Labels:**
- `tool:<name>` — Specifies which AI tool to use (e.g., `tool:copilot`, `tool:claude`)
- `model:<name>` — Specifies which AI model to use (e.g., `model:gpt-5-mini`, `model:claude-opus-4.6`)
- `done` — Issue implementation is complete

**Milestones:**
- `PENDING` → Issue is under user's control, bot ignores it
- `GO_PLANNING` → Issue must be taken by planner agent, any matching bot can take it
- `PLANNING` → Issue is in planning, other bots ignore it
- `GO_WORKING` → Issue must be taken by worker agent, any matching bot can take it
- `WORKING` → Issue is in work, other bots ignore it

## Architecture

```
zbobr/ (repo root)
├── zbobr/src/              # CLI binary (main.rs)
├── zbobr-dispatcher/src/          # Library (backend, config, MCP, task model, etc.)
├── .github/agents/         # Agent instruction files (manager.md, worker.md)
├── .github/scripts/        # Legacy shell scripts
├── zbobr.toml.sample       # Sample configuration
├── TASK_PROJECT.md       # Task project guide
├── AGENTS.md               # Agent registry
└── README.md               # This file

YoroolGui/copilot-zenoh/ (Task Project - created by zbobr setup)
├── zbobr.toml              # zbobr configuration (default model, stage settings)
├── prompts/
│   ├── common.md           # Shared context and domain knowledge
│   ├── planner.md          # Additional planner context
│   └── worker.md           # Additional worker context
└── README.md               # Setup instructions
```

## Usage Examples

### Set up a new task project

```bash
# Create labels and milestones in the configured task repo
zbobr setup

# Force-update existing labels
zbobr setup --force
```

### Run the manager loop

```bash
# Poll for issues every 30 seconds, clean up every 10 minutes
zbobr loop --interval 30 --cleanup-interval 600
```

### Manually process a specific issue

```bash
# Process issue #42 according to its current stage (single step)
zbobr task process 42
```

## Configuration

All settings can be provided via CLI flags or `zbobr.toml`:

All configuration is read from CLI flags or the `zbobr.toml` file in the task project.
CLI flags override values in `zbobr.toml`.

### Available Models

Zbobr supports 14+ models. Common options include:

- `gpt-5-mini` (default)
- `claude-sonnet-4.5`, `claude-opus-4.6`
- `gpt-5.4`, `gpt-5.3-codex`
- `gpt-5.2`, `gpt-5.2-codex`
- `gemini-3-pro-preview`

Use labels like `model:claude-opus-4.6` on issues to select a specific model, and `tool:copilot` or `tool:claude` to select the AI tool.

## GitHub Authentication

Zbobr uses the GitHub API via a personal access token. It reads the token from the `GH_TOKEN` environment variable (or `GITHUB_TOKEN` as fallback).

If you already have the `gh` CLI authenticated (i.e., `gh auth status` shows you're logged in), you can export the `gh` session token into `GH_TOKEN` so zbobr picks it up:

```bash
# Export your existing gh session token for zbobr to use
export GH_TOKEN=$(gh auth token)
```

Add this to your shell profile (e.g., `~/.bashrc`, `~/.zshrc`) to make it persistent.

Alternative: create a [GitHub Personal Access Token](https://github.com/settings/tokens) with `repo` scope and set it directly:

```bash
export GH_TOKEN=ghp_xxxxxxxxxxxxxxxxxxxx
```

Token resolution order used by zbobr:
1. `GH_TOKEN` environment variable (preferred — matches `gh` CLI convention)
2. `GITHUB_TOKEN` environment variable (fallback — matches GitHub Actions convention)

Required token permissions: The token needs `repo` scope (full access to repositories) to manage issues/labels/milestones and push branches.

### GitHub Backend Token Requirements

Zbobr uses two separate backend tokens. See [docs/github-token-permissions.md](docs/github-token-permissions.md) for the full per-operation breakdown.

#### Repo Backend Token (`ZBOBR_REPO_GITHUB_TOKEN`)

Manages branches and pull requests on the configured repository.

Classic PAT scopes:

- `repo`
- `workflow` (required when the repository contains `.github/workflows/`)

Fine-grained PAT — on the **target repository**:

- `Contents` (Read/Write)
- `Workflows` (Read/Write)
- `Pull requests` (Read/Write)
- `Metadata` (Read-only)

#### Task Backend Token (`ZBOBR_TASK_GITHUB_TOKEN`)

Manages the task project repository: issues, milestones, labels, and comments.

Classic PAT scopes:

- `repo`

Fine-grained PAT — on the **task repo**:

- `Issues` (Read/Write)
- `Metadata` (Read-only)
- `Administration` (Read/Write) — `zbobr setup` only, to create the task repo

### GitHub Tokens for Agents and Copilot

Zbobr manages **three distinct GitHub tokens** with different access levels and purposes:

#### 1. Repo Token
- **Purpose**: Used by the repo backend to clone, push branches, and create pull requests on the configured target repository
- **Access Level**: Write access to the configured code repository
- **Config File**: `github_token` in `[repo]` section of `zbobr.toml`

#### 1b. Task Token
- **Purpose**: Used by the task backend to manage issues, labels, milestones, and comments on the task project repository
- **Access Level**: Write access to the task project repository
- **Config File**: `github_token` in `[tasks]` section of `zbobr.toml`

#### 2. Agent Token — **REQUIRED for restricted access**
- **Purpose**: Passed to agent processes (Copilot/Claude sessions) as `GH_TOKEN` and `GITHUB_TOKEN`
- **Access Level**: Read-only (should have minimal permissions)
- **Why**: Restricts what agents can do on GitHub—they can read repos but cannot push code or modify settings
- **Must Be Different From**: owner token (security requirement—agents cannot have write access)
- **Resolution**: Set `agent_github_token` in `zbobr.toml` or provide via CLI. There is no zbobr-specific environment variable fallback for this token.
- **Config File**: `agent_github_token` in `zbobr.toml`

#### 3. Copilot Token — **Required when restricting agent token**
- **Purpose**: Copilot CLI's own GitHub token (passed as `COPILOT_GITHUB_TOKEN` to agent sessions)
- **Access Level**: Copilot's own permissions (typically full access)
- **Why**: When you restrict the agent token for gh commands, Copilot itself may need its own full-access token to work properly
- **Resolution Order**:
   1. `COPILOT_GITHUB_TOKEN` environment variable
   2. `GH_TOKEN` environment variable
   3. `GITHUB_TOKEN` environment variable
   4. `copilot_github_token` in the `[executor.copilot]` section of `zbobr.toml`
- **Config File**: `executor.copilot.copilot_github_token` in `zbobr.toml`


**Token Validation**: Zbobr validates at startup:
- `ZBOBR_AGENT_GH_TOKEN` must be set
- `ZBOBR_AGENT_GH_TOKEN` must be **different** from `ZBOBR_OWNER_GH_TOKEN` (prevents accidental write access)

## Important Notes

**Security Model**

- **Three tokens**: owner token (write, used by dispatcher), agent token (read-only, passed to agents as `GH_TOKEN`/`GITHUB_TOKEN`), and copilot token (for Copilot CLI, passed as `COPILOT_GITHUB_TOKEN`).
- **dispatcher (octocrab)** uses the owner token when talking to the GitHub API. See [zbobr-dispatcher/src/lib.rs](zbobr-dispatcher/src/lib.rs) which constructs `Octocrab` with the owner token.
- **`gh` cloning** for private repos runs with the owner token injected into the environment to ensure authenticated clones. See [zbobr-dispatcher/src/backend/github.rs](zbobr-dispatcher/src/backend/github.rs) where `gh repo clone` is invoked with `GH_TOKEN`/`GITHUB_TOKEN` set to the owner token.
- **Agent processes** (Copilot/Claude) are spawned with the agent token in `GH_TOKEN`/`GITHUB_TOKEN` and the Copilot token in `COPILOT_GITHUB_TOKEN` so agents have read-only access while Copilot can use its own permissions. See [zbobr-dispatcher/src/tool_executor.rs](zbobr-dispatcher/src/tool_executor.rs).
- **Runtime checks**: configuration validation enforces `ZBOBR_AGENT_GH_TOKEN` is set and different from the owner token to avoid accidental privilege escalation. See [zbobr-dispatcher/src/config.rs](zbobr-dispatcher/src/config.rs).

Detailed configuration parameters:

- Owner token:
   - Env vars: `GH_TOKEN`, `GITHUB_TOKEN`.
   - TOML field: `github_token` in `[repo]` section of `zbobr.toml`.
   - Used for: `octocrab` API calls and owner-level `gh` operations.

- Agent token (read-only):
   - Env var: `ZBOBR_AGENT_GH_TOKEN` (required).
   - TOML field: `agent_github_token` in `zbobr.toml`.
   - Usage: passed to agent subprocesses as `GH_TOKEN` and `GITHUB_TOKEN` to restrict agent GitHub permissions.

- Copilot token:
   - Env vars: `ZBOBR_COPILOT_GITHUB_TOKEN` (preferred), `COPILOT_GITHUB_TOKEN`, `GH_TOKEN`, `GITHUB_TOKEN`, or `$(gh auth token)`.
   - TOML field: `copilot_github_token` under `[executor.copilot]` in `zbobr.toml`.
   - Usage: passed as `COPILOT_GITHUB_TOKEN` to Copilot CLI subprocesses so Copilot can perform operations requiring its own permissions when the agent token is restricted.

See `zbobr.toml.sample` for examples of these fields and the code references above for enforcement and usage.

- **All issue/PR management** happens in the task project repository
- **Agents work directly** in the configured target repository — no forks
- **Labels and milestones** are dispatcher-managed and universal
- **Each zbobr instance** manages exactly one target repository

## Documentation

- [Task Project Guide](TASK_PROJECT.md) — How task projects work (stages, labels, prompts, configuration)
- [Sample Configuration](zbobr.toml.sample) — Example `zbobr.toml` with all options
- [Manager Agent Instructions](.github/agents/manager.md) — Manager workflow
- [Worker Agent Instructions](.github/agents/worker.md) — Worker workflow
- [Agent Registry](AGENTS.md) — Agent definitions for `/agent` command

## Development

Build and test:

```bash
cargo build
cargo test
```
