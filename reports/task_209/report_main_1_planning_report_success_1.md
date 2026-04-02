## Approach

Two targeted fixes, both constrained to the validation/type boundary layer without touching the dispatch or execution logic.

### Fix 1: Model type enforcement
`Model::try_new` is the single source of truth for the no-spaces constraint. All construction paths (FromStr, Deserialize) go through it. ToolEntry and StageInfo switch from raw String to typed Model so the constraint is enforced wherever models are parsed or constructed — catching bad config at parse time rather than at persistence time.

### Fix 2: Eager tool-name validation
`validate()` gains a check on the global `dispatcher.tool`. A new `validate_workflow_refs()` method handles roles and stages. Both are called from `ZbobrDispatcher::validated()`, the existing single entry point for config correctness. The `Workflow::config()` getter already exists, so no new accessors are needed.

### Key constraints
- Keep the `Model` wrapper type (user requirement).
- All 20 existing tests must continue to pass.
- New tests should cover: Model rejects spaces, validate() rejects unknown global tool, validate_workflow_refs() rejects unknown tool in role/stage.