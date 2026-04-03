In zbobr-dispatcher/src/prompts.rs, the sample_task_and_comments function uses "claude".to_string() instead of Tool::CLAUDE.to_string(), and repeats "https://github.com/example/repo/issues/1" multiple times. Fix by:
1. Adding Tool to imports from zbobr_api::task
2. Replace "claude".to_string() with Tool::CLAUDE.to_string()
3. Add a local const SAMPLE_ISSUE_URL = "https://github.com/example/repo/issues/1" and use it in the comment URLs and report_link