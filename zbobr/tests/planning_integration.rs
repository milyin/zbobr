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

#[tokio::test]
async fn planning_get_description_via_mcp_tester() {
    // Check that mcp-tester is installed; skip gracefully if not
    let mcp_check = tokio::process::Command::new("mcp-tester")
        .arg("--version")
        .output()
        .await;
    if mcp_check.is_err() || !mcp_check.unwrap().status.success() {
        eprintln!("Skipping test: mcp-tester not installed (cargo install mcp-tester)");
        return;
    }

    // Copy static fixture to a temp directory (zbobr modifies the environment)
    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/planning_get_description");
    let tmp = TempDir::new().expect("failed to create temp dir");
    copy_dir_all(&fixture_dir, tmp.path()).expect("failed to copy fixture");

    let config_path = tmp.path().join("zbobr.toml");
    let zbobr_bin = env!("CARGO_BIN_EXE_zbobr");

    // Run: zbobr plan 1 --config <temp>/zbobr.toml
    let output = tokio::process::Command::new(zbobr_bin)
        .args(["plan", "1", "--config", config_path.to_str().unwrap()])
        .current_dir(tmp.path())
        .env("RUST_LOG", "info")
        .output()
        .await
        .expect("failed to run zbobr binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "zbobr plan failed with exit code {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        stdout,
        stderr,
    );
}
