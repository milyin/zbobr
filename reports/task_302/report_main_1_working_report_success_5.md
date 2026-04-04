# Fix: Presence-aware patch types for list fields in WorkflowToml merging

## Problem
The previous implementation used `Vec<T>` with `if !other.mcp.is_empty()` logic in
`RoleDefinition::merge_toml` and `StageDefinition::merge_toml`. This meant:
- Absent `mcp` field in overlay → empty vec → **incorrectly kept base value** ✓
- Explicit `mcp = []` in overlay → empty vec → **incorrectly kept base value** ✗ (broken)

So it was impossible to clear an inherited list by setting it to `[]` in an overlay
config file.

## Solution
Introduced three new "patch" types in `zbobr-api/src/config.rs` that are used
exclusively in `WorkflowToml` for TOML deserialization and merging:

- `RoleDefinitionPatch`: like `RoleDefinition` but with `mcp: Option<Vec<McpTool>>`
- `StageDefinitionPatch`: like `StageDefinition` but with `prompts: Option<Vec<PathBuf>>`  
- `PipelineConfigPatch`: like `PipelineConfig` but with `stages: IndexMap<Stage, StageDefinitionPatch>`

With `Option<Vec<T>>`:
- Absent field → `None` → `other.mcp.or(self.mcp)` keeps base ✓
- `mcp = []` → `Some(vec![])` → `other.mcp.or(self.mcp)` replaces base with empty ✓
- `mcp = ["x"]` → `Some(vec!["x"])` → replaces base ✓

## Changes Made

### `zbobr-api/src/config.rs`
- Removed `MergeToml for RoleDefinition`, `MergeToml for StageDefinition`, `MergeToml for PipelineConfig`
- Added `RoleDefinitionPatch`, `StageDefinitionPatch`, `PipelineConfigPatch` with correct `MergeToml` impls (using `.or()` semantics)
- Added `resolve_paths` methods on the patch types
- Added `From<XxxPatch> for Xxx` (patch → final) and `From<Xxx> for XxxPatch` (final → patch) conversions
- Updated `WorkflowToml.roles` to `Option<IndexMap<String, RoleDefinitionPatch>>`
- Updated `WorkflowToml.pipelines` to `Option<HashMap<Pipeline, PipelineConfigPatch>>`
- Updated `WorkflowToml::try_into_config` and `Config::build` to convert patch → final types
- Updated all tests that create `WorkflowToml` with old types
- Added two new behavior-oriented tests:
  - `role_mcp_cleared_by_empty_list_overlay`: verifies `mcp = []` clears base mcp
  - `stage_prompts_cleared_by_empty_list_overlay`: verifies `prompts = []` clears base prompts

### `zbobr-utility/src/lib.rs`
- Updated `MergeToml` trait docs to reflect correct semantics (replaced old "non-empty" wording)

### `zbobr/src/init.rs`
- Updated `build_toml_config` to convert `WorkflowConfig.roles/pipelines` to patch types via `From` conversions when constructing `WorkflowToml`

## Test Results
All tests in `zbobr-api`, `zbobr-dispatcher`, and `zbobr` pass. The 3 pre-existing failures in `zbobr-task-backend-github` are crypto provider issues unrelated to this change.
