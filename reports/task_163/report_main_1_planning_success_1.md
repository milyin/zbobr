# Plan: Replace Checklist with TaskContext Structure

## Rationale

The previous plan had 9 steps and was largely correct but needed fixes:
1. **StageInfo types**: User flagged that `pipeline` and `stage` fields must use `Pipeline` (enum) and `Stage` (struct) types instead of strings
2. **No comment duplication**: User explicitly said "The comments should not be posted at all. Stages results are stored in context only, no duplication" — so `report_impl` must NOT call `post_comment`
3. **StageInfo.run_id**: The MD stage header format `<!-- Stage: pipeline #run_id stage -->` requires run_id in StageInfo, which the current struct lacks

## Analog

The implementation follows the existing checklist pattern as the primary analog:
- `checklist_format.rs` → `context_format.rs` (same module structure, serialize/parse pair)
- `CHECKLIST_SEPARATOR` → `CONTEXT_SEPARATOR` in separator.rs (same section-based approach)
- `RoleSession` checklist methods → context methods (same modify_task + Arc<Mutex> pattern for return values)
- `McpTool` enum changes follow the existing pattern of as_str/FromStr/ALL_TOOLS constants

## Key Design Decisions

1. **Global numeric IDs** instead of pipeline-scoped string IDs — simplifies everything, no more scope prefix manipulation
2. **Result return from parse_description_full** — context parsing errors must be propagated immediately (task description says "do not try to make assumptions")
3. **store_report on RoleSession** — since reports no longer go through comments, RoleSession needs direct access to report storage
4. **pipeline_scope removal** — since context has no scoping, the pipeline_scope parameter is removed from all prompt-building functions
5. **run_id in StageInfo** — needed for the MD format stage header, added as a new field

## Implementation Order (dependency chain)

1. fix-stage-info-types (unblocks context-format)
2. create-context-format (unblocks separator, prompts)
3. update-separator (unblocks backends)
4. update-backends (parallel: GitHub + FS)
5. update-mcp-definitions (unblocks mcp-impls)
6. update-role-session (unblocks mcp-impls)
7. update-mcp-impls (depends on 5+6)
8. update-prompts (depends on 2)
9. stage-creation-cli (depends on 1+6)
10. cleanup-and-tests (final, depends on all above)