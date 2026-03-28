## Review summary
The change set largely follows the planned analog (prompt storage + prompt_link in stage title) and updates the stage title format + parsing + executor output capture as required. However, there is a **high-likelihood functional bug** for GitHub-backed tasks: `output_link` is stored as a backend filename but is **never converted to a display URL** when serializing context, so the new `<sub>[output](...)</sub>` link is likely broken/relative in GitHub issue descriptions.

Because this directly impacts the task requirement (“Add link to this file to the title of the stage”), this review is **FAIL** until fixed.

---

## What changed (quick audit)
- `StageInfo` gained `output_link: Option<String>` (zbobr-api/src/task.rs).
- Stage title format updated to:
  - backticked timestamp
  - optional `<sub>[prompt](url)</sub>` and `<sub>[output](url)</sub>`
  - backward-compatible parsing of old `<sub>` timestamp format (zbobr-api/src/context/stage_title.rs).
- `ToolExecutor::execute` now returns `ExecutorOutput { output, exit_ok }`, and executors capture **stdout + stderr** and return output even on non-zero exit (zbobr-api/src/tool_executor.rs + executor crates).
- Dispatcher stores output after execution via `store_report(...)` and sets `stage.info.output_link` (zbobr-dispatcher/src/cli.rs).

These are all aligned with the plan.

---

## Blocking issue 1: `output_link` is not URL-transformed when rendering context
**Where:** zbobr-api/src/context/mod.rs, `MdStage::from_stage_context`.

Current behavior:
- `prompt_link` is optionally transformed via `report_url` if it’s not already http(s).
- **`output_link` is not transformed at all.**

This matters because:
- In the GitHub backend, `store_report` returns **a filename** (e.g. `output_main_..._end.md`), not a full URL.
- For GitHub issue descriptions/comments, relative links won’t resolve to the repo’s reports branch; they’ll be relative to the issue page.

So stage titles will likely look like:
`... <sub>[output](output_main_1_working_end.md)</sub>`
which does not meet the requirement example (`https://...`).

**Required fix:** Apply the same `report_url` mapping logic to `title.output_link` that is applied to `title.prompt_link`.

Suggested patch shape:
- In `MdStage::from_stage_context`, after the `prompt_link` transform block, add an equivalent block for `output_link`.

---

## Blocking issue 2: prompt-mode context may include output link
**Where:** zbobr-api/src/context/mod.rs, `MdStage::from_stage_context`.

For `for_prompt == true`, the code sets `title.prompt_link = None`, but leaves `title.output_link` intact. Since `MdStageTitle::Display` will emit the output sub-link, the prompt context passed to agents could include `<sub>[output](...)</sub>`.

Given the stage_title module explicitly has a “for prompt” display variant that omits *both* prompt/output links (and comments suggest prompt-serialization concerns), this seems unintended.

**Required fix:** When `for_prompt`, also set `title.output_link = None`.

---

## Non-blocking findings / suggestions
1) **DRY / repeated literals:** `"--- stderr ---"` separator is duplicated across three executor crates. If the project rule “avoid repeated string literals” is interpreted strictly, consider a shared const (or at least a per-crate const).

2) **Lossy output capture:** executors collect output via `BufReadExt::lines()`, which drops newline characters and may behave oddly with non-UTF8 / partial lines. Probably acceptable for logs, but worth noting.

3) **Error fidelity:** `execute_tool` reports non-zero exit as a generic error (`"Tool exited with non-zero status"`) without including exit code/status. Not required by the task, but would improve diagnostics.

---

## Analog consistency assessment
- The analog of storing prompt text via `store_report` and linking it in stage metadata is used correctly for output.
- Stage title parsing is kept backward-compatible similarly to existing context parsing patterns.
- Main deviation is the missing URL mapping for the new `output_link`, which breaks parity with the existing prompt_link behavior.

---

## Checklist status
- [x] Added output_link field to StageInfo and MdStageTitle
- [x] Updated MdStageTitle format (backtick timestamp + prompt/output sub-links)
- [x] Captured model output (stdout+stderr) and return it even on non-zero exit
- [ ] **Ensure output link renders as a correct URL in GitHub contexts** (currently missing)
- [ ] **Ensure for_prompt serialization omits output link**
