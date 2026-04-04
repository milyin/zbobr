Extend WorkflowToml::resolve_paths() to resolve all relative path fields:
- RoleDefinition.prompt (Option<PathBuf>)
- StageDefinition.role_prompt (Option<PathBuf>)  
- StageDefinition.prompts (Vec<PathBuf>)

These paths need to become absolute before cross-file merge so that shared configs with relative prompt paths work correctly regardless of which config file they came from.