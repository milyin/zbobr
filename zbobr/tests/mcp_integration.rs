use std::path::Path;
use tempfile::TempDir;

/// Recursively copy a directory tree.
fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dest_path)?;
        } else {
            std::fs::copy(entry.path(), &dest_path)?;
        }
    }
    Ok(())
}

async fn run_mcp_test(command: &str, task_id: u64) {
    // Check that mcp-tester is installed; skip gracefully if not
    let mcp_check = tokio::process::Command::new("mcp-tester")
        .arg("--version")
        .output()
        .await;
    if mcp_check.is_err() || !mcp_check.unwrap().status.success() {
        eprintln!("Skipping test: mcp-tester not installed (cargo install mcp-tester)");
        return;
    }

    // Copy static fixture to a temp directory
    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mcp_dummy");
    let tmp = TempDir::new().expect("failed to create temp dir");
    copy_dir_all(&fixture_dir, tmp.path()).expect("failed to copy fixture");

    let config_path = tmp.path().join("zbobr.toml");
    let zbobr_bin = env!("CARGO_BIN_EXE_zbobr");

    // Run: zbobr <command> <task_id> --config <temp>/zbobr.toml
    let rust_log = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let status = tokio::process::Command::new(zbobr_bin)
        .args([
            command,
            &task_id.to_string(),
            "--config",
            config_path.to_str().unwrap(),
        ])
        .current_dir(tmp.path())
        .env("RUST_LOG", &rust_log)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .await
        .expect("failed to run zbobr binary");

    assert!(
        status.success(),
        "zbobr {} failed with exit code {:?}",
        command,
        status.code(),
    );
}

#[tokio::test]
async fn preparator_get_description_via_mcp_tester() {
    run_mcp_test("prepare", 1).await;
}

#[tokio::test]
async fn planner_get_description_via_mcp_tester() {
    run_mcp_test("plan", 1).await;
}

#[tokio::test]
async fn worker_get_description_via_mcp_tester() {
    run_mcp_test("work", 1).await;
}

#[tokio::test]
async fn reviewer_get_description_via_mcp_tester() {
    run_mcp_test("review", 1).await;
}

#[tokio::test]
async fn merger_get_description_via_mcp_tester() {
    run_mcp_test("merge", 1).await;
}
