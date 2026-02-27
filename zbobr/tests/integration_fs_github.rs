/// Integration tests: filesystem task backend + GitHub repo backend.
///
/// Individual test functions are marked `#[ignore]` by default; run them
/// explicitly after adding your credentials using
/// `cargo test --test integration_fs_github -- --ignored`.
///
/// Activated when `zbobr_github_test.toml` at the workspace root contains a
/// `[repo.github]` section with valid credentials.
/// Run this group with: `cargo test --test integration_fs_github`
/// or filter by prefix: `cargo test test_fs_github_`
mod mcp_integration;

use std::sync::Arc;
use tokio::sync::OnceCell;

use mcp_integration::IntegrationTestEnv;
use mcp_integration::env::{RepoBackendArgs, TaskBackendArgs};
use mcp_integration::github_config::GitHubTestConfig;
use mcp_integration::{scenarios, test_helpers};
use zbobr_dispatcher::Stage;

// panicking version: missing configuration is considered an error
static ENV: OnceCell<Arc<IntegrationTestEnv>> = OnceCell::const_new();
static TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn get_env() -> Arc<IntegrationTestEnv> {
    ENV.get_or_init(|| async {
        let cfg = GitHubTestConfig::load()
            .expect("zbobr_github_test.toml not found; required for GitHub tests");
        let repo = cfg.repo
            .expect("[repo.github] section missing in zbobr_github_test.toml");

        let base = match std::env::var("CARGO_TARGET_TMPDIR") {
            Ok(p) => std::path::PathBuf::from(p).join("integration_env_fs_github"),
            Err(_) => std::env::temp_dir().join("zbobr_integration_env_fs_github"),
        };
        let tasks_dir = base.join("tasks");

        let target_repo = cfg.tasks.as_ref().map(|t| t.github.task_repo.clone());
        IntegrationTestEnv::init(
            "fs_github",
            TaskBackendArgs::Filesystem { tasks_dir },
            RepoBackendArgs::GitHub {
                fork_owner: repo.github.fork_owner,
                repo_token: repo.github.token,
            },
            cfg.dispatcher.agent_token,
            target_repo,
        )
        .await
        .expect("failed to initialize FS/GitHub environment; check credentials")
    })
    .await
    .clone()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_fs_github_preparation() {
    let _guard = TEST_LOCK.lock().await;
    let env = get_env().await;
    test_helpers::run_preparation(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_fs_github_planning() {
    let _guard = TEST_LOCK.lock().await;
    let env = get_env().await;
    test_helpers::run_planning(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_fs_github_working() {
    let _guard = TEST_LOCK.lock().await;
    let env = get_env().await;
    test_helpers::run_working(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_fs_github_reviewing() {
    let _guard = TEST_LOCK.lock().await;
    let env = get_env().await;
    test_helpers::run_reviewing(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_fs_github_merging() {
    let _guard = TEST_LOCK.lock().await;
    let env = get_env().await;
    test_helpers::run_merging(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_fs_github_merging_with_real_conflict() {
    let _guard = TEST_LOCK.lock().await;
    let env = get_env().await;
    test_helpers::run_merging_with_real_conflict(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_fs_github_conflict_detection() {
    let _guard = TEST_LOCK.lock().await;
    let env = get_env().await;
    test_helpers::run_conflict_detection(&env).await;
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_fs_github_reviewing_approval() {
    let _guard = TEST_LOCK.lock().await;
    let env = get_env().await;
    test_helpers::run_reviewing_approval(&env).await;
}

// ---------------------------------------------------------------------------
// GitHub repo backend tests — exercise clone_and_setup() with a real GitHub
// repository.  These require env.target_repo to be set (populated from
// [tasks.github].task_repo in zbobr_github_test.toml).  Since fork_owner and
// the target repo owner are the same organisation, same-org mode is used: no
// fork is created and there is no "fork" remote in the workspace.
// ---------------------------------------------------------------------------

/// A well-known public repository owned by a different organisation than any
/// typical test user.  Used by the cross-org tests below to exercise the fork
/// creation path (fork_owner != target repo owner).
const CROSS_ORG_DEST_REPO: &str = "octocat/Spoon-Knife";

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml with [tasks.github]"]
async fn test_fs_github_repo_backend_clone() {
    let _guard = TEST_LOCK.lock().await;
    let env = get_env().await;

    let target = match env.target_repo.as_deref() {
        Some(t) => t,
        None => {
            eprintln!("[{}] Skipping: target_repo not configured (add [tasks.github] to zbobr_github_test.toml)", env.name());
            return;
        }
    };

    let task_id = env
        .create_task("Clone test", "Test clone_and_setup via repo backend", Stage::Pending)
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-clone-test");
    env.update_task_branches(task_id, target, "main", &work_branch).await;

    // Clone via the GitHub repo backend (exercises clone_and_setup)
    env.run_zbobr("task", &["clone", &task_id.to_string()]).await;

    let repo_name = target.rsplit('/').next().unwrap_or(target);
    let workspace_dir = env.workspaces_dir.join(format!("task#{task_id}")).join(repo_name);

    assert!(
        workspace_dir.exists(),
        "[{}] Workspace directory missing after clone: {}",
        env.name(),
        workspace_dir.display()
    );
    assert!(
        workspace_dir.join(".git").exists(),
        "[{}] Workspace is not a git repository",
        env.name()
    );

    // Verify origin remote points to the target repo
    let origin_url = tokio::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(&workspace_dir)
        .output()
        .await
        .expect("failed to run git remote get-url origin");
    assert!(
        origin_url.status.success(),
        "[{}] origin remote not found in workspace",
        env.name()
    );
    let origin = String::from_utf8_lossy(&origin_url.stdout);
    assert!(
        origin.contains(repo_name),
        "[{}] origin remote '{}' does not contain repo name '{}'",
        env.name(),
        origin.trim(),
        repo_name
    );

    // Verify there is NO fork remote (same-org mode)
    let fork_check = tokio::process::Command::new("git")
        .args(["remote", "get-url", "fork"])
        .current_dir(&workspace_dir)
        .output()
        .await
        .expect("failed to run git remote get-url fork");
    assert!(
        !fork_check.status.success(),
        "[{}] Unexpected fork remote found in same-org workspace",
        env.name()
    );
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml with [tasks.github]"]
async fn test_fs_github_repo_backend_planning() {
    let _guard = TEST_LOCK.lock().await;
    let env = get_env().await;

    let target = match env.target_repo.as_deref() {
        Some(t) => t,
        None => {
            eprintln!("[{}] Skipping: target_repo not configured", env.name());
            return;
        }
    };

    let task_id = env
        .create_task("Repo backend planning", "Dummy task description", Stage::Pending)
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-plan-test");
    env.update_task_branches(task_id, target, "main", &work_branch).await;
    env.prepare_workspace_via_repo_backend(task_id, target, &work_branch).await;

    env.run_stage(task_id, Stage::Planning, scenarios::planning_scenario()).await;

    let output = env.show_task(task_id).await;
    assert!(
        output.contains("Signal:      go_work"),
        "[{}] Planner should emit go_work after posting plan",
        env.name()
    );
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml with [tasks.github]"]
async fn test_fs_github_repo_backend_working() {
    let _guard = TEST_LOCK.lock().await;
    let env = get_env().await;

    let target = match env.target_repo.as_deref() {
        Some(t) => t,
        None => {
            eprintln!("[{}] Skipping: target_repo not configured", env.name());
            return;
        }
    };

    let task_id = env
        .create_task("Repo backend working", "Dummy task description", Stage::Working)
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-work-test");
    env.update_task_branches(task_id, target, "main", &work_branch).await;
    env.prepare_workspace_via_repo_backend(task_id, target, &work_branch).await;

    env.run_stage(task_id, Stage::Working, scenarios::working_scenario()).await;

    let output = env.show_task(task_id).await;
    assert!(
        output.contains("Signal:      go_review"),
        "[{}] Worker should emit go_review",
        env.name()
    );
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml with [tasks.github]"]
async fn test_fs_github_repo_backend_reviewing() {
    let _guard = TEST_LOCK.lock().await;
    let env = get_env().await;

    let target = match env.target_repo.as_deref() {
        Some(t) => t,
        None => {
            eprintln!("[{}] Skipping: target_repo not configured", env.name());
            return;
        }
    };

    let task_id = env
        .create_task("Repo backend reviewing", "Dummy task description", Stage::Reviewing)
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-review-test");
    env.update_task_branches(task_id, target, "main", &work_branch).await;
    env.prepare_workspace_via_repo_backend(task_id, target, &work_branch).await;

    env.run_stage(task_id, Stage::Reviewing, scenarios::reviewing_scenario()).await;

    let output = env.show_task(task_id).await;
    assert!(
        output.contains("Signal:      go_work"),
        "[{}] Reviewer should emit go_work when checklist has unchecked items",
        env.name()
    );
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml with [tasks.github]"]
async fn test_fs_github_repo_backend_merging() {
    let _guard = TEST_LOCK.lock().await;
    let env = get_env().await;

    let target = match env.target_repo.as_deref() {
        Some(t) => t,
        None => {
            eprintln!("[{}] Skipping: target_repo not configured", env.name());
            return;
        }
    };

    let task_id = env
        .create_task("Repo backend merging", "Dummy task description", Stage::Merging)
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-merge-test");
    env.update_task_branches(task_id, target, "main", &work_branch).await;
    env.prepare_workspace_via_repo_backend(task_id, target, &work_branch).await;

    env.run_stage(task_id, Stage::Merging, scenarios::merging_scenario("report")).await;

    let output = env.show_task(task_id).await;
    assert!(
        output.contains("Merger complete."),
        "[{}] Merger report not found in discussion",
        env.name()
    );
}

// ---------------------------------------------------------------------------
// Cross-org GitHub repo backend tests — exercise clone_and_setup() against
// octocat/Spoon-Knife whose owner differs from fork_owner, so the backend
// must create a real fork and add a "fork" remote to the workspace.
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_fs_github_repo_backend_clone_cross_org() {
    let _guard = TEST_LOCK.lock().await;
    let env = get_env().await;

    let target = CROSS_ORG_DEST_REPO;
    let repo_name = target.rsplit('/').next().unwrap_or(target);

    let task_id = env
        .create_task(
            "Clone cross-org test",
            "Test clone_and_setup with cross-org repo (octocat/Spoon-Knife)",
            Stage::Pending,
        )
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-clone-xorg");
    env.update_task_branches(task_id, target, "main", &work_branch).await;

    // Clone via the GitHub repo backend (exercises fork creation)
    env.run_zbobr("task", &["clone", &task_id.to_string()]).await;

    let workspace_dir = env.workspaces_dir.join(format!("task#{task_id}")).join(repo_name);

    assert!(
        workspace_dir.exists(),
        "[{}] Workspace directory missing after cross-org clone: {}",
        env.name(),
        workspace_dir.display()
    );
    assert!(
        workspace_dir.join(".git").exists(),
        "[{}] Workspace is not a git repository after cross-org clone",
        env.name()
    );

    // Verify origin remote points to the upstream repo (octocat/Spoon-Knife)
    let origin_out = tokio::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(&workspace_dir)
        .output()
        .await
        .expect("failed to run git remote get-url origin");
    assert!(
        origin_out.status.success(),
        "[{}] origin remote not found in cross-org workspace",
        env.name()
    );
    let origin = String::from_utf8_lossy(&origin_out.stdout);
    assert!(
        origin.contains(repo_name),
        "[{}] origin remote '{}' does not contain repo name '{}'",
        env.name(),
        origin.trim(),
        repo_name
    );

    // Verify fork remote EXISTS (cross-org mode: fork_owner != octocat)
    let fork_out = tokio::process::Command::new("git")
        .args(["remote", "get-url", "fork"])
        .current_dir(&workspace_dir)
        .output()
        .await
        .expect("failed to run git remote get-url fork");
    assert!(
        fork_out.status.success(),
        "[{}] Fork remote missing in cross-org workspace (expected fork of {} under {})",
        env.name(),
        target,
        env.fork_owner().unwrap_or("<unknown>")
    );
    let fork_url = String::from_utf8_lossy(&fork_out.stdout);
    if let Some(fork_owner) = env.fork_owner() {
        assert!(
            fork_url.contains(fork_owner),
            "[{}] fork remote '{}' does not contain fork owner '{}'",
            env.name(),
            fork_url.trim(),
            fork_owner
        );
    }
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_fs_github_repo_backend_planning_cross_org() {
    let _guard = TEST_LOCK.lock().await;
    let env = get_env().await;

    let target = CROSS_ORG_DEST_REPO;
    let task_id = env
        .create_task("Repo backend planning cross-org", "Dummy task description", Stage::Pending)
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-plan-xorg");
    env.update_task_branches(task_id, target, "main", &work_branch).await;
    env.prepare_workspace_via_repo_backend(task_id, target, &work_branch).await;

    env.run_stage(task_id, Stage::Planning, scenarios::planning_scenario()).await;

    let output = env.show_task(task_id).await;
    assert!(
        output.contains("Signal:      go_work"),
        "[{}] Planner should emit go_work after posting plan (cross-org)",
        env.name()
    );
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_fs_github_repo_backend_working_cross_org() {
    let _guard = TEST_LOCK.lock().await;
    let env = get_env().await;

    let target = CROSS_ORG_DEST_REPO;
    let task_id = env
        .create_task("Repo backend working cross-org", "Dummy task description", Stage::Working)
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-work-xorg");
    env.update_task_branches(task_id, target, "main", &work_branch).await;
    env.prepare_workspace_via_repo_backend(task_id, target, &work_branch).await;

    env.run_stage(task_id, Stage::Working, scenarios::working_scenario()).await;

    let output = env.show_task(task_id).await;
    assert!(
        output.contains("Signal:      go_review"),
        "[{}] Worker should emit go_review (cross-org)",
        env.name()
    );
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_fs_github_repo_backend_reviewing_cross_org() {
    let _guard = TEST_LOCK.lock().await;
    let env = get_env().await;

    let target = CROSS_ORG_DEST_REPO;
    let task_id = env
        .create_task("Repo backend reviewing cross-org", "Dummy task description", Stage::Reviewing)
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-review-xorg");
    env.update_task_branches(task_id, target, "main", &work_branch).await;
    env.prepare_workspace_via_repo_backend(task_id, target, &work_branch).await;

    env.run_stage(task_id, Stage::Reviewing, scenarios::reviewing_scenario()).await;

    let output = env.show_task(task_id).await;
    assert!(
        output.contains("Signal:      go_work"),
        "[{}] Reviewer should emit go_work when checklist has unchecked items (cross-org)",
        env.name()
    );
}

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml"]
async fn test_fs_github_repo_backend_merging_cross_org() {
    let _guard = TEST_LOCK.lock().await;
    let env = get_env().await;

    let target = CROSS_ORG_DEST_REPO;
    let task_id = env
        .create_task("Repo backend merging cross-org", "Dummy task description", Stage::Merging)
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-merge-xorg");
    env.update_task_branches(task_id, target, "main", &work_branch).await;
    env.prepare_workspace_via_repo_backend(task_id, target, &work_branch).await;

    env.run_stage(task_id, Stage::Merging, scenarios::merging_scenario("report")).await;

    let output = env.show_task(task_id).await;
    assert!(
        output.contains("Merger complete."),
        "[{}] Merger report not found in discussion (cross-org)",
        env.name()
    );
}

// ---------------------------------------------------------------------------
// Confirm flag behaviour
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "GitHub-backed test; requires zbobr_github_test.toml with [tasks.github]"]
async fn cli_confirm_flag_pauses_on_stage_change() {
    let _guard = TEST_LOCK.lock().await;
    let env = get_env().await;

    let task_id = env
        .create_task_with_confirm("Confirm test", "desc", Stage::Pending, true)
        .await;

    // verify confirm field is visible in show output (print_task now prints it)
    let output = env.show_task(task_id).await;
    assert!(output.contains("Confirm:"), "confirm should be printed: {}", output);

    // update the stage using CLI; pause should automatically be set because confirm = true
    env.run_zbobr(
        "task",
        &["update", &task_id.to_string(), "--stage", "PLANNING"],
    )
    .await;
    let output2 = env.show_task(task_id).await;
    assert!(
        output2.contains("Pause:       true"),
        "task should be paused after stage change with confirm\n{output2}"
    );
}
