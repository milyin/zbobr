In `zbobr/src/commands.rs`, call `prompt_builder.validate_all_prompts()?` after `ConfiguredPromptBuilder` is constructed in both startup paths:

1. **Without backends path** (~line 208): After building `prompt_builder` with `with_var` calls and before `return run_without_backends(command, &prompt_builder)`
2. **With backends path** (~line 219): After building `prompt_builder` with `with_var` calls and before constructing the dispatcher builder

**Why:** Both paths construct a `ConfiguredPromptBuilder` — validation must happen in both so that prompt errors are caught regardless of which path is taken. Placing it right after construction and before use ensures early failure with clear error messages.