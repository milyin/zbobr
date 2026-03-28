Plan is set. Here's the summary:

**Approach**: Add `allowed_usernames: Option<Vec<String>>` directly to `ZbobrTaskBackendGithubConfig` (not the dispatcher config). The `TaskBackend::list_tasks` trait stays unchanged. Filtering happens client-side in `list_tasks` by matching `issue.user.login` against the configured list.

**Why this differs from the reverted PR #230**: That PR modified the trait signature and put the filter in dispatcher config — too broad. This keeps it self-contained in the GitHub backend.

**Files affected**: `config.rs` (new field), `github.rs` (deserialize user field + filter logic), `init.rs` (default template).