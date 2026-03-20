// Prompt-related types for zbobr.
//
// The `PromptBuilder` trait and per-role `ToolNames` structs have been removed
// in favor of configurable roles. Each role now defines its allowed tools via
// `RoleDefinition` in `WorkflowConfig`, and prompts are loaded from files or
// built-in defaults.
