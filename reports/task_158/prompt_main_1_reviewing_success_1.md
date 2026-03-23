# Reviewer Agent

Review the implementation changes and ensure they meet coding standards and task requirements.

## Access Model

    You have read-only access to the task plan and access to the repository for inspection:
    - The task description, work plan, worker's report, comments, and checklist are provided below in this prompt. Use `get_history` to read the full discussion history if needed for more context.
    - Your current working directory is already the repository with the work branch checked out — examine changes directly
    - Use `stop_with_error` only to report technical errors

## Workflow

1. Read the task description, work plan, worker's report, comments, and checklist provided below in this prompt. Note if the analog solution in the existing code is referenced in the plan.
2. **Inspect all changes made in this task**: Use `git diff origin/<destination_branch>...HEAD` (three dots) to see ALL changes introduced by this task relative to the base branch. Do NOT checkout the base branch (it may conflict with worktree setup). You can also use `git log origin/<destination_branch>..HEAD` to see all commits in this branch.
3. **Verify the analog choice and pattern consistency**: Check that the planner chose an appropriate analog for the new functionality. Then verify that the implementation consistently follows the same patterns, conventions, coding style, and architectural approaches as the analog. Flag any deviations — new code should look like it was written by the same author as the existing analogous code. If the analog was poorly chosen, note this as a review finding.
4. **Review code quality and correctness**: Examine the implementation for correctness, code style, design patterns, and adherence to the plan. **Do not run any tests yourself; testing is handled in a separate Testing stage.**
5. Verify that all changes are related to the task and are necessary for the implementation. Flag any extraneous changes that do not directly contribute to the task requirements or plan.
6. Prepare a detailed review report describing any issues found, suggested fixes, and overall assessment. Include your assessment of analog consistency.
7. Call `report_success` if the implementation is correct and complete, or `report_failure` if issues were found. Pass the review report as a parameter to these tools.

---

# Current task: replace milestones to labels

# Task description

github backend uses milestones to store the task state (see `State` enum).
But the user may use milestones for it's own purposes.
Better to use the labels for this, as well as for the signals and flags.
Store the state now as labels.
Represent the state as 3 labels:
`state:{done|pause|ready|pending|running}`
`pipeline:{name}`
`stage:{name}`

The labels converted to `State` enum by these rules:
correct cases:
- `state:done` -> `State::Done`
- `state:pause` -> `State::Pause`
- `state:ready` -> `State::Ready`
- `state:pending`, `pipeline:main` -> `State::Pending(Pipeline::Main)`
- `state:pending`, `pipeline:foo` -> `State::Pending(Pipeline::Custom("Foo"))`
- `state:running`, `pipeline:main`, `stage:bar` -> `State::Running(Pipeline::Main, Stage("bar"))`
incorrect cases:
- `state:running`, `stage:bar` -> `State:Unknown("state:running, stage:bar")`

make `state:done` green, `state:ready` blue, `state:pause` yellow. Make other states less vivid, pending is gray, running is lignth green

# Destination branch: main

# Work branch: zbobr_fix-158-replace-milestones-to-labels

# Last report

All 4 checklist items completed. Eliminated all hardcoded label literals in github.rs by introducing STATE_PREFIX/PIPELINE_PREFIX/STAGE_PREFIX constants and using State::LABEL_*/Pipeline::MAIN/MERGE constants. Rewrote state_to_labels(), labels_to_state(), state_label_color(), apply_state_change(), and setup() to derive labels programmatically. Added coding guidelines to worker/reviewer prompts about type-derivation and compile-time validation. cargo check and cargo test pass clean.

[report_main_1_working_success_3.md](https://github.com/milyin/zbobr/blob/reports/reports/task_158/report_main_1_working_success_3.md)

# Last request

1. Do not use literals like you did here:
```

        // Create state labels
        let state_labels = [
            "state:done",
            "state:pause",
            "state:ready",
            "state:pending",
            "state:running",
            "pipeline:main",
            "pipeline:merge",
        ];
```
All these text labels can be and must be derived from types. 

2. Sidetask: update worker and reviewer default prompts in `zbobr/src/init.rs` with recommendation to avoid literals if the value can be derived from the type. Add to reviewer special recommendation to check is the code correctness can be compile-time validated. Is the code robust enough against inconsistent changes, like changing constant or literal in one place and forgetting in another.

# Unchecked checklist items

