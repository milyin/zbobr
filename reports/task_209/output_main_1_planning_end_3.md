Plan is finalized with two checklist items:

1. **Model type enforcement** — add `Model::try_new` rejecting whitespace, update `FromStr`/`Deserialize`, change `ToolEntry.model` and `StageInfo.model` to `Model`, update `select_provider` return type to `(ResolvedProvider, Model)`.

2. **Eager tool-name validation** — extend `validate()` to check the global `dispatcher.tool`, add `validate_workflow_refs()` for roles/stages, call it from `ZbobrDispatcher::validated()`.