Change the serialization and parsing of `MdStageTitle` in `zbobr-api/src/context/stage_title.rs`.

**New format** (replace the old `<sub>` timestamp pattern):
```
pipeline:run_id:**stage** `tool` `model` `YYYY-MM-DD HH:MM:SS +HHMM` <sub>[prompt](url)</sub> <sub>[output](url)</sub>
```
- Timestamp moves from `<sub>...</sub>` / `<sub>[timestamp](url)</sub>` to a backtick token (like tool/model)
- Prompt link becomes an optional standalone `<sub>[prompt](url)</sub>` element
- Output link becomes an optional standalone `<sub>[output](url)</sub>` element
- Both `<sub>` link elements are omitted when the respective link is `None`

**Display (`fmt::Display`)**: emit timestamp as `` `YYYY-MM-DD HH:MM:SS +HHMM` ``, then optionally ` <sub>[prompt](url)</sub>`, then optionally ` <sub>[output](url)</sub>`.

**Parsing (`FromStr`)**: after parsing the backtick tokens (tool, model), try to parse one more backtick as the timestamp. Then try to parse zero or more `<sub>...</sub>` elements, identifying them by their label (`prompt` or `output`). For **backward compatibility**, also handle the old format where the timestamp is in a trailing `<sub>` (with or without link).

**`MdMdStageTitleForPrompt`** wrapper: update to use the new backtick timestamp format and omit both prompt and output links.

Update all tests in the module to match the new format, and add tests for:
- New format roundtrip
- Old format parsing (backward compat)
- With/without prompt link, with/without output link