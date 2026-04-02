Add unit tests in `zbobr-dispatcher/src/prompts.rs` test module covering:
1. Valid templates pass validation
2. Undefined placeholder like `{mcp_nonexistent}` fails validation
3. Missing prompt file fails validation
4. `call` stages are skipped (no error even without prompt files)