# Tests: FS Backend Comment Timestamp Timezone Conversion

**File**: `zbobr-task-backend-fs/src/fs.rs`, new `#[cfg(test)] mod tests` block

## Rationale
The `read_comments_structured` function has new conditional logic: when `config.timezone` is `Some(tz)`, it applies `with_timezone` to every comment. When `None`, it returns timestamps unchanged. This branch has zero test coverage. The FS backend has no unit test module at all.

## Approach
Use `tempfile::tempdir()` (already a test dependency or add it), write a YAML `comments.yaml` fixture with a UTC timestamp, then call `read_comments_structured` and assert the returned timestamp's timezone offset.

## Test cases to add

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use zbobr_api::task::FixedOffsetTz;

    fn make_config_with_timezone(tasks_dir: PathBuf, tz: Option<FixedOffsetTz>) -> ZbobrTaskBackendFsConfig {
        ZbobrTaskBackendFsConfig {
            tasks_dir,
            timezone: tz,
            // ... other fields with defaults
        }
    }

    async fn write_comment_fixture(dir: &Path, task_id: u64, timestamp_str: &str) {
        // Write a minimal comments.yaml with one comment having the given timestamp
        let yaml = format!(
            "comments:\n  - timestamp: \"{}\"\n    author: user\n    body: hello\n",
            timestamp_str
        );
        let path = dir.join(format!("task_{}/comments.yaml", task_id));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, yaml).unwrap();
    }

    #[tokio::test]
    async fn read_comments_converts_to_configured_timezone() {
        let dir = tempfile::tempdir().unwrap();
        write_comment_fixture(dir.path(), 1, "2025-01-01T12:00:00+00:00").await;

        let tz: FixedOffsetTz = "+03:00".parse().unwrap();
        let config = make_config_with_timezone(dir.path().to_path_buf(), Some(tz));
        let backend = ZbobrTaskBackendFs::new(config);
        let comments = backend.read_comments_structured(1).await.unwrap();

        assert_eq!(comments.len(), 1);
        // The timestamp should be at UTC+3 = 15:00
        assert_eq!(comments[0].timestamp.offset().local_minus_utc(), 3 * 3600);
        assert_eq!(comments[0].timestamp.hour(), 15);
    }

    #[tokio::test]
    async fn read_comments_unchanged_when_no_timezone() {
        let dir = tempfile::tempdir().unwrap();
        write_comment_fixture(dir.path(), 1, "2025-01-01T12:00:00+00:00").await;

        let config = make_config_with_timezone(dir.path().to_path_buf(), None);
        let backend = ZbobrTaskBackendFs::new(config);
        let comments = backend.read_comments_structured(1).await.unwrap();

        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].timestamp.offset().local_minus_utc(), 0);
    }
}
```

Ensure `tempfile` is listed as a `[dev-dependencies]` in `zbobr-task-backend-fs/Cargo.toml` if not already present.
