# TOML Config Merging Testing Patterns - Complete Analysis

## 1. STRUCT DEFINITIONS FOR ROLEDEFINITION, STAGEDEFINITION, AND WORKFLOWTOML

### RoleDefinition
**File:** `/data/home/skynet/zdam/zbobr-dev/workspaces/task-302/zbobr/zbobr-api/src/config.rs`
**Lines:** 26-35

```rust
pub struct RoleDefinition {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp: Option<Vec<McpTool>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<PathBuf>,
    pub tool: Option<String>,
}
```

**Key Fields:**
- `mcp: Option<Vec<McpTool>>` — List field using Option<Vec<T>> pattern
- `prompt: Option<PathBuf>` — Path field
- `tool: Option<String>` — Scalar option

**Merge Implementation:**
**Lines:** 49-57

```rust
impl zbobr_utility::MergeToml for RoleDefinition {
    fn merge_toml(self, other: Self) -> Self {
        Self {
            mcp: other.mcp.or(self.mcp),
            prompt: other.prompt.or(self.prompt),
            tool: other.tool.or(self.tool),
        }
    }
}
```

---

### StageDefinition
**File:** `/data/home/skynet/zdam/zbobr-dev/workspaces/task-302/zbobr/zbobr-api/src/config.rs`
**Lines:** 188-210

```rust
pub struct StageDefinition {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub call: Option<Pipeline>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role_prompt: Option<PathBuf>,
    /// `None` means absent in config (inherit from base during merging, or no extra prompts at runtime).
    /// `Some(vec![])` explicitly sets an empty list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompts: Option<Vec<PathBuf>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_success: Option<StageTransition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_failure: Option<StageTransition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_intermediate: Option<StageTransition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_no_report: Option<StageTransition>,
}
```

**Key Fields:**
- `prompts: Option<Vec<PathBuf>>` — **Critical:** List field with Option pattern that supports None (inherit) vs Some(vec![]) (clear)

**Merge Implementation:**
**Lines:** 257-271

```rust
impl zbobr_utility::MergeToml for StageDefinition {
    fn merge_toml(self, other: Self) -> Self {
        Self {
            role: other.role.or(self.role),
            call: other.call.or(self.call),
            tool: other.tool.or(self.tool),
            role_prompt: other.role_prompt.or(self.role_prompt),
            prompts: other.prompts.or(self.prompts),
            on_success: other.on_success.or(self.on_success),
            on_failure: other.on_failure.or(self.on_failure),
            on_intermediate: other.on_intermediate.or(self.on_intermediate),
            on_no_report: other.on_no_report.or(self.on_no_report),
        }
    }
}
```

---

### WorkflowToml
**File:** `/data/home/skynet/zdam/zbobr-dev/workspaces/task-302/zbobr/zbobr-api/src/config.rs`
**Lines:** 410-417

```rust
pub struct WorkflowToml {
    #[serde(default)]
    pub prompts_dir: Option<PathBuf>,
    #[serde(default)]
    pub roles: Option<IndexMap<String, RoleDefinition>>,
    #[serde(default)]
    pub pipelines: Option<HashMap<Pipeline, PipelineConfig>>,
}
```

**Key Fields:**
- `roles: Option<IndexMap<String, RoleDefinition>>` — **Map field** that requires key-wise merging
- `pipelines: Option<HashMap<Pipeline, PipelineConfig>>` — **Map field** that requires key-wise merging

**Merge Implementation (Crucial for understanding map-merge semantics):**
**Lines:** 464-496

```rust
pub fn merge_toml(self, other: Self) -> Self {
    Self {
        prompts_dir: other.prompts_dir.or(self.prompts_dir),
        roles: match (self.roles, other.roles) {
            (Some(mut base), Some(over)) => {
                for (k, v) in over {
                    if let Some(base_v) = base.get(&k).cloned() {
                        base.insert(k, base_v.merge_toml(v));
                    } else {
                        base.insert(k, v);
                    }
                }
                Some(base)
            }
            (None, over) => over,
            (base, None) => base,
        },
        pipelines: match (self.pipelines, other.pipelines) {
            (Some(mut base), Some(over)) => {
                for (k, v) in over {
                    if let Some(base_v) = base.get(&k).cloned() {
                        base.insert(k, base_v.merge_toml(v));
                    } else {
                        base.insert(k, v);
                    }
                }
                Some(base)
            }
            (None, over) => over,
            (base, None) => base,
        },
    }
}
```

**Pattern:** Maps iterate over overlay keys and recursively merge matching keys from base.

---

## 2. TOOLENTRY TYPE AND INDEXMAP USAGE

### ToolEntry Definition
**File:** `/data/home/skynet/zdam/zbobr-dev/workspaces/task-302/zbobr/zbobr-api/src/config.rs`
**Lines:** 107-114

```rust
pub struct ToolEntry {
    pub provider: String,
    pub model: Model,
    /// Per-entry priority override. When set, replaces the priority inherited from the provider.
    /// Use a lower value than the provider's default to mark this entry as a fallback.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
}
```

### IndexMap<String, Vec<ToolEntry>> Usage in ZbobrDispatcherConfig
**File:** `/data/home/skynet/zdam/zbobr-dev/workspaces/task-302/zbobr/zbobr-api/src/config.rs`
**Lines:** 714-716

```rust
/// Named tool definitions. Each tool is a list of (provider, model) pairs.
#[config(skip_args)]
pub tools: IndexMap<String, Vec<ToolEntry>>,
```

**Usage in Tests:**
**Lines:** 945-955 (make_config helper function)

```rust
fn make_config(
    providers: IndexMap<String, ProviderDefinition>,
    tools: IndexMap<String, Vec<ToolEntry>>,
) -> ZbobrDispatcherConfig {
    ZbobrDispatcherConfig {
        providers,
        tools,
        ..Default::default()
    }
}
```

---

## 3. MERGE METHOD IMPLEMENTATIONS

### All Merge Methods Summary

| Type | File | Lines | Pattern |
|------|------|-------|---------|
| RoleDefinition | config.rs | 49-57 | Implements `MergeToml` with `.or()` on all Option fields |
| ProviderDefinition | config.rs | 82-92 | Uses `.or()` for all Option fields |
| StageDefinition | config.rs | 257-271 | Uses `.or()` for all Option fields including `prompts` |
| PipelineConfig | config.rs | 377-388 | Key-wise merge of stages with recursive merge for matching keys |
| WorkflowToml | config.rs | 464-496 | Key-wise merge of roles and pipelines with recursive merge |

**Key Pattern:** Uses `match` on `(self, other)` tuples to handle 4 cases:
1. `(Some(base), Some(over))` — iterate overlay, recursively merge matching keys
2. `(None, over)` — use overlay
3. `(base, None)` — use base
4. Never creates both when both are Some simultaneously — always returns base + overlay modifications

---

## 4. EXISTING TESTS - COMMIT efde01cb

**File:** `/data/home/skynet/zdam/zbobr-dev/workspaces/task-302/zbobr/zbobr-dispatcher/src/cli.rs`
**Test Block:** Lines 2213-2278

### resolve_config_location tests (4 tests):
1. **resolve_config_location_default_when_empty** (Lines 2213-2219)
   - Tests that empty config paths use default "zbobr.toml"
   - Uses `resolve_config_location(&[], "zbobr.toml")`
   - Verifies config_dir == current_dir

2. **resolve_config_location_multiple_paths** (Lines 2221-2238)
   - Creates temp files in different directories
   - Tests that config_dir is derived from last file's parent
   - Uses `tempfile::tempdir()` crate
   - Verifies canonicalization

3. **resolve_config_location_missing_file_errors** (Lines 2240-2244)
   - Tests error handling for nonexistent files
   - Verifies `.is_err()` result

4. **config_file_arg_short_flag_registered** (Lines 2248-2261)
   - Tests clap argument structure
   - Verifies `-c` short alias exists on `--config`
   - Uses `GlobalArgs::augment_args(clap::Command::new(""))`

5. **global_args_includes_logs_flag** (Lines 2263-2278)
   - Tests `--logs` flag is SetTrue action
   - Verifies flag structure via clap

---

## 5. EXISTING TESTS - COMMIT 23570484

**File:** `/data/home/skynet/zdam/zbobr-dev/workspaces/task-302/zbobr/zbobr-api/src/config.rs`
**Test Block:** Lines 1621-2265

### resolve_paths tests:

1. **role_definition_resolve_paths_makes_prompt_absolute** (Lines 1623-1635)
   - Relative path becomes absolute under config_dir
   - Calls `role.resolve_paths(Path::new("/shared/configs"))`

2. **role_definition_resolve_paths_preserves_absolute** (Lines 1637-1649)
   - Absolute paths remain unchanged

3. **stage_definition_resolve_paths_resolves_all_prompt_fields** (Lines 1651-1670)
   - Resolves `role_prompt` and `prompts` fields
   - Tests both relative and absolute paths in same structure

4. **pipeline_config_resolve_paths_resolves_stage_prompts** (Lines 1672-1692)
   - Tests nested resolution through IndexMap stages

5. **workflow_toml_resolve_paths_resolves_nested_prompt_fields** (Lines 1694-1744)
   - Tests full nested resolution: WorkflowToml → roles → stages → prompts
   - Verifies prompts_dir is used as base when present

6. **workflow_toml_merge_preserves_resolved_paths_from_base** (Lines 1746-1777)
   - **Critical test:** Shows path resolution followed by merge
   - Verifies base paths stay anchored to original config_dir after merge
   - Pattern: resolve per-file, then merge

### merge_toml tests (9 major tests):

1. **workflow_toml_merge_roles_key_wise** (Lines 1788-1844)
   - Base defines 2 roles, overlay overrides 1
   - Verifies modified role is overridden AND unmodified role survives
   - Assert both `roles["reviewer"]` and `roles["worker"]` exist with correct values

2. **workflow_toml_merge_pipelines_key_wise** (Lines 1846-1912)
   - Base defines 2 pipelines, overlay overrides 1
   - Tests `Pipeline::Main` and `Pipeline::Custom("fix")`
   - Verifies merged map has 3 entries total

3. **dispatcher_toml_merge_providers_key_wise** (Lines 1914-1982)
   - Uses `ZbobrDispatcherConfigToml` (macro-generated struct)
   - Base has 2 providers, overlay overrides 1 and adds 1
   - Verifies final map has 3 providers
   - Pattern shows how to test map merging at dispatcher level

4. **provider_partial_patch_preserves_base_fields** (Lines 1984-2037)
   - **Critical:** Demonstrates `.or()` semantics for partial patches
   - Base provider has `executor`, `priority`, `plan_mode`
   - Overlay only provides `priority` (executor and plan_mode are None)
   - Verifies `executor` and `plan_mode` survive from base

5. **role_partial_patch_preserves_base_fields** (Lines 2039-2094)
   - Base role has `mcp: Some(vec![...])` and `prompt: Some(...)`
   - Overlay has `mcp: None` and only sets `tool`
   - Verifies mcp and prompt survive via `.or(self)` pattern
   - **Key insight:** `None` means "absent in overlay, inherit from base"

6. **role_mcp_cleared_by_empty_list_overlay** (Lines 2096-2138)
   - **Critical for Vec<T> handling:** Shows difference between `None` and `Some(vec![])`
   - Base has `mcp: Some(vec![McpTool::ReportSuccess])`
   - Overlay has `mcp: Some(vec![])` (explicit empty)
   - Verifies merged.mcp is `Some(vec![])` (not inherited)

7. **pipeline_partial_stage_patch_preserves_other_stages** (Lines 2140-2209)
   - Nested map merging: pipeline contains stages (IndexMap)
   - Base pipeline has 2 stages, overlay patches 1
   - Verifies modified stage role changed, tool preserved, other stage unchanged
   - Pattern shows recursive merge through nested structures

