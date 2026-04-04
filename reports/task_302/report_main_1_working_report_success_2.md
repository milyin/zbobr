## Fix: Complete per-file path resolution for workflow prompt fields

### Problem
The review identified that `WorkflowToml::resolve_paths()` only resolved `prompts_dir`, leaving relative prompt paths in `RoleDefinition.prompt`, `StageDefinition.role_prompt`, and `StageDefinition.prompts` unresolved. When merging configs from different directories, these relative paths would be resolved against the wrong base directory (the last config file's location instead of their source config's location).

### Changes

**`zbobr-api/src/config.rs`** (2 commits):

1. **Added `resolve_paths()` methods** to:
   - `RoleDefinition` — resolves `prompt: Option<PathBuf>`
   - `StageDefinition` — resolves `role_prompt: Option<PathBuf>` and `prompts: Vec<PathBuf>`
   - `PipelineConfig` — delegates to all contained stages

2. **Updated `WorkflowToml::resolve_paths()`** to also walk into `roles` and `pipelines`, calling their respective `resolve_paths()` methods so all prompt path fields become absolute before cross-file merge.

3. **Added 6 tests** covering:
   - `RoleDefinition::resolve_paths` with relative and absolute paths
   - `StageDefinition::resolve_paths` resolving both `role_prompt` and `prompts`
   - `PipelineConfig::resolve_paths` delegating to stages
   - `WorkflowToml::resolve_paths` resolving all nested prompt fields
   - End-to-end: per-file resolution + merge preserves paths anchored to source config directory

### How it works
The existing flow in `main.rs` already calls `parsed.resolve_paths(file_dir)` on each config file before merging. The `RootConfigToml::resolve_paths()` delegates to `WorkflowToml::resolve_paths()`, which now fully resolves all path fields. After resolution, all paths are absolute, so merging configs from different directories is safe.

### Test results
All 110 tests in zbobr-api pass. All 94 tests in zbobr-dispatcher pass. No regressions.