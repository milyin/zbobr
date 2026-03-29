# Implementation Plan: `Secret` type for storing sensitive values

## Approach

Introduce a `Secret` enum in `zbobr-api` (the shared API crate), then migrate all 4 token fields across the codebase from `String` to `Secret`.

## Key Design Decisions

**Analog**: `StageTransition` in `zbobr-api/src/config.rs` — same pattern of a type with custom serde that deserializes from a specific table format.

**Placement**: `zbobr-api` — already has serde, is depended on by all crates that need `Secret` (repo/task backends, executor copilot, dispatcher config).

**TOML format**: `{ value = "..." }` or `{ env = "VAR_NAME" }` — implemented via an untagged serde helper with distinct keys per variant. No backward-compatible plain-string form.

**CLI args**: Token fields must be marked `#[config(skip_args)]` — secrets don't belong on the command line.

**Copilot ad-hoc env logic removed**: The current special `std::env::var("COPILOT_GITHUB_TOKEN")` fallback in `ZbobrExecutorCopilotConfig::build()` is removed; users who want env-var sourcing must now write `copilot_github_token = { env = "COPILOT_GITHUB_TOKEN" }` explicitly in their TOML.

## Checklist

1. Define `Secret` enum in `zbobr-api` with serde + `resolve()` method
2. Migrate 4 token fields from `String` to `Secret`, removing clap env attrs, updating defaults and build methods
3. Update validation methods and all callsites to call `.resolve()?`
4. Add unit tests for `Secret` and update existing token-related tests