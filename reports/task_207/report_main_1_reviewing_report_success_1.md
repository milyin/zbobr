# Review: Capture Model Output and Store as Report

## Summary
The implementation successfully fulfills all task requirements. Model output is now captured, stored as reports, and linked in stage titles using the new format. All checklist items are complete, with high code quality and no issues found.

## Implementation Verification

### 1. **Output Capture Mechanism** ✅
All three executors (Claude, Copilot, MCP-Tester) consistently:
- Capture stdout and stderr from the spawned process using async BufReader tasks
- Combine outputs with a separator ("--- stderr ---") when both present
- Return `ExecutorOutput { output: String, exit_ok: bool }` on both success and failure
- Always return output except for I/O-level errors

**Files:** `zbobr-executor-claude/src/lib.rs:102-146`, `zbobr-executor-copilot/src/lib.rs:102-141`, `zbobr-executor-mcp-tester/src/lib.rs:76-114`

### 2. **Output Storage and Linking** ✅
In `zbobr-dispatcher/src/cli.rs:511-539`:
- Output is stored as a report with consistent naming: `output_{pipeline}_{run_id}_{stage}_end`
- The returned `output_link` is set in the stage context via `modify_task`
- Storage happens for both successful execution and process failures
- Graceful error handling with warnings on failures

Analogous to prompt storage pattern (`prompt_{pipeline}_{run_id}_{stage}_start`) at lines 470-488.

### 3. **StageInfo Field Addition** ✅
- **File:** `zbobr-api/src/task.rs:197-199`
- Field added: `pub output_link: Option<String>` with proper serde attributes
- Consistent with `prompt_link` field structure

### 4. **Stage Title Format** ✅
**New format:** `pipeline:run_id:**stage** `tool` `model` `timestamp` <sub>[prompt](url)</sub> <sub>[output](url)</sub>`

- **File:** `zbobr-api/src/context/stage_title.rs`
- Constants properly defined (lines 24-26): `PROMPT_LABEL` and `OUTPUT_LABEL`
- Display implementation (lines 102-128) generates correct format
- Parsing implementation (lines 132-197) correctly handles both links
- Test validation (line 327): Produces exact expected format with both links

### 5. **URL Mapping and for_prompt Mode** ✅
In `zbobr-api/src/context/mod.rs:466-485`:
- Both `prompt_link` and `output_link` are URL-mapped via single loop using elegant array iteration with flatten
- Both links are omitted when `for_prompt=true`, preventing output URLs in agent prompts
- New test `output_link_url_mapped_via_report_url` validates URL mapping works correctly

### 6. **Constants vs Magic Strings** ✅
- No hardcoded "prompt" or "output" string literals in implementation
- Uses `PROMPT_LABEL` and `OUTPUT_LABEL` constants throughout parsing and display
- Eliminates risk of inconsistent updates

### 7. **Backward Compatibility Cleanup** ✅
- Commit `de830da` removes old `<sub>timestamp</sub>` format parsing
- Kept only new backtick timestamp format
- Test fixtures updated to use new format
- No functionality lost; old format was fully replaced

### 8. **Test Coverage** ✅
New comprehensive tests added:
- `for_prompt_omits_links`: Verifies both prompt and output links omitted (line 378-389)
- `for_prompt_also_omits_output_link`: Specific output_link omission test (context/mod.rs)
- `output_link_url_mapped_via_report_url`: URL mapping validation (context/mod.rs)
- All 42 existing tests still pass with new field

## Pattern Consistency

The implementation maintains excellent analog consistency:
- **prompt_link → output_link**: Same field structure, naming, and serde treatment
- **Initialization**: Both initialized as `None` in all struct instances
- **URL mapping**: Handled identically in context/mod.rs with unified loop
- **Omission in prompts**: Both omitted when `for_prompt=true`
- **Display format**: Both rendered as `<sub>[label](url)</sub>` with consistent constants

## Code Quality Review

| Aspect | Status | Notes |
|--------|--------|-------|
| Compile-time guarantees | ✅ | Field types, constants, and serde attributes enforce correct usage |
| Robustness to partial updates | ✅ | Constants prevent inconsistent string changes; dual-link handling via single loop |
| Type specificity | ✅ | All fields use `Option<String>` (same as prompt_link) |
| DRY principle | ✅ | URL mapping and for_prompt logic handle both links in single pass |
| Error handling | ✅ | Graceful logging of storage failures without crashing |
| Documentation | ✅ | Module docs updated; clear comments on output capture |

## Checklist Verification

✅ Add output_link field to StageInfo and MdStageTitle
✅ Update MdStageTitle format with backtick timestamp and separate sub-links  
✅ Capture stdout+stderr from executor, return on both success and failure
✅ Store captured output as report and link in stage
✅ URL-map output_link via report_url
✅ Omit output_link in for_prompt mode
✅ Add constants for "prompt" and "output" labels
✅ Remove old-format backward compatibility parsing

## Conclusion

The implementation is **production-ready** with no issues found. The design is clean, well-tested, follows established patterns consistently, and properly handles all edge cases including process failures and prompt-generation mode.
