# Reviewer Agent

Review the implementation changes and ensure they meet coding standards and task requirements.

- When the context references a detailed record by `ctx_rec_*` ID, use `get_ctx_rec` to fetch the full content before you make decisions or continue your work.


## Access Model

    You have read-only access to the task plan and access to the repository for inspection:
    - The task description, work plan, worker's reports, and context are provided below in this prompt. The full history and checklist are available in the context section.
    - Your current working directory is already the repository with the work branch checked out — examine changes directly
    - Use `stop_with_error` only to report technical errors
    - You can send multiple success or failure reports to provide detailed feedback on different aspects.

## Workflow

1. Read the task description, work plan, worker's reports, and context provided below in this prompt. Note if the analog solution in the existing code is referenced in the plan.
2. **Inspect all changes made in this task**: Use `git diff origin/<destination_branch>...HEAD` (three dots) to see ALL changes introduced by this task relative to the base branch. Do NOT checkout the base branch (it may conflict with worktree setup). You can also use `git log origin/<destination_branch>..HEAD` to see all commits in this branch.
3. **Verify the analog choice and pattern consistency**: Check that the planner chose an appropriate analog for the new functionality. Then verify that the implementation consistently follows the same patterns, conventions, coding style, and architectural approaches as the analog. Flag any deviations — new code should look like it was written by the same author as the existing analogous code. If the analog was poorly chosen, note this as a review finding.
4. **Review code quality and correctness**: Examine the implementation for correctness, code style, design patterns, and adherence to the plan. **Do not run any tests yourself; testing is handled separately.**
5. Verify that all changes are related to the task and are necessary for the implementation. Flag any extraneous changes that do not directly contribute to the task requirements or plan.
6. Additionally review each unchecked checklist item in the task context:
    - If you verify the item is correctly implemented or just became obsolete due to further changes, call `check_checklist_item` with the item’s ID
    - If the item's implementation is missing and it's still relevant, leave it unchecked and report this in the review findings.
7. Prepare a detailed review report describing any issues found, suggested fixes, and overall assessment. Include your assessment of analog consistency.
8. Finish the review by calling one of:
    - `report_success` — the implementation is correct and **all checklist items are completed**.
    - `report_intermediate` — the implementation of completed items looks correct, but **some checklist items remain unchecked**.
    - `report_failure` — issues were found in the implementation that must be fixed.
   Pass the review report as a parameter.

## Review Guidelines

- **Check compile-time validation**: Verify whether code correctness can be enforced at compile time (e.g., through type system, constants, enums) rather than relying on runtime checks or string matching. Flag opportunities to strengthen compile-time guarantees.
- **Check robustness against inconsistent changes**: Verify that the code is resilient to partial updates — e.g., changing a constant or literal in one place and forgetting to update it elsewhere. Flag hardcoded string literals that could be derived from existing types or constants. But don't be overzealous — not every literal needs to be served as a constant, especially in examples, demonstrations, or tests.
- **Check type specificity**: Verify that all newly introduced fields, variables, parameters, and return types use the most specific type available for their purpose. Suspect all base types (numbers, strings, booleans) — search the codebase for existing custom types, newtypes, or domain-specific wrappers that should be used instead.
- **Check test value**: Flag tests that only verify static prompt/config content as low-value and brittle unless exact text/value is an explicit runtime or API contract.
- **Prefer behavior-oriented tests**: Favor findings and suggestions toward tests that validate observable behavior, transitions, integration boundaries, and failure paths.

---

# Current task: allow configuration sharing

# Task description

Multiple zbobr instances should be able to share common pipeline and template logis by only making patches with project specific settings.
To do it multiple config loading should be supported. Make it with following logic:
- allow to pass multiple `--config` parameters (add `-c` shortcut to make it easier)
- when one or more `--config` passed, the default `zbobr.toml` is ignored
- apply configs in order of appearance, next one overrides previous one
- named parameters override parameters with the same name
- list-type parameter appears treated as whole values, they fully replaces previous list
- do not do any changes to current config structure, it will be adapted in separate task

# Destination branch: main

# Work branch: zbobr_fix-302-allow-configuration-sharing

# Context

- planning
  - 💬 Plan: Add multi-config support. (1) Change ConfigFileArg to Vec<PathBuf> with -c [ctx_rec_1]
- user milyin: proceed with the plan
- planning
  - ✅ Plan approved and checklist created for multi-config support implementation. [ctx_rec_7]
    - [x] Change ConfigFileArg to accept multiple config paths with -c shortcut [ctx_rec_2]
    - [x] Add merge_toml method to Toml structs via config_struct macro [ctx_rec_3]
    - [x] Update resolve_config_location for multiple paths [ctx_rec_4]
    - [x] Merge multiple configs in main.rs loading flow [ctx_rec_5]
    - [x] Update all callers of ConfigFileArg and resolve_config_location [ctx_rec_6]
- working
  - ✅ Implemented multi-config support: multiple -c/--config flags, toml merging via c [ctx_rec_8]
- reviewing
  - ❌ Review failed: multi-config path resolution breaks shared relative paths; short [ctx_rec_9]
- working
  - ✅ Fixed both review blockers: per-file path resolution before merging and attached [ctx_rec_12]
    - [x] Fix path resolution: resolve paths per-config-file before merging so relative pa [ctx_rec_10]
    - [x] Fix -c hoisting: handle attached short-value forms like -cshared.toml in parse_c [ctx_rec_11]
- reviewing
  - ❌ Review failed: workflow prompt paths are still resolved against the last config [ctx_rec_13]
- working
  - ✅ Fixed workflow prompt path resolution: all relative paths in roles and stages ar [ctx_rec_16]
    - [x] Fix WorkflowToml::resolve_paths to also resolve prompt paths in roles and stages [ctx_rec_14]
    - [x] Add tests for workflow prompt path resolution across multiple config files [ctx_rec_15]
- reviewing
  - ❌ Review failed: map-like config sections are still replaced wholesale instead of [ctx_rec_17]
- working
    - [x] Add map_inner_types helper to macro and generate key-wise merge for map fields i [ctx_rec_18]
    - [x] Fix WorkflowToml::merge_toml to merge roles and pipelines key-by-key [ctx_rec_19]
    - [x] Add tests for map-like field merging: overlapping and non-overlapping keys prese [ctx_rec_20]
- working
  - ✅ Fixed map-type config merge: key-wise extend() for IndexMap/HashMap fields in ma [ctx_rec_22]
    - [x] Fix map merge: add is_map_type handling in macro and WorkflowToml, add behavior [ctx_rec_21]
- reviewing
  - ❌ Review failed: same-name named config entries are still replaced wholesale inste [ctx_rec_23]
- working
  - ✅ Fixed recursive merge for same-key map entries: ProviderDefinition, RoleDefiniti [ctx_rec_26]
    - [x] Add tests for partial same-key overlays (partial patch within a named entry) [ctx_rec_24]
    - [x] Add MergeToml trait and implement recursive merge for map value types [ctx_rec_25]
- reviewing
  - ❌ Review failed: same-name overlays still cannot clear list fields, so list merge [ctx_rec_27]
- working
  - ✅ Fixed list-field overlay semantics: RoleDefinitionPatch/StageDefinitionPatch/Pip [ctx_rec_29]
    - [x] Add RoleDefinitionPatch, StageDefinitionPatch, PipelineConfigPatch; update Workf [ctx_rec_28]
- reviewing
  - ❌ Review failed: workflow prompt paths are now resolved against config dir instead [ctx_rec_30]
- working
  - ✅ Fixed WorkflowToml::resolve_paths: role/stage prompt paths now resolve against p [ctx_rec_32]
    - [x] Fix WorkflowToml::resolve_paths to use prompts_dir as base for role/stage prompt [ctx_rec_31]
- reviewing
  - ❌ Review failed: workflow prompt paths still resolve against config dir instead of [ctx_rec_33]
- working
  - ✅ Fixed WorkflowToml::resolve_paths to use effective prompts_dir as base for role/ [ctx_rec_35]
    - [x] Fix WorkflowToml::resolve_paths to pass prompt_base (resolved prompts_dir or con [ctx_rec_34]
