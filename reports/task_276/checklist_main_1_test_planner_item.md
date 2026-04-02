## Test: `default_workflow_roles_have_tool`

**File**: `zbobr/src/init.rs`, inside existing `mod tests` block (after line ~877)

**Purpose**: Prevent regression where predefined roles in `default_workflow()` lack a `tool` field, which would make `zbobr init` produce an invalid config that fails `validate_workflow_refs`.

**Implementation**:
```rust
#[test]
fn default_workflow_roles_have_tool() {
    let workflow = default_workflow();
    for (name, role_def) in &workflow.roles {
        assert!(
            role_def.tool.is_some(),
            "Role '{}' in default_workflow must have a tool defined",
            name
        );
    }
}
```

**Verification**: `cargo test -p zbobr -- default_workflow_roles_have_tool`