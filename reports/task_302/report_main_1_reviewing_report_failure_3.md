Overall assessment: **fail**.

The branch is close, and the analog choice was still broadly reasonable: layering `merge_toml` alongside the existing `merge_with_args` flow fits the codebase, and the path-resolution fixes are aligned with the earlier review findings. The changed files are all task-related; I did not find unrelated edits. However, one blocking correctness issue remains, and it still conflicts with the task’s main requirement of sharing common config with project-specific patches.

## Blocking issue: conflicting named entries are still replaced as whole values

### Where
- `zbobr-macros/src/lib.rs:322-331`
- `zbobr-api/src/config.rs:405-423`
- impacted config/value types:
  - `dispatcher.providers: IndexMap<String, ProviderDefinition>` (`zbobr-api/src/config.rs:631-636`)
  - `workflow.roles: Option<IndexMap<String, RoleDefinition>>` (`zbobr-api/src/config.rs:355-357`)
  - `workflow.pipelines: Option<HashMap<Pipeline, PipelineConfig>>` (`zbobr-api/src/config.rs:355-357`)
  - and inside pipelines, `PipelineConfig.stages: IndexMap<Stage, StageDefinition>` (`zbobr-api/src/config.rs:233-246`)

### What is wrong
The latest fix correctly preserves **different keys** in map-like sections, but for the **same key** it still replaces the whole value instead of merging that value’s fields.

Current behavior:
- macro-generated map merge uses `base.extend(over)` for every `IndexMap` / `HashMap`
- `WorkflowToml::merge_toml()` does the same for `roles` and `pipelines`

That means:
- if the overlay defines the same provider name, the entire `ProviderDefinition` is replaced
- if the overlay defines the same role name, the entire `RoleDefinition` is replaced
- if the overlay defines the same pipeline name, the entire `PipelineConfig` is replaced, so untouched stages from the base pipeline are dropped

This is still too shallow for the task requirement: **named parameters override parameters with the same name**. A project-specific patch should be able to override one field inside a shared named section without having to restate the whole section.

### Concrete failure shapes
1. **Provider patch loses base fields**
   - base: `[dispatcher.providers.shared] executor = "copilot" priority = 10`
   - overlay: `[dispatcher.providers.shared] priority = 20`
   - result now: `executor` is lost, because the overlay `ProviderDefinition` replaces the whole base entry.

2. **Role patch loses base fields**
   - base role defines `mcp`, `prompt`, maybe `tool`
   - overlay role sets only `tool`
   - result now: base `mcp` / `prompt` disappear because the overlay role replaces the whole base role.

3. **Pipeline patch loses untouched stages**
   - base `workflow.pipelines.main` contains several stages
   - overlay defines only one stage change under the same pipeline
   - result now: `base.extend(over)` at the pipeline-map level replaces the entire `PipelineConfig`, so all untouched base stages vanish.

This still prevents the intended “shared base config + small project patch” workflow.

## Why this is blocking
The task description explicitly targets configuration sharing through layered patches. With the current merge semantics, overlays still need to repeat entire provider/role/pipeline definitions whenever they touch an existing named entry. That defeats the core goal and leaves partial patches unsafe.

## Suggested fix
1. **Merge map values recursively when the same key exists**, instead of always replacing them.
   - For `ProviderDefinition`, merge each field so later fields override earlier ones.
   - For `RoleDefinition`, merge fields; list fields like `mcp` should still replace as whole values.
   - For `PipelineConfig`, merge `stages` key-by-key.
   - For `StageDefinition`, merge fields; list fields like `prompts` should still replace as whole values.
2. Keep whole-value replacement only where it actually matches the requirement, e.g. `tools: IndexMap<String, Vec<ToolEntry>>`, because each tool entry value is a list and lists should replace wholesale.
3. Add behavior-oriented tests for **partial same-key overlays**, not only whole-entry overrides.
   - provider: overlay only changes `priority`
   - role: overlay only changes `tool` or `prompt`
   - pipeline: overlay changes one stage while preserving other stages from base

## Test coverage note
The new tests cover map preservation for distinct keys and whole-entry override cases, but they do not cover the blocking behavior above: **partial overlays on the same named entry**. That gap is why this issue slipped through.

## Analog consistency
The analog was fine, but it was applied too mechanically. Extending maps by key is a step forward, yet for named config sections whose values are themselves structured configs, it still does not match the repository’s existing recursive-merge pattern or the task requirement.

## Checklist status
I did not mark the remaining unchecked checklist items complete. The final map-merge work is still incomplete for same-name partial patches, so those items should stay open until recursive value merging and coverage for it are added.