## Scope / diff inspected
Compared `origin/main...HEAD` on branch `zbobr_fix-207-capture-model-output`.

Changed files (11):
- `zbobr-api/src/task.rs` (StageInfo)
- `zbobr-api/src/context/stage_title.rs` (format + parse)
- `zbobr-api/src/context/mod.rs` (URL mapping + prompt-mode omission)
- `zbobr-api/src/tool_executor.rs` (ExecutorOutput + trait signature)
- `zbobr-dispatcher/src/cli.rs` (store output report + set output_link)
- `zbobr-executor-{claude,copilot,mcp-tester}/src/lib.rs` (capture stdout+stderr)
- plus small updates in `zbobr-api/src/lib.rs`, `zbobr-dispatcher/src/task.rs`, `zbobr-task-backend-github/src/separator.rs` for new field.

## Requirements coverage
### 1) Collect all model output and store in a file
- Executors now collect **stdout and stderr** lines and return combined output via `ExecutorOutput { output, exit_ok }`.
- Output is returned even on non-zero exit (`exit_ok: false`), enabling storage on failures.
- Dispatcher stores the captured output via `role_session.store_report("output_<pipeline>_<run>_<stage>_end", output)`.

✅ Meets the “collect output” requirement (with the note below about stdout/stderr interleaving).

### 2) Add output link to the stage title
- Added `StageInfo.output_link: Option<String>`.
- `MdStageTitle` now renders:
  `pipeline:run:**stage** `tool` `model` `timestamp` <sub>[prompt](...)</sub> <sub>[output](...)</sub>`
  with prompt/output sub-links optional.
- Output link is set on the last stage after execution.

✅ Meets stage-title format requirement.

### 3) URL mapping and prompt-mode behavior
- `serialize_context(..., report_url)` now maps both `prompt_link` and `output_link` using the provided `report_url` closure when they are relative.
- When `for_prompt == true`, both links are removed.

✅ Fixes the previously reported “broken URL mapping” and “output link in prompt context” issues.

## Analog / pattern consistency
Analog: existing prompt capture + `prompt_link` plumbing.

Implementation matches the analog well:
- Same store mechanism (`store_report`) and same late `modify_task` pattern.
- Same Option<String> domain type for links.
- Stage-title handling mirrors existing Markdown serialization patterns and keeps backwards-compat parsing.

## Code quality / correctness notes
### ✅ Strong points
- Backwards compatibility: old `<sub>` timestamp formats are still parsed.
- The new “labels” (`prompt`, `output`) are constants, avoiding repeated string literals in the stage title renderer/parser.
- Execution API change (`ToolExecutor::execute -> Result<ExecutorOutput>`) cleanly separates:
  - I/O-level failures (`Err`)
  - process exit failure (`Ok(... exit_ok=false)`)
  enabling output storage even on failures.

### ⚠️ Minor improvement opportunities (non-blocking)
1) **Ordering fidelity**: output is collected separately for stdout/stderr and concatenated, which loses interleaving. If “exact transcript” is important, consider a merged stream or timestamped line capture.
2) **Repeated stderr separator literal**: `"--- stderr ---"` is duplicated across 3 executors. Consider a shared `const` (in `zbobr-api` or a small helper) to satisfy the project-wide “avoid repeated literals” rule more broadly.
3) **Documentation drift**: `zbobr-api/src/context/mod.rs`’s `MdStage` doc comment still shows an older timestamp/<sub> structure. Not functional, but could confuse future maintainers.

## Extraneous changes
No unrelated functional changes spotted beyond necessary field propagation and test updates.

## Overall assessment
Meets task requirements and follows existing patterns closely. Changes look coherent across layers (executor → dispatcher → context serialization → GitHub URL mapping). Recommended to merge after normal CI/compilation checks.