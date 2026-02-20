Project Copilot Instructions
=============================

Add the following project-wide rules for Copilot / agent-related changes:

1) Avoid repeated string literals
- If a string literal is used in multiple places, prefer a `const` or an `enum` with a single canonical representation.
- Rationale: prevents accidental divergence and makes refactors safer.
- Example (Rust):

  pub const ISSUE_LABEL_BUG: &str = "bug";

  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum Tool { Copilot, Claude }

  impl std::fmt::Display for Tool {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
          match self {
              Tool::Copilot => write!(f, "copilot"),
              Tool::Claude => write!(f, "claude"),
          }
      }
  }

2) Signals are processed on the coordinator level
- All control signals (stage transitions, cancel, pause, reassign, etc.) must be handled by the coordinator process. Agents should not implement independent signal-handling logic or rely on receiving those signals directly.
- Rationale: keeps agent implementations simple and ensures a single source of truth for orchestration decisions.


3) Keep setup script in sync with stage and label changes
- When adding, removing, or modifying task stages (milestones) or labels, update [zbobr-dispatcher/src/backend/github.rs](zbobr-dispatcher/src/backend/github.rs) in the `setup_repository` function to match.
- Rationale: ensures the setup command continues to properly initialize repositories with all necessary stages and labels.

4) Never create commits on the agent's own initiative
- Agents must never create, stage, commit, amend, or push Git commits without an explicit developer instruction and approval.
- Rationale: commits represent the authoritative project history and are the developer's prerogative; autonomous commits can bypass review, break workflows, or produce unexpected repository state.
- Implementation note: Agents may prepare diffs, suggested patches, or proposed commit messages, but must not execute `git` operations that create or push commits unless explicitly directed by a developer.

