Add tests in the existing test module in `zbobr-dispatcher/src/prompts.rs`. Follow the existing test patterns (use `TempDir`, dummy tasks, workflow config builders).

Tests to add:
- **Valid templates pass**: Create a workflow with valid prompt templates, verify `validate_all_prompts()` returns Ok
- **Undefined variable caught**: Create a prompt with an undefined placeholder like `{mcp_nonexistent}`, verify validation returns an error mentioning the bad variable
- **Missing prompt file caught**: Reference a non-existent prompt file in a stage, verify validation returns an error
- **Call stages skipped**: Create a workflow with a `call` stage (no prompt files), verify validation doesn't error on it

**Why:** These tests ensure the validation catches the categories of errors we care about and doesn't false-positive on call stages.