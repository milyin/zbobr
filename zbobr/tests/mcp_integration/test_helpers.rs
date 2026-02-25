/// Shared test bodies used by all four backend-combination files.
/// Each function takes a fully-initialised `IntegrationTestEnv` and runs
/// one complete test scenario against it.
use std::path::PathBuf;
use zbobr_dispatcher::Stage;

use super::env::IntegrationTestEnv;
use super::scenarios;

// ---------------------------------------------------------------------------
// Preparation
// ---------------------------------------------------------------------------

pub async fn run_preparation(env: &IntegrationTestEnv) {
    let repo_path = env.create_git_repo("repo_preparation").await;
    let task_id = env
        .create_task("Dummy Task", "Dummy task description", Stage::Preparing)
        .await;

    env.run_stage(
        task_id,
        Stage::Preparing,
        scenarios::preparation_scenario(&repo_path.to_string_lossy()),
    )
    .await;

    let output = env.show_task(task_id).await;
    assert!(
        output.contains("Signal:      go_plan"),
        "[{}] Preparator should emit go_plan after setting repo/branches",
        env.name()
    );
}

// ---------------------------------------------------------------------------
// Planning
// ---------------------------------------------------------------------------

pub async fn run_planning(env: &IntegrationTestEnv) {
    let repo_path = env.create_git_repo("repo_planning").await;
    let task_id = env
        .create_task("Dummy Task", "Dummy task description", Stage::Preparing)
        .await;

    let dest_repo = env.target_repo
        .as_deref()
        .map(|r| format!("https://github.com/{r}"))
        .unwrap_or_else(|| repo_path.to_string_lossy().to_string());
    let repo_name = dest_repo.rsplit('/').next().unwrap_or(&dest_repo).to_string();
    let work_branch = format!("zbobr_fix-{task_id}-test");
    env.update_task_branches(
        task_id,
        &dest_repo,
        "main",
        &work_branch,
    )
    .await;
    if let Some(target) = env.target_repo.as_deref() {
        env.prepare_workspace_via_repo_backend(task_id, target, &work_branch).await;
    } else {
        env.prepare_workspace(task_id, &repo_path, &work_branch).await;
    }

    env.run_stage(task_id, Stage::Planning, scenarios::planning_scenario())
        .await;

    let output = env.show_task(task_id).await;
    assert!(
        output.contains("Signal:      go_work"),
        "[{}] Planner should emit go_work after posting plan",
        env.name()
    );

    assert_workspace_ok(env, task_id, &repo_name, &work_branch).await;
}

// ---------------------------------------------------------------------------
// Working
// ---------------------------------------------------------------------------

pub async fn run_working(env: &IntegrationTestEnv) {
    let repo_path = env.create_git_repo("repo_working").await;
    let task_id = env
        .create_task("Dummy Task", "Dummy task description", Stage::Working)
        .await;

    let dest_repo = env.target_repo
        .as_deref()
        .map(|r| format!("https://github.com/{r}"))
        .unwrap_or_else(|| repo_path.to_string_lossy().to_string());
    let repo_name = dest_repo.rsplit('/').next().unwrap_or(&dest_repo).to_string();
    let work_branch = format!("zbobr_fix-{task_id}-test");
    env.update_task_branches(
        task_id,
        &dest_repo,
        "main",
        &work_branch,
    )
    .await;
    if let Some(target) = env.target_repo.as_deref() {
        env.prepare_workspace_via_repo_backend(task_id, target, &work_branch).await;
    } else {
        env.prepare_workspace(task_id, &repo_path, &work_branch).await;
    }

    env.run_stage(task_id, Stage::Working, scenarios::working_scenario())
        .await;

    let output = env.show_task(task_id).await;
    assert!(
        output.contains("Worker complete."),
        "[{}] Worker report not found in discussion",
        env.name()
    );
    assert!(
        output.contains("Signal:      go_review"),
        "[{}] Worker should emit go_review when all checklist items are checked",
        env.name()
    );
    assert!(
        output.contains("[x] Implement and validate worker stage integration coverage"),
        "[{}] Expected checked checklist item not found",
        env.name()
    );

    assert_workspace_ok(env, task_id, &repo_name, &work_branch).await;
}

// ---------------------------------------------------------------------------
// Reviewing
// ---------------------------------------------------------------------------

