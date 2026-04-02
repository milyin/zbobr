All done. Both checklist items implemented and all tests pass:

1. **Model no-spaces enforcement**: `Model::try_new()` rejects whitespace; `FromStr`/`Deserialize` delegate to it. `StageInfo.model`, `ToolEntry.model`, and `MdStageTitle.model` now use `Option<Model>`. `select_provider` returns `Model` directly.

2. **Eager tool-name validation**: `ZbobrDispatcherConfig::validate()` now checks the global `tool` field exists in `[tools]`. New `validate_workflow_refs()` checks all `role.tool` and `stage.tool` references. `ZbobrDispatcher::validated()` calls it at startup.