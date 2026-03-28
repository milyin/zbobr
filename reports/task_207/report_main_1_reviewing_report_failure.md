## Review: capture model output + output link in stage title

### Scope / diff inspected
Branch `zbobr_fix-207-capture-model-output` vs `origin/main` (commit `55d50e4`). Touched:
- `zbobr-api`: `StageInfo`/`MdStageTitle` extended with `output_link`; stage title formatting/parsing updated.
- `zbobr-dispatcher`: stage runner stores output report post-exec and sets `output_link`.
- Executors (`claude`, `copilot`, `mcp-tester`): `ToolExecutor::execute` now returns captured output.

### What looks good
- **Stage title format** matches requested shape: timestamp is now a backtick token and prompt/output are separate `<sub>` links.
- **Backwards compatibility**: parsing still accepts old `<sub>` timestamp formats.
- **Data model wiring**: `StageInfo` ↔ `MdStageTitle` conversions include `output_link`; serde attributes mirror `prompt_link`.
- **Prompt/output storage locality**: both are stored via the same `store_report()` mechanism, so they end up in the same task report directory ("next to" each other), and links are consistent with the existing prompt approach.

### Blocking issues (must fix)
1) **“All output” is not captured**
   - `ClaudeExecutor` / `CopilotExecutor` return **stdout only** (`stdout_result.join("\n")`). Stderr is collected but only used to enrich the error message.
   - Task requirement says: *“Collect all the output from the model”*. In practice, many CLIs stream important content/progress to stderr; even if not today, this is a fragile assumption.
   **Fix:** return a combined transcript (stdout + stderr) or at least store both streams in the report (e.g., prefix sections `=== stdout ===`, `=== stderr ===`).

2) **No output report/link on failure or interrupt**
   - In `execute_tool()`, `execution_output` is set only on `Ok(output)`; on `Err(e)` and on Ctrl-C path it is `None`.
   - As a result, **the stage will not get an output report** exactly when the output is most valuable for debugging.
   **Fix:** propagate whatever was captured even when the process exits non-zero or is interrupted. This likely requires changing the executor contract to return output alongside status (e.g., a struct `{ output, status }`) or ensuring `execute_tool()` stores output in `Err` paths.

### Coding standards / maintainability findings
3) **Repeated string literals: `"prompt"` / `"output"`**
   - `zbobr-api/src/context/stage_title.rs` hardcodes these labels in multiple places (Display + parser + tests).
   - Project rule explicitly says to avoid repeated string literals.
   **Fix:** introduce constants, e.g. `const LINK_LABEL_PROMPT: &str = "prompt"; const LINK_LABEL_OUTPUT: &str = "output";` and use them consistently.

### Minor / non-blocking observations
- Consider skipping storing an output report when the collected output is empty/whitespace to reduce noise, unless an empty report is desired for consistency.
- Timestamp detection in backticks relies on “contains spaces” + chrono parse. It’s likely fine given current `Tool`/`Model` formats, but if labels ever include spaces, parsing will get ambiguous. A helper function or explicit ordering would make this more robust.

### Analog / pattern consistency
- The change follows the existing **prompt report** pattern closely (store report → set link on `StageInfo` → render via `MdStageTitle`), which is the right analog.
- The new output behavior is integrated in the same place in the stage runner, which is consistent.

### Overall assessment
Core plumbing is in place and the title format change is good, but the implementation does **not yet meet** the requirement to capture **all output**, and it fails to store/link output in error cases. Address the blocking items above before merging.