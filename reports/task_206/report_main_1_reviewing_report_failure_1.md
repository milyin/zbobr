Review scope
- Base: origin/main
- Branch: zbobr_fix-206-make-error-up-to-date
- Diff: only zbobr-dispatcher/src/task.rs (+8 lines)

Task requirements coverage
1) Prefix error with red ❌ + timestamp
- Implemented in RoleSession::set_error by mapping Option<String> and prepending "❌ <YYYY-MM-DD HH:MM:SS> <+ZZZZ> <msg>".
- Timestamp source uses chrono::Utc::now().with_timezone(&self.zbobr.config().fixed_offset()), consistent with existing timestamp creation in zbobr-dispatcher/src/cli.rs (StageContext timestamp).

2) Clear error when task enters Running
- Implemented in TaskSession::set_state:
  if task.state != state && state.is_running() { task.error = None; }
- This clears only on transitions into Running (including pipeline/stage changes), addressing the previously-reported “too aggressive” clearing.

Issues (must fix)
1) Unit test now incompatible (will fail)
- zbobr-dispatcher/src/task.rs test mcp_helper_includes_explicit_model asserts:
  assert_eq!(task.error.as_deref(), Some("oops"));
- After this change, task.error will be prefixed with ❌ + timestamp, so it will never equal "oops".
- Suggested fix: change the assertion to be format-tolerant, e.g.
  - assert!(task.error.as_deref().unwrap_or("").contains("oops"));
  - and optionally assert it starts with "❌ " and has a timestamp-ish substring.
  Avoid exact timestamp comparisons since it’s time-dependent.

Design/robustness notes (recommended)
- Formatting is applied only in zbobr-dispatcher RoleSession::set_error. The generic backend implementation (zbobr-api/src/backend.rs set_error) still stores raw strings. If any code path uses that backend API to set task.error, it will bypass the new ❌+timestamp requirement.
  Consider ensuring all writers go through the formatting function, or moving formatting closer to rendering/serialization (GitHub description error section), depending on intended architecture.

Analog / consistency assessment
- Timestamp acquisition matches the established pattern using dispatcher config fixed_offset (see zbobr-dispatcher/src/cli.rs:422).
- The serializer for the GitHub issue body (zbobr-task-backend-github/src/separator.rs) appends error text verbatim; the new prefixing should render correctly.

No extraneous changes detected
- Only one source file changed, and changes are directly related to the task requirements.
