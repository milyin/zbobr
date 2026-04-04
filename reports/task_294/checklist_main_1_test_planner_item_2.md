# Test: PROMPT_FILES completeness for default workflow roles

## Location
`zbobr/src/init.rs` — `#[cfg(test)] mod tests`

## What to test
Iterate all roles in `default_workflow().roles`, collect those with a `prompt` field set, and assert each prompt file name (without `.md`) appears as a key in `PROMPT_FILES`.

```rust
#[test]
fn all_default_workflow_role_prompts_are_registered() {
    let wf = default_workflow();
    let registered: std::collections::HashSet<&str> =
        PROMPT_FILES.iter().map(|(name, _)| *name).collect();
    for (role_name, role_def) in &wf.roles {
        if let Some(prompt_path) = &role_def.prompt {
            let key = prompt_path
                .file_stem()
                .and_then(|s| s.to_str())
                .expect("prompt path has no file stem");
            assert!(
                registered.contains(key),
                "Role '{}' references prompt file '{}' but it is not in PROMPT_FILES",
                role_name, key
            );
        }
    }
}
```

## Rationale
This test guards against silent missing-prompt bugs. If a new role is added to `default_workflow()` with a `prompt` path but its content is not registered in `PROMPT_FILES`, the setup command will produce an incomplete prompts directory. The `linter_worker` addition is a concrete example where this could go wrong; the test ensures future additions are consistent.
