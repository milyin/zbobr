# Fix: Recursive merge for same-key map entries in config

## Problem
When merging two config files where the overlay redefined a named entry (e.g. a provider, role, or pipeline stage), the entire value was replaced wholesale. This prevented the intended "shared base + small project patch" workflow.

## Solution

### New `MergeToml` trait (zbobr-utility)
- `pub trait MergeToml: Sized { fn merge_toml(self, other: Self) -> Self; }`
- Blanket impl for `Vec<T>`: returns `other` (lists replace wholesale)
- Impl for `PathBuf`: returns `other` (scalar paths replace wholesale)

### Impl for structured config types (zbobr-api)
- `ProviderDefinition::merge_toml`: field-level Option merge (`other.field.or(self.field)`)
- `RoleDefinition::merge_toml`: Option fields merge, `mcp` list uses overlay if non-empty
- `StageDefinition::merge_toml`: all Option fields merge; `prompts` list uses overlay if non-empty
- `PipelineConfig::merge_toml`: merges `stages` map key-by-key with recursive `StageDefinition::merge_toml`

### Macro update (zbobr-macros)
Changed the `is_map_type` branch from `base.extend(over)` to key-by-key iteration:
```rust
for (k, over_v) in over {
    if let Some(base_v) = base.get(&k).cloned() {
        base.insert(k, ::zbobr_utility::MergeToml::merge_toml(base_v, over_v));
    } else {
        base.insert(k, over_v);
    }
}
```
This preserves `IndexMap` insertion order for existing keys while appending new ones.

### WorkflowToml::merge_toml updated
Uses the same key-by-key pattern for `roles` and `pipelines`.

## Tests added
- `provider_partial_patch_preserves_base_fields`: overlay changes only `priority`, verifies `executor` and `plan_mode` survive from base
- `role_partial_patch_preserves_base_fields`: overlay sets only `tool`, verifies `mcp` and `prompt` survive
- `pipeline_partial_stage_patch_preserves_other_stages`: overlay patches one stage's role, verifies stage's `tool` and other stages survive unchanged

All 116 zbobr-api tests pass.