pub async fn run_reviewing(env: &IntegrationTestEnv) {
    let repo_path = env.create_git_repo("repo_reviewing").await;
    let task_id = env
        .create_task("Dummy Task", "Dummy task description", Stage::Reviewing)
        .await;

    let dest_repo = env.target_repo
        .as_deref()
        .map(|r| format!("https://github.com/{r}"))
        .unwrap_or_else(|| repo_path.to_string_lossy().to_string());
    let repo_name = dest_repo.rsplit('/').next().unwrap_or(&dest_repo).to_string();
    let work_branch = format!("zbobr_fix-{task_id}-test");
    env.update_task_branches(
        task_id,
        &dest_repo,
        "main",
        &work_branch,
    )
    .await;
    if let Some(target) = env.target_repo.as_deref() {
        env.prepare_workspace_via_repo_backend(task_id, target, &work_branch).await;
    } else {
        env.prepare_workspace(task_id, &repo_path, &work_branch).await;
    }

    env.run_stage(task_id, Stage::Reviewing, scenarios::reviewing_scenario())
        .await;

    let output = env.show_task(task_id).await;
    assert!(
        output.contains("Reviewer complete."),
        "[{}] Reviewer report not found in discussion",
        env.name()
    );
    assert!(
        output.contains("Signal:      go_work"),
        "[{}] Reviewer should emit go_work when checklist has unchecked items",
        env.name()
    );
    assert!(
        output.contains("[ ] Fix review issue: adjust edge-case handling"),
        "[{}] Expected unchecked review item not found",
        env.name()
    );

    assert_workspace_ok(env, task_id, &repo_name, &work_branch).await;
}

// ---------------------------------------------------------------------------
// Merging
// ---------------------------------------------------------------------------

pub async fn run_merging(env: &IntegrationTestEnv) {
    let repo_path = env.create_git_repo("repo_merging").await;
    let dest_repo = env.target_repo
        .as_deref()
        .map(|r| format!("https://github.com/{r}"))
        .unwrap_or_else(|| repo_path.to_string_lossy().to_string());
    let repo_name = dest_repo.rsplit('/').next().unwrap_or(&dest_repo).to_string();

    // ---- report ending ----
    let task_report = env
        .create_task("Dummy Task", "Dummy task description", Stage::Merging)
        .await;
    let branch_report = format!("zbobr_fix-{task_report}-test");
    env.update_task_branches(task_report, &dest_repo, "main", &branch_report)
        .await;
    if let Some(target) = env.target_repo.as_deref() {
        env.prepare_workspace_via_repo_backend(task_report, target, &branch_report).await;
    } else {
        env.prepare_workspace(task_report, &repo_path, &branch_report).await;
    }

    env.run_stage(
        task_report,
        Stage::Merging,
        scenarios::merging_scenario("report"),
    )
    .await;

    let output = env.show_task(task_report).await;
    assert!(
        output.contains("Merger complete."),
        "[{}] Merger report not found in discussion",
        env.name()
    );
    assert!(
        output.contains("Signal:      (none)"),
        "[{}] Merger should not set a follow-up signal",
        env.name()
    );

    assert_workspace_ok(env, task_report, &repo_name, &branch_report).await;

    // ---- ask ending ----
    let task_ask = env
        .create_task("Dummy Task", "Dummy task description", Stage::Merging)
        .await;
    let branch_ask = format!("zbobr_fix-{task_ask}-test");
    env.update_task_branches(task_ask, &dest_repo, "main", &branch_ask)
        .await;
    if let Some(target) = env.target_repo.as_deref() {
        env.prepare_workspace_via_repo_backend(task_ask, target, &branch_ask).await;
    } else {
        env.prepare_workspace(task_ask, &repo_path, &branch_ask).await;
    }

    env.run_stage(
        task_ask,
        Stage::Merging,
        scenarios::merging_scenario("ask"),
    )
    .await;

    let output = env.show_task(task_ask).await;
    assert!(
        output.contains("Need guidance on merge"),
        "[{}] Ask-user message not found in discussion",
        env.name()
    );
    assert!(
        output.contains("Pause:       true"),
        "[{}] ask_user should set the pause flag",
        env.name()
    );
}

pub async fn run_merging_with_real_conflict(env: &IntegrationTestEnv) {
    let repo_path = env.create_git_repo("repo_merging_conflict").await;
    let repo_path_str = repo_path.to_string_lossy().to_string();
    let dest_repo = env.target_repo
        .as_deref()
        .map(|r| format!("https://github.com/{r}"))
        .unwrap_or_else(|| repo_path_str.clone());
    let repo_name = dest_repo.rsplit('/').next().unwrap_or(&dest_repo).to_string();

    // build conflicting history on the local repo
    write_and_commit(&repo_path, "conflict_file.txt", "line1\nline2\nline3\n", "Initial").await;

    let work_branch = "work_branch_conflict";
    git_in(&repo_path, &["checkout", "-b", work_branch]).await;
    write_and_commit(&repo_path, "conflict_file.txt", "line1\nline2 work\nline3\n", "Work change").await;

    git_in(&repo_path, &["checkout", "main"]).await;
    write_and_commit(&repo_path, "conflict_file.txt", "line1\nline2 main\nline3\n", "Main change").await;

    let task_id = env
        .create_task("Conflict task", "Test merging with real conflicts", Stage::Merging)
        .await;
    env.update_task_branches(task_id, &dest_repo, "main", work_branch)
        .await;

    // Manually set up workspace with a live merge conflict
    let workspace_dir = env.workspaces_dir.join(format!("task#{task_id}"));
    tokio::fs::create_dir_all(&workspace_dir).await.unwrap();
    let work_dir = workspace_dir.join(&repo_name);

    let cp_ok = tokio::process::Command::new("cp")
        .args(["-r", &repo_path_str, work_dir.to_str().unwrap()])
        .status()
        .await
        .unwrap()
        .success();
    assert!(cp_ok, "[{}] cp to workspace failed", env.name());

    git_in(&work_dir, &["checkout", work_branch]).await;

    // merge should produce conflict markers
    let merge = tokio::process::Command::new("git")
        .args(["merge", "main", "--no-edit"])
        .current_dir(&work_dir)
        .output()
        .await
        .unwrap();
    assert!(
        !merge.status.success(),
        "[{}] Expected merge conflict but merge succeeded",
        env.name()
    );

    env.run_stage(
        task_id,
        Stage::Merging,
        scenarios::merging_conflict_scenario(),
    )
    .await;

    let output = env.show_task(task_id).await;
    assert!(
        output.contains("Detected merge conflicts"),
        "[{}] Merger should report detected conflicts",
        env.name()
    );
    assert!(
        output.contains("Conflict:    false"),
        "[{}] Merger should clear the conflict flag",
        env.name()
    );
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

async fn write_and_commit(repo: &PathBuf, file: &str, content: &str, msg: &str) {
    tokio::fs::write(repo.join(file), content).await.unwrap();
    git_in(repo, &["add", file]).await;
    git_in(repo, &["commit", "-m", msg]).await;
}

async fn git_in(dir: &PathBuf, args: &[&str]) {
    let ok = tokio::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .await
        .unwrap()
        .success();
    assert!(ok, "git {:?} in {} failed", args, dir.display());
}

/// Assert that the task workspace exists, is a git repo, contains the right
/// branches, and is currently on the work branch.
async fn assert_workspace_ok(
    env: &IntegrationTestEnv,
    task_id: u64,
    repo_name: &str,
    work_branch: &str,
) {
    let work_dir = env
        .workspaces_dir
        .join(format!("task#{task_id}"))
        .join(repo_name);

    assert!(
        work_dir.exists(),
        "[{}] Workspace directory missing: {}",
        env.name(),
        work_dir.display()
    );
    assert!(
        work_dir.join(".git").exists(),
        "[{}] Workspace is not a git repository",
        env.name()
    );

    let branches = git_output(&work_dir, &["branch"]).await;
    assert!(
        branches.contains("main"),
        "[{}] 'main' branch not found in workspace",
        env.name()
    );
    assert!(
        branches.contains(work_branch),
        "[{}] Work branch '{work_branch}' not found in workspace",
        env.name()
    );

    let current = git_output(&work_dir, &["branch", "--show-current"]).await;
    assert_eq!(
        current.trim(),
        work_branch,
        "[{}] Current branch is not the work branch",
        env.name()
    );
}

async fn git_output(dir: &PathBuf, args: &[&str]) -> String {
    let out = tokio::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .await
        .unwrap();
    String::from_utf8_lossy(&out.stdout).to_string()
}
