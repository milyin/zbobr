Fix WorkflowToml::resolve_paths in zbobr-api/src/config.rs:

1. Resolve prompts_dir against config_dir first
2. Compute prompt_base = resolved_prompts_dir.as_deref().unwrap_or(config_dir)
3. Pass prompt_base to role.resolve_paths() and pipeline.resolve_paths() instead of config_dir

Update tests:
- workflow_toml_resolve_paths_resolves_nested_prompt_fields: expect /shared/prompts/reviewer.md, /shared/prompts/review_stage.md, /shared/prompts/common.md
- workflow_toml_merge_preserves_resolved_paths_from_base: expect /shared/prompts/reviewer.md