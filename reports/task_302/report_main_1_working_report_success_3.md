# Fix: Map-type config sections now merge key-wise

## Problem
The previous multi-config merge implementation replaced entire map-type sections (IndexMap/HashMap) wholesale when a later config defined any value for that section. This broke the core requirement that "named parameters override parameters with the same name" — a project overlay defining one provider/role/pipeline would silently drop all entries from the shared base config.

## Changes Made

### zbobr-macros/src/lib.rs
- Added `is_map_type()` helper function that detects `IndexMap<K,V>` and `HashMap<K,V>` by type name
- Modified the `merge_toml_fields` code generation: when a field is a map type, generates key-wise `extend()`-based merge instead of `other.or(self)` replacement
- This fixes `ZbobrDispatcherConfigToml::merge_toml()` for `providers` and `tools` fields

### zbobr-api/src/config.rs
- `WorkflowToml::merge_toml()`: changed `roles` and `pipelines` from `other.roles.or(self.roles)` to key-wise `extend()` merge
- Added 3 new behavior tests:
  - `workflow_toml_merge_roles_key_wise`: base has 2 roles, overlay overrides 1 → both preserved, overridden one updated
  - `workflow_toml_merge_pipelines_key_wise`: base has 2 pipelines, overlay overrides 1 → both preserved, overridden one updated  
  - `dispatcher_toml_merge_providers_key_wise`: base has 2 providers, overlay overrides 1 and adds 1 → all 3 providers in final config

## Test Results
All 113 tests in zbobr-api pass. Build succeeds.

## Commit
`42aa4349 fix: merge map-type config sections key-wise instead of wholesale replacement`