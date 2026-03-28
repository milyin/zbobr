Add `output_link: Option<String>` to `StageInfo` in `zbobr-api/src/task.rs`, mirroring the existing `prompt_link` field (same serde attributes: `default`, `skip_serializing_if = "Option::is_none"`).

Update `MdStageTitle` in `zbobr-api/src/context/stage_title.rs`:
- Add `output_link: Option<String>` field
- Update `From<&StageInfo> for MdStageTitle` and `From<MdStageTitle> for StageInfo` to include the new field

This is purely additive data-model work; no format or logic changes yet.