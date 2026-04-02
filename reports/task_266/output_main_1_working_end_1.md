All done. Added the 4 missing unit tests for `validate_all_prompts()` that the review flagged:

1. **Valid templates pass** — verifies Ok with a well-formed prompt
2. **Undefined variable fails** — catches `{mcp_nonexistent}` 
3. **Missing prompt file fails** — catches non-existent file reference
4. **Call stages skipped** — confirms call stages don't trigger validation errors

All 21 tests pass. Committed as `367a23b6`.