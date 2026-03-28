## Add `instance` to `StageInfo` and update `MdStageTitle` format

**Part A — Update `StageInfo`** (`zbobr-api/src/task.rs`):
- Add `instance: String` field to `StageInfo`  
- Use `#[serde(default, skip_serializing_if = "String::is_empty")]` for backwards-compatible serialization (existing stored contexts without `instance` will deserialize as empty string)

**Part B — Update `MdStageTitle`** (`zbobr-api/src/context/stage_title.rs`):
- Add `instance: String` field to `MdStageTitle`
- Update `PipelineStage<'a>` wrapper struct: add `instance: &'a str` field
- Update `PipelineStage::fmt()` display: when instance is non-empty, format as `instance:pipeline:run_id:**stage**`; when empty, keep old format `pipeline:run_id:**stage**` (backwards compatibility for reading old titles)
- Update `parse_next_pipeline_stage()`: detect the new format by checking if there are 3 colon-separated parts before `:**` (instance present) vs 2 parts (old format, no instance). Parse accordingly.
- Update `From<&StageInfo> for MdStageTitle` and `From<MdStageTitle> for StageInfo` to include `instance`
- Update `MdMdStageTitleForPrompt` to use the updated `PipelineStage` wrapper

**Why:** The instance prefix in stage titles allows multiple zbobr instances to visually distinguish their stage history in the same GitHub issue context.

**Pattern to follow:** The `PipelineStage` display wrapper is the exact analog — it already formats `pipeline:run_id:**stage**`. Extend it by optionally prepending `instance:`.

**Tests:** Update existing tests to include `instance`. Add a test for backwards parsing of old-format titles (no instance prefix).