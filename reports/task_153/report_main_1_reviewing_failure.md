# Review Report: Task 153

## Assessment
The implementation partially meets the requirements but fails on the display requirement.

## Requirements Check
1.  **Display stage counter in parameters:** FAILED.
    *   The requirement "make sure that the task's stage counter is displayed in the parameters, similarly to the pipeline counter" is not implemented.
    *   The `stage_count` field was added to the `Task` struct.
    *   However, it is NOT included in `zbobr-dispatcher/src/cli.rs`'s `print_task` function (which displays task details to CLI users).
    *   It is NOT included in `zbobr-dispatcher/src/prompts.rs`'s `build_template_variables` (which provides variables to the LLM prompt).
    *   It is NOT included in `zbobr-api/src/checklist_format.rs` or `zbobr-dispatcher/src/cli.rs`'s `CommentTag` (which displays the `[pipeline#run_id]` tag).
2.  **Back-counted stage counter:** PASSED.
    *   `task_stage_limit` config added.
    *   `stage_count` initialized from config for fresh tasks in `cli.rs`.
    *   `decrement_stage_count` implemented in `task.rs` using saturating subtraction.
3.  **Pause on zero:** PASSED.
    *   Logic in `cli.rs` correctly checks if `new_count == 0` and limit is set, then pauses.

## Code Quality
*   The logic for initialization and decrementing seems sound and follows the pattern of `pipeline_run_id`.
*   Code style is consistent.

## Findings
*   **Missing Display Implementation**: The `stage_count` is tracked internally but effectively invisible to both the user (CLI) and the agent (Prompt). To satisfy "displayed in the parameters, similarly to the pipeline counter", it should likely be added to:
    *   `print_task` in `zbobr-dispatcher/src/cli.rs`.
    *   Prompt variables in `zbobr-dispatcher/src/prompts.rs` (e.g. `VAR_STAGE_COUNT`).

## Recommendation
Implement the display logic for `stage_count` in the CLI task view and optionally in the prompt context if "parameters" referred to prompt variables.
