Overall assessment: the implementation follows the plan well. Moving the helper into `zbobr-dispatcher`, renaming it to `sample_task_and_comments`, enriching the sample task/comment data, exporting it, and reusing it from `validate_all_prompts` and CLI placeholder paths is the right analog and is consistently applied. I did not find unrelated changes.

However, I found one issue that should be fixed before accepting the branch:

1. **Hardcoded canonical tool name / repeated sample literals reduce consistency and drift safety**
   - Location: `zbobr-dispatcher/src/prompts.rs:128-190`
   - The new helper introduces canonical string values directly, most notably `tool: Some("claude".to_string())`, even though the codebase already defines a canonical tool constant/newtype in `zbobr_api::task::Tool` (`Tool::CLAUDE`, plus constructors).
   - This is exactly the kind of inconsistency the project rule warns about: if a canonical spelling changes in one place and not another, the sample helper silently drifts. The same helper also repeats the same example GitHub URL prefix multiple times instead of deriving it from a shared constant.
   - Why this matters here: this helper was added specifically to improve prompt validation coverage. Because it is now a shared sample-data source used across validation and CLI placeholder rendering, it becomes a long-lived reference point. Keeping canonical values centralized is more important here than in one-off test scaffolding.
   - Suggested fix:
     - Replace the hardcoded tool literal with the existing canonical constant, e.g. `tool: Some(zbobr_api::Tool::CLAUDE.to_string())` (or equivalent imported alias).
     - Factor repeated sample URL pieces into local `const` values inside `sample_task_and_comments()` or nearby module-level constants, so the PR URL and issue-comment URLs are derived from a single base string.

Analog consistency assessment: good overall. The planner chose the right analog (existing dummy/sample task construction around prompt rendering), and the implementation preserves that structure. The only notable deviation is the introduction of new hardcoded canonical literals rather than reusing existing canonical representations.

Checklist assessment: the listed implementation items appear completed; no additional unchecked checklist items were present in the provided context.