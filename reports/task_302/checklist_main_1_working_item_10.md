Create three new "patch" structs in zbobr-api/src/config.rs that are used only in WorkflowToml for TOML deserialization and merging:

1. `RoleDefinitionPatch`: like `RoleDefinition` but with `mcp: Option<Vec<McpTool>>` (instead of `Vec`). Absent = None = inherit base; present (even `[]`) = Some(v) = replace base.

2. `StageDefinitionPatch`: like `StageDefinition` but with `prompts: Option<Vec<PathBuf>>` (instead of `Vec`). Same semantics.

3. `PipelineConfigPatch`: like `PipelineConfig` but with `stages: IndexMap<Stage, StageDefinitionPatch>`.

Changes needed:
- Add the three new types with MergeToml impls that use `.or()` for list fields
- Add resolve_paths methods for the patch types
- Change `WorkflowToml.roles` type from `Option<IndexMap<String, RoleDefinition>>` to `Option<IndexMap<String, RoleDefinitionPatch>>`
- Change `WorkflowToml.pipelines` type from `Option<HashMap<Pipeline, PipelineConfig>>` to `Option<HashMap<Pipeline, PipelineConfigPatch>>`
- Update `WorkflowToml::merge_toml` to use the new types (same structure, just different types)
- Update `WorkflowToml::resolve_paths` to call `RoleDefinitionPatch::resolve_paths` and `PipelineConfigPatch::resolve_paths`
- Update `WorkflowToml::try_into_config` to convert `RoleDefinitionPatch → RoleDefinition` (mcp.unwrap_or_default()) and `PipelineConfigPatch → PipelineConfig` (stages with StageDefinitionPatch → StageDefinition)
- Update all tests in config.rs that create `WorkflowToml` with `RoleDefinition` / `StageDefinition` / `PipelineConfig` to use the patch types
- Update `zbobr/src/init.rs` to use `RoleDefinitionPatch` / `StageDefinitionPatch` / `PipelineConfigPatch` in the `WorkflowToml` it creates
- Add tests for `mcp = []` and `prompts = []` clearing base lists in overlays