8. **stage_prompts_cleared_by_empty_list_overlay** (Lines 2211-2264)
   - Stage has `prompts: Some(vec![PathBuf::from(...)])`
   - Overlay sets `prompts: Some(vec![])` (explicit empty)
   - Verifies prompts are cleared (not inherited)
   - **Key:** Demonstrates Option<Vec<>> semantics for list clearing

---

## 6. MERGETOML TRAIT DEFINITION

**File:** `/data/home/skynet/zdam/zbobr-dev/workspaces/task-302/zbobr/zbobr-utility/src/lib.rs`
**Lines:** 10-35

```rust
/// Trait for merging two TOML configuration snapshots.
///
/// The overlay's value wins for `Option` scalar fields; map fields are merged key-by-key
/// with recursive merging for matching keys; list fields replace the base wholesale.
///
/// For structured types that contain list fields (e.g. role `mcp` or stage `prompts`),
/// use `Option<Vec<_>>` in the TOML representation and merge with `.or()` semantics:
/// a `None` field means "inherit from base"; `Some(v)` (even `Some(vec![])`) means
/// "replace base wholesale".
pub trait MergeToml: Sized {
    fn merge_toml(self, other: Self) -> Self;
}

// Lists are always replaced wholesale
impl<T> MergeToml for Vec<T> {
    fn merge_toml(self, other: Self) -> Self {
        other
    }
}

// Scalar path values replace wholesale when used as map values
impl MergeToml for std::path::PathBuf {
    fn merge_toml(self, other: Self) -> Self {
        other
    }
}
```

---

## 7. TEST STYLE AND PATTERNS

### Helper Functions Pattern:
**Lines:** 945-955

```rust
fn make_config(
    providers: IndexMap<String, ProviderDefinition>,
    tools: IndexMap<String, Vec<ToolEntry>>,
) -> ZbobrDispatcherConfig {
    ZbobrDispatcherConfig { providers, tools, ..Default::default() }
}

fn make_workflow_with_role(role_name: &str, tool: Option<String>) -> WorkflowConfig {
    let mut roles = IndexMap::new();
    roles.insert(role_name.to_string(), RoleDefinition { ... });
    WorkflowConfig { ..Default::default() }
}
```

### Test Structure Pattern:
1. Build structures manually with `IndexMap::new()` and `.insert()`
2. For nested structures use `IndexMap::from([(...), (...)])` shorthand
3. Build overlay with subset of modifications
4. Call `.merge_toml()` method
5. Assert both modified AND unmodified entries
6. Use `assert_eq!` with specific field access patterns like `roles["key"]`

### Assertion Patterns:
- Direct field access: `assert_eq!(roles["key"].field, expected)`
- Option unwrap: `assert_eq!(roles["key"].tool.as_deref(), Some("value"))`
- Verification of list content: `assert!(roles["worker"].mcp.as_ref().is_some_and(|v| !v.is_empty()))`
- Clearing verification: `assert!(roles["worker"].mcp.as_ref().is_some_and(|v| v.is_empty()))`

---

## 8. KEY INSIGHTS FOR TESTING TOML MERGING

### The Option<Vec<T>> Pattern
- **None in overlay** → Field inherits from base (via `.or()`)
- **Some(vec![])** in overlay → Field is explicitly cleared (replaces base)
- This allows distinguishing between "not specified" (None) and "explicitly empty" (Some(vec![]))

### Map Merging Algorithm
```rust
for (k, v) in overlay {
    if let Some(base_v) = base.get(&k).cloned() {
        base.insert(k, base_v.merge_toml(v));  // Recursive merge
    } else {
        base.insert(k, v);  // New key from overlay
    }
}
```

### Path Resolution Pattern
1. Each config file resolves paths against its own directory
2. After resolution, paths become absolute
3. Merge happens on already-resolved paths
4. Base paths stay anchored to original config_dir (they're already absolute)

### Test Coverage Essentials
- Overlay overrides base key
- Overlay adds new key
- Base key survives when not in overlay
- Partial field patches preserve other fields
- Empty list overlay clears base list
- Nested structure recursive merging
- Tool entries with priority field
