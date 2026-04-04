Commands run:
1) cargo fmt --all -- --check
2) cargo clippy --all-targets --all-features

Output (cargo fmt):
Diff in /data/home/skynet/zdam/zbobr-dev/workspaces/skynet/task-307/zbobr/zbobr/src/init.rs:1202:
         let new_content = tokio::fs::read_to_string(&new_file_path)
             .await
             .expect("Failed to read .new file");
-        assert_eq!(new_content, "new content", ".new file should contain new content");
+        assert_eq!(
+            new_content, "new content",
+            ".new file should contain new content"
+        );
     }
 
     #[tokio::test]
Diff in /data/home/skynet/zdam/zbobr-dev/workspaces/skynet/task-307/zbobr/zbobr/src/main.rs:85:
         return init::init_workspace(directory, force).await;
     }
 
-    let location =
-        zbobr_dispatcher::resolve_config_location(&cli.config_file.paths, "zbobr.toml")?;
+    let location = zbobr_dispatcher::resolve_config_location(&cli.config_file.paths, "zbobr.toml")?;
 
     let root_toml = {
         let mut merged: Option<RootConfigToml> = None;
... (additional rustfmt diffs omitted for brevity; full diffs available in command output)

Output (cargo clippy):
Compiling and checking workspace; finished `dev` profile [unoptimized + debuginfo] target(s) in 23.08s
(Note: cargo clippy produced warnings across multiple crates; see clippy_output.txt in repo for details.)

Summary of issues found:
- cargo fmt: repository is not formatted; rustfmt diffs reported (see detailed diffs above).
- cargo clippy: warnings present in multiple crates (zbobr-api, zbobr-dispatcher, zbobr-task-backend-fs, etc.).

Next steps:
- Run `cargo fmt --all` to apply formatting changes.
- Review clippy warnings in clippy_output.txt and decide which to fix or allow.

Full raw output captured during run is available in the session logs.