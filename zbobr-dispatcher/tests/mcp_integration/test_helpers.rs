//! Shared test bodies used by all backend-combination test files.
//! Each function takes a fully-initialised `IntegrationTestEnv` and runs
//! one complete test scenario against it.
#![allow(dead_code)]

use std::path::PathBuf;

use zbobr_dispatcher::{CommentType, Signal, Stage, task::Parameter};
use zbobr_api::task::ChecklistItem;

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

    let task = env.get_task(task_id).await;
    assert_eq!(
        task.signal,
        Some(Signal::GoPlan),
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

    let dest_repo = env
        .target_repo
        .as_deref()
        .map(|r| format!("https://github.com/{r}"))
        .unwrap_or_else(|| repo_path.to_string_lossy().to_string());
    let repo_name = dest_repo
        .rsplit('/')
        .next()
        .unwrap_or(&dest_repo)
        .to_string();
    let work_branch = format!("zbobr_fix-{task_id}-test");
    env.update_task_branches(task_id, &dest_repo, "main", &work_branch)
        .await;

    env.run_stage(task_id, Stage::Planning, scenarios::planning_scenario())
        .await;

    let task = env.get_task(task_id).await;
    assert_eq!(
        task.signal,
        Some(Signal::GoWork),
        "[{}] Planner should emit go_work after posting plan",
        env.name()
    );
    assert!(
        task.parameters.contains_key(&Parameter::PrUrl),
        "[{}] PR URL should be stored after planning stage",
        env.name()
    );
    assert_pr_url_points_to_branch(env, &task, &work_branch).await;

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

    let dest_repo = env
        .target_repo
        .as_deref()
        .map(|r| format!("https://github.com/{r}"))
        .unwrap_or_else(|| repo_path.to_string_lossy().to_string());
    let repo_name = dest_repo
        .rsplit('/')
        .next()
        .unwrap_or(&dest_repo)
        .to_string();
    let work_branch = format!("zbobr_fix-{task_id}-test");
    env.update_task_branches(task_id, &dest_repo, "main", &work_branch)
        .await;

    env.run_stage(task_id, Stage::Working, scenarios::working_scenario())
        .await;

    let task = env.get_task(task_id).await;
    let comments = env.get_comments(task_id).await;
    assert!(
        comments.iter().any(|c| c.text.contains("Worker complete.")),
        "[{}] Worker report not found in discussion",
        env.name()
    );
    assert_eq!(
        task.signal,
        Some(Signal::GoReview),
        "[{}] Worker should emit go_review when all checklist items are checked",
        env.name()
    );
    assert!(
        task.checklist
            .iter()
            .any(|i| i.checked && i.text.contains("Implement and validate worker stage")),
        "[{}] Expected checked checklist item not found",
        env.name()
    );
    assert!(
        task.parameters.contains_key(&Parameter::PrUrl),
        "[{}] PR URL should be stored after working stage",
        env.name()
    );
    assert_pr_url_points_to_branch(env, &task, &work_branch).await;

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

    let dest_repo = env
        .target_repo
        .as_deref()
        .map(|r| format!("https://github.com/{r}"))
        .unwrap_or_else(|| repo_path.to_string_lossy().to_string());
    let repo_name = dest_repo
        .rsplit('/')
        .next()
        .unwrap_or(&dest_repo)
        .to_string();
    let work_branch = format!("zbobr_fix-{task_id}-test");
    env.update_task_branches(task_id, &dest_repo, "main", &work_branch)
        .await;

    // insert a dummy unchecked checklist item so the reviewer has something to
    // report and therefore will route back to the planner.  This mirrors the
    // behaviour of actual workflows where a review usually discovers issues.
    env.zbobr
        .modify_task(
            task_id,
            Box::new(|mut task| {
                task.checklist.push(ChecklistItem {
                    id: "issue".to_string(),
                    text: "issue found during review".to_string(),
                    checked: false,
                });
                task
            }),
        )
        .await
        .unwrap_or_else(|e| {
            panic!(
                "[{}] failed to add review checklist item: {e}",
                env.name()
            )
        });

    env.run_stage(task_id, Stage::Reviewing, scenarios::reviewing_scenario())
        .await;

    let task = env.get_task(task_id).await;
    let comments = env.get_comments(task_id).await;
    assert!(
        comments.iter().any(|c| c.text.contains("Reviewer complete.")),
        "[{}] Reviewer report not found in discussion",
        env.name()
    );
    assert_eq!(
        task.signal,
        Some(Signal::GoPlan),
        "[{}] Reviewer should emit go_plan to route to planner",
        env.name()
    );
    assert!(
        task.parameters.contains_key(&Parameter::PrUrl),
        "[{}] PR URL should be stored after reviewing stage",
        env.name()
    );
    assert_pr_url_points_to_branch(env, &task, &work_branch).await;

    assert_workspace_ok(env, task_id, &repo_name, &work_branch).await;
}

// ---------------------------------------------------------------------------
// Reviewing — approval path (no issues → DONE + PR)
// ---------------------------------------------------------------------------

pub async fn run_reviewing_approval(env: &IntegrationTestEnv) {
    let repo_path = env.create_git_repo("repo_reviewing_approval").await;
    let task_id = env
        .create_task("Dummy Task", "Dummy task description", Stage::Reviewing)
        .await;

    let dest_repo = env
        .target_repo
        .as_deref()
        .map(|r| format!("https://github.com/{r}"))
        .unwrap_or_else(|| repo_path.to_string_lossy().to_string());
    let repo_name = dest_repo
        .rsplit('/')
        .next()
        .unwrap_or(&dest_repo)
        .to_string();
    let work_branch = format!("zbobr_fix-{task_id}-test");
    env.update_task_branches(task_id, &dest_repo, "main", &work_branch)
        .await;

    if let Some(target) = env.target_repo.as_deref() {
        env.prepare_workspace_via_repo_backend(task_id, target, &work_branch)
            .await;
    } else {
        env.prepare_workspace(task_id, &repo_path, &work_branch)
            .await;
    }

    // Add a placeholder commit so the work branch differs from main (PR requires changes).
    let work_dir = env
        .workspaces_dir
        .join(format!("task#{task_id}"))
        .join(&repo_name);
    write_and_commit(
        &work_dir,
        "ZBOBR_PLACEHOLDER.md",
        &format!("placeholder for task #{task_id}\n"),
        "chore: add placeholder for PR",
    )
    .await;

    env.run_stage(
        task_id,
        Stage::Reviewing,
        scenarios::reviewing_approval_scenario(),
    )
    .await;

    let task = env.get_task(task_id).await;
    // approval path should mark the task done instead of signalling planner
    assert_eq!(
        task.stage,
        Stage::Done,
        "[{}] Reviewer approval should mark task DONE when no checklist items remain",
        env.name()
    );
    assert!(
        task.signal.is_none(),
        "[{}] DONE task should not have any follow-up signal",
        env.name()
    );
    assert!(
        task.parameters.contains_key(&Parameter::PrUrl),
        "[{}] PR URL should be stored after reviewer approval",
        env.name()
    );
    assert_pr_url_points_to_branch(env, &task, &work_branch).await;
    assert_pr_has_commits(env, &task, "main").await;
}

// ---------------------------------------------------------------------------
// Merging
// ---------------------------------------------------------------------------

pub async fn run_merging(env: &IntegrationTestEnv) {
    let repo_path = env.create_git_repo("repo_merging").await;
    let dest_repo = env
        .target_repo
        .as_deref()
        .map(|r| format!("https://github.com/{r}"))
        .unwrap_or_else(|| repo_path.to_string_lossy().to_string());
    let repo_name = dest_repo
        .rsplit('/')
        .next()
        .unwrap_or(&dest_repo)
        .to_string();

    // ---- report ending ----
    let task_report = env
        .create_task("Dummy Task", "Dummy task description", Stage::Merging)
        .await;
    let branch_report = format!("zbobr_fix-{task_report}-test");
    env.update_task_branches(task_report, &dest_repo, "main", &branch_report)
        .await;
    if let Some(target) = env.target_repo.as_deref() {
        env.prepare_workspace_via_repo_backend(task_report, target, &branch_report)
            .await;
    } else {
        env.prepare_workspace(task_report, &repo_path, &branch_report)
            .await;
    }

    // Add a dummy commit so the work branch has changes above main (required for PR creation).
    let work_dir_report = env
        .workspaces_dir
        .join(format!("task#{task_report}"))
        .join(&repo_name);
    write_and_commit(
        &work_dir_report,
        "ZBOBR_PLACEHOLDER.md",
        &format!("placeholder for task #{task_report}\n"),
        "chore: add placeholder for PR",
    )
    .await;

    env.run_stage(
        task_report,
        Stage::Merging,
        scenarios::merging_scenario("report"),
    )
    .await;

    let task = env.get_task(task_report).await;
    let comments = env.get_comments(task_report).await;
    assert!(
        comments.iter().any(|c| c.text.contains("Merger complete.")),
        "[{}] Merger report not found in discussion",
        env.name()
    );
    assert!(
        task.signal.is_none(),
        "[{}] Merger should not set a follow-up signal",
        env.name()
    );
    assert!(
        task.parameters.contains_key(&Parameter::PrUrl),
        "[{}] PR URL should be stored in task parameters after merger",
        env.name()
    );
    assert_pr_url_points_to_branch(env, &task, &branch_report).await;
    assert_pr_has_commits(env, &task, "main").await;

    assert_workspace_ok(env, task_report, &repo_name, &branch_report).await;

    // ---- ask ending ----
    let task_ask = env
        .create_task("Dummy Task", "Dummy task description", Stage::Merging)
        .await;
    let branch_ask = format!("zbobr_fix-{task_ask}-test");
    env.update_task_branches(task_ask, &dest_repo, "main", &branch_ask)
        .await;
    if let Some(target) = env.target_repo.as_deref() {
        env.prepare_workspace_via_repo_backend(task_ask, target, &branch_ask)
            .await;
    } else {
        env.prepare_workspace(task_ask, &repo_path, &branch_ask)
            .await;
    }

    env.run_stage(task_ask, Stage::Merging, scenarios::merging_scenario("ask"))
        .await;

    let task_ask_data = env.get_task(task_ask).await;
    let comments_ask = env.get_comments(task_ask).await;
    assert!(
        comments_ask
            .iter()
            .any(|c| c.text.contains("Need guidance on merge")),
        "[{}] Ask-user message not found in discussion",
        env.name()
    );
    assert!(
        task_ask_data.pause,
        "[{}] ask_user should set the pause flag",
        env.name()
    );
}

pub async fn run_merging_with_real_conflict(env: &IntegrationTestEnv) {
    let repo_path = env.create_git_repo("repo_merging_conflict").await;
    let repo_path_str = repo_path.to_string_lossy().to_string();
    let dest_repo = env
        .target_repo
        .as_deref()
        .map(|r| format!("https://github.com/{r}"))
        .unwrap_or_else(|| repo_path_str.clone());
    let repo_name = dest_repo
        .rsplit('/')
        .next()
        .unwrap_or(&dest_repo)
        .to_string();

    let task_id = env
        .create_task(
            "Conflict task",
            "Test merging with real conflicts",
            Stage::Merging,
        )
        .await;
    let work_branch = format!("zbobr_conflict-{task_id}-test");
    env.update_task_branches(task_id, &dest_repo, "main", &work_branch)
        .await;

    // Set up workspace with a live merge conflict.
    let work_dir = if let Some(target) = env.target_repo.as_deref() {
        let wd = env
            .prepare_workspace_via_repo_backend(task_id, target, &work_branch)
            .await;

        write_and_commit(
            &wd,
            "conflict_file.txt",
            "line1\nline2 work\nline3\n",
            "Work change",
        )
        .await;

        git_in(&wd, &["checkout", "main"]).await;
        write_and_commit(
            &wd,
            "conflict_file.txt",
            "line1\nline2 main\nline3\n",
            "Main change",
        )
        .await;

        git_in(&wd, &["checkout", &work_branch]).await;
        wd
    } else {
        // Local-only: build conflicting history on the source repo then cp -r.
        write_and_commit(
            &repo_path,
            "conflict_file.txt",
            "line1\nline2\nline3\n",
            "Initial",
        )
        .await;

        git_in(&repo_path, &["checkout", "-b", &work_branch]).await;
        write_and_commit(
            &repo_path,
            "conflict_file.txt",
            "line1\nline2 work\nline3\n",
            "Work change",
        )
        .await;

        git_in(&repo_path, &["checkout", "main"]).await;
        write_and_commit(
            &repo_path,
            "conflict_file.txt",
            "line1\nline2 main\nline3\n",
            "Main change",
        )
        .await;

        let workspace_dir = env.workspaces_dir.join(format!("task#{task_id}"));
        tokio::fs::create_dir_all(&workspace_dir).await.unwrap();
        let wd = workspace_dir.join(&repo_name);

        let cp_ok = tokio::process::Command::new("cp")
            .args(["-r", &repo_path_str, wd.to_str().unwrap()])
            .status()
            .await
            .unwrap()
            .success();
        assert!(cp_ok, "[{}] cp to workspace failed", env.name());

        git_in(&wd, &["checkout", &work_branch]).await;
        wd
    };

    // Confirm there is a merge conflict.
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

    let task = env.get_task(task_id).await;
    let comments = env.get_comments(task_id).await;
    assert!(
        comments.iter().any(|c| c.text.contains("Detected merge conflicts")),
        "[{}] Merger should report detected conflicts",
        env.name()
    );
    assert!(
        !task.conflict,
        "[{}] Merger should clear the conflict flag",
        env.name()
    );
}

// ---------------------------------------------------------------------------
// Conflict detection
// ---------------------------------------------------------------------------

/// Verify the automatic conflict-detection path executed by the role session
/// code.
pub async fn run_conflict_detection(env: &IntegrationTestEnv) {
    if env.target_repo.is_some() {
        eprintln!(
            "[{}] Skipping run_conflict_detection: requires local repo backend",
            env.name()
        );
        return;
    }

    let repo_path = env.create_git_repo("repo_conflict_detection").await;
    let repo_path_str = repo_path.to_string_lossy().to_string();
    let repo_name = repo_path.file_name().unwrap().to_str().unwrap().to_string();
    let work_branch = "zbobr_conflict-detect-work";

    git_in(&repo_path, &["checkout", "-b", work_branch]).await;
    write_and_commit(
        &repo_path,
        "conflict_file.txt",
        "line1\nline2 work\nline3\n",
        "Work change",
    )
    .await;
    git_in(&repo_path, &["checkout", "main"]).await;
    write_and_commit(
        &repo_path,
        "conflict_file.txt",
        "line1\nline2 main\nline3\n",
        "Main change",
    )
    .await;

    let task_id = env
        .create_task(
            "Conflict Detection",
            "Dummy task description",
            Stage::Working,
        )
        .await;
    env.update_task_branches(task_id, &repo_path_str, "main", work_branch)
        .await;

    // Run the Worker stage.  The dispatcher clones the repo, attempts
    // `git merge main`, detects the conflict, sets conflict=true, and exits
    // successfully without invoking the mcp-tester agent.
    env.run_stage(task_id, Stage::Working, scenarios::working_scenario())
        .await;

    let task = env.get_task(task_id).await;
    assert!(
        task.conflict,
        "[{}] Conflict flag should be set after automatic conflict detection",
        env.name()
    );

    // Workspace must exist and be in an unresolved merge state.
    let work_dir = env
        .workspaces_dir
        .join(format!("task#{task_id}"))
        .join(&repo_name);
    let git_status = git_output(&work_dir, &["status"]).await;
    assert!(
        git_status.contains("You have unmerged paths") || git_status.contains("Unmerged paths"),
        "[{}] Workspace should be in a conflicted git state:\n{git_status}",
        env.name()
    );

    // Run the Merger on the already-conflicted workspace.
    env.run_stage(
        task_id,
        Stage::Merging,
        scenarios::merging_conflict_scenario(),
    )
    .await;

    let task = env.get_task(task_id).await;
    assert!(
        !task.conflict,
        "[{}] Conflict flag should be cleared after Merger session",
        env.name()
    );
}

// ---------------------------------------------------------------------------
// report_error signal preservation
// ---------------------------------------------------------------------------

pub async fn run_report_error_preserves_signal(env: &IntegrationTestEnv) {
    let repo_path = env.create_git_repo("repo_report_error").await;
    let dest_repo = env
        .target_repo
        .as_deref()
        .map(|r| format!("https://github.com/{r}"))
        .unwrap_or_else(|| repo_path.to_string_lossy().to_string());
    let task_id = env
        .create_task("Error Task", "Dummy task description", Stage::Working)
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-err-test");
    env.update_task_branches(task_id, &dest_repo, "main", &work_branch)
        .await;

    // Set a signal before the session so we can verify it survives report_error.
    env.update_task_signal(task_id, "go_work").await;

    env.run_stage(
        task_id,
        Stage::Working,
        scenarios::worker_report_error_scenario(),
    )
    .await;

    let task = env.get_task(task_id).await;
    let comments = env.get_comments(task_id).await;
    assert!(
        comments
            .iter()
            .any(|c| c.text.contains("Something went wrong during work")),
        "[{}] report_error message should appear in discussion",
        env.name()
    );
    assert!(
        task.pause,
        "[{}] report_error must set the pause flag",
        env.name()
    );
    assert_eq!(
        task.signal,
        Some(Signal::GoWork),
        "[{}] report_error must not clear the signal",
        env.name()
    );
}

// ---------------------------------------------------------------------------
// Signal Preservation During Conflict Resolution
// ---------------------------------------------------------------------------

pub async fn run_signal_preservation_during_conflict(env: &IntegrationTestEnv) {
    if env.target_repo.is_some() {
        eprintln!(
            "[{}] Skipping run_signal_preservation_during_conflict: requires local repo backend",
            env.name()
        );
        return;
    }

    let repo_path = env.create_git_repo("repo_signal_preservation").await;
    let repo_path_str = repo_path.to_string_lossy().to_string();
    let work_branch = "zbobr_signal-preserve-work";

    git_in(&repo_path, &["checkout", "-b", work_branch]).await;
    write_and_commit(
        &repo_path,
        "conflict_file.txt",
        "line1\nline2 work\nline3\n",
        "Work change",
    )
    .await;
    git_in(&repo_path, &["checkout", "main"]).await;
    write_and_commit(
        &repo_path,
        "conflict_file.txt",
        "line1\nline2 main\nline3\n",
        "Main change",
    )
    .await;

    let task_id = env
        .create_task(
            "Signal Preservation Test",
            "Test that signal is preserved during merge conflict resolution",
            Stage::Working,
        )
        .await;

    env.update_task_branches(task_id, &repo_path_str, "main", work_branch)
        .await;

    // Set the task to have a go_work signal BEFORE running the worker.
    // Because conflict detection exits BEFORE the signal-clearing step,
    // this signal must survive intact through the early-merge exit path.
    env.update_task_signal(task_id, "go_work").await;

    env.run_stage(task_id, Stage::Working, scenarios::working_scenario())
        .await;

    let task = env.get_task(task_id).await;
    assert!(
        task.conflict,
        "[{}] Conflict flag should be set after automatic conflict detection",
        env.name()
    );
    assert_eq!(
        task.signal,
        Some(Signal::GoWork),
        "[{}] Signal must be preserved when conflict detection exits before clearing step",
        env.name()
    );

    // Run the Merger stage to resolve the conflict.
    env.run_stage(
        task_id,
        Stage::Merging,
        scenarios::merging_conflict_scenario(),
    )
    .await;

    let task = env.get_task(task_id).await;
    assert!(
        !task.conflict,
        "[{}] Conflict flag should be cleared after Merger session",
        env.name()
    );

    // CRITICAL: The signal should STILL be present after Merger finishes!
    assert_eq!(
        task.signal,
        Some(Signal::GoWork),
        "[{}] Signal should be preserved after Merger completes",
        env.name()
    );
}

// ---------------------------------------------------------------------------
// Plan history with index (GET_PLAN offset parameter)
// ---------------------------------------------------------------------------

/// Verify that GET_PLAN:
///  - returns the task description as a user Reply comment when no plan exists
///  - returns only the plan and subsequent comments up to the next plan for each offset
///  - returns an error for an out-of-range offset
pub async fn run_plan_history_with_index(env: &IntegrationTestEnv) {
    let repo_path = env.create_git_repo("repo_plan_history").await;
    const TASK_DESCRIPTION: &str = "Plan history MCP test description";

    let dest_repo = env
        .target_repo
        .as_deref()
        .map(|r| format!("https://github.com/{r}"))
        .unwrap_or_else(|| repo_path.to_string_lossy().to_string());
    let task_id = env
        .create_task("Plan History Task", TASK_DESCRIPTION, Stage::Planning)
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-plan-history");
    env.update_task_branches(task_id, &dest_repo, "main", &work_branch)
        .await;

    env.run_stage(
        task_id,
        Stage::Planning,
        scenarios::multiple_plans_scenario(TASK_DESCRIPTION),
    )
    .await;

    // Directly verify the structured comment history in the backend.
    let comments = env
        .zbobr
        .get_task_comments(task_id)
        .await
        .unwrap_or_else(|e| panic!("[{}] failed to get structured comments: {e}", env.name()));

    // Should have exactly 3 comments: plan-1, error, plan-2
    let plan_comments: Vec<_> = comments
        .iter()
        .filter(|c| c.comment_type == CommentType::Plan)
        .collect();
    assert_eq!(
        plan_comments.len(),
        2,
        "[{}] Expected exactly 2 Plan comments, got {} (comments: {:?})",
        env.name(),
        plan_comments.len(),
        comments
            .iter()
            .map(|c| format!("{:?}: {}", c.comment_type, &c.text[..c.text.len().min(40)]))
            .collect::<Vec<_>>()
    );

    assert!(
        plan_comments[0].text.contains("First plan"),
        "[{}] First plan comment should contain 'First plan', got: {}",
        env.name(),
        plan_comments[0].text
    );
    assert!(
        plan_comments[1].text.contains("Second plan"),
        "[{}] Second plan comment should contain 'Second plan', got: {}",
        env.name(),
        plan_comments[1].text
    );

    // Verify the error comment (between the two plans) is present
    let error_comments: Vec<_> = comments
        .iter()
        .filter(|c| c.comment_type == CommentType::Error)
        .collect();
    assert_eq!(
        error_comments.len(),
        1,
        "[{}] Expected exactly 1 Error comment between plans",
        env.name()
    );
    assert!(
        error_comments[0].text.contains("Issue found after first plan"),
        "[{}] Error comment should contain expected text, got: {}",
        env.name(),
        error_comments[0].text
    );

    // Verify the ordering: plan-1, error, plan-2
    let plan1_pos = comments.iter().position(|c| {
        c.comment_type == CommentType::Plan && c.text.contains("First plan")
    });
    let error_pos = comments.iter().position(|c| c.comment_type == CommentType::Error);
    let plan2_pos = comments.iter().position(|c| {
        c.comment_type == CommentType::Plan && c.text.contains("Second plan")
    });

    assert!(
        plan1_pos < error_pos && error_pos < plan2_pos,
        "[{}] Comments must be ordered: plan-1 ({plan1_pos:?}) < error ({error_pos:?}) < plan-2 ({plan2_pos:?})",
        env.name()
    );

    // plan-2 (latest) is at the end, so there should be no comments after it
    let plan2_idx = plan2_pos.unwrap();
    assert_eq!(
        plan2_idx,
        comments.len() - 1,
        "[{}] plan-2 should be the last comment (no trailing comments after it)",
        env.name()
    );
}

/// A well-known public repository owned by a different organisation than any
/// typical test user.  Used by the cross-org tests to exercise the fork
/// creation path (fork_owner != target repo owner).
const CROSS_ORG_DEST_REPO: &str = "octocat/Spoon-Knife";

/// Test `clone_and_setup` against a same-org target (`env.target_repo`).
/// Skipped when the repo backend is not GitHub or `target_repo` is not set.
pub async fn run_repo_backend_clone(env: &IntegrationTestEnv) {
    let Some(target) = env.target_repo.as_deref() else {
        eprintln!(
            "[{}] Skipping run_repo_backend_clone: target_repo not configured",
            env.name()
        );
        return;
    };
    if env.fork_owner().is_none() {
        eprintln!(
            "[{}] Skipping run_repo_backend_clone: not a GitHub repo backend",
            env.name()
        );
        return;
    }

    let repo_name = target.rsplit('/').next().unwrap_or(target);
    let task_id = env
        .create_task(
            "Clone test",
            "Test clone_and_setup via repo backend",
            Stage::Pending,
        )
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-clone-test");
    env.update_task_branches(task_id, target, "main", &work_branch)
        .await;

    let task = env.get_task(task_id).await;
    let dest_repo = task
        .parameters
        .get(&Parameter::DestinationRepository)
        .cloned()
        .unwrap();
    let dest_branch = task
        .parameters
        .get(&Parameter::DestinationBranch)
        .cloned()
        .unwrap_or_else(|| "main".to_string());
    env.zbobr
        .clone_and_setup(&dest_repo, &work_branch, &dest_branch, task_id)
        .await
        .unwrap();

    let workspace_dir = env
        .workspaces_dir
        .join(format!("task#{task_id}"))
        .join(repo_name);
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

    // Same-org mode: no fork remote expected.
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

/// Test `clone_and_setup` against `octocat/Spoon-Knife` (cross-org).
/// Skipped when the repo backend is not GitHub.
pub async fn run_repo_backend_clone_cross_org(env: &IntegrationTestEnv) {
    if env.fork_owner().is_none() {
        eprintln!(
            "[{}] Skipping run_repo_backend_clone_cross_org: not a GitHub repo backend",
            env.name()
        );
        return;
    }

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
    env.update_task_branches(task_id, target, "main", &work_branch)
        .await;

    let task = env.get_task(task_id).await;
    let dest_repo = task
        .parameters
        .get(&Parameter::DestinationRepository)
        .cloned()
        .unwrap();
    let dest_branch = task
        .parameters
        .get(&Parameter::DestinationBranch)
        .cloned()
        .unwrap_or_else(|| "main".to_string());
    env.zbobr
        .clone_and_setup(&dest_repo, &work_branch, &dest_branch, task_id)
        .await
        .unwrap();

    let workspace_dir = env
        .workspaces_dir
        .join(format!("task#{task_id}"))
        .join(repo_name);
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

    // Cross-org mode: fork remote must exist.
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

// ---------------------------------------------------------------------------
// Repo backend — same-org planning / working / reviewing / merging
// ---------------------------------------------------------------------------

pub async fn run_repo_backend_planning(env: &IntegrationTestEnv) {
    let Some(target) = env.target_repo.as_deref() else {
        eprintln!(
            "[{}] Skipping run_repo_backend_planning: target_repo not configured",
            env.name()
        );
        return;
    };
    if env.fork_owner().is_none() {
        eprintln!(
            "[{}] Skipping run_repo_backend_planning: not a GitHub repo backend",
            env.name()
        );
        return;
    }
    repo_backend_planning_for(env, target, "plan").await;
}

pub async fn run_repo_backend_working(env: &IntegrationTestEnv) {
    let Some(target) = env.target_repo.as_deref() else {
        eprintln!(
            "[{}] Skipping run_repo_backend_working: target_repo not configured",
            env.name()
        );
        return;
    };
    if env.fork_owner().is_none() {
        eprintln!(
            "[{}] Skipping run_repo_backend_working: not a GitHub repo backend",
            env.name()
        );
        return;
    }
    repo_backend_working_for(env, target, "work").await;
}

pub async fn run_repo_backend_reviewing(env: &IntegrationTestEnv) {
    let Some(target) = env.target_repo.as_deref() else {
        eprintln!(
            "[{}] Skipping run_repo_backend_reviewing: target_repo not configured",
            env.name()
        );
        return;
    };
    if env.fork_owner().is_none() {
        eprintln!(
            "[{}] Skipping run_repo_backend_reviewing: not a GitHub repo backend",
            env.name()
        );
        return;
    }
    repo_backend_reviewing_for(env, target, "review").await;
}

pub async fn run_repo_backend_merging(env: &IntegrationTestEnv) {
    let Some(target) = env.target_repo.as_deref() else {
        eprintln!(
            "[{}] Skipping run_repo_backend_merging: target_repo not configured",
            env.name()
        );
        return;
    };
    if env.fork_owner().is_none() {
        eprintln!(
            "[{}] Skipping run_repo_backend_merging: not a GitHub repo backend",
            env.name()
        );
        return;
    }
    repo_backend_merging_for(env, target, "merge").await;
}

// ---------------------------------------------------------------------------
// Repo backend — cross-org planning / working / reviewing / merging
// ---------------------------------------------------------------------------

pub async fn run_repo_backend_planning_cross_org(env: &IntegrationTestEnv) {
    if env.fork_owner().is_none() {
        eprintln!(
            "[{}] Skipping run_repo_backend_planning_cross_org: not a GitHub repo backend",
            env.name()
        );
        return;
    }
    repo_backend_planning_for(env, CROSS_ORG_DEST_REPO, "xorg-plan").await;
}

pub async fn run_repo_backend_working_cross_org(env: &IntegrationTestEnv) {
    if env.fork_owner().is_none() {
        eprintln!(
            "[{}] Skipping run_repo_backend_working_cross_org: not a GitHub repo backend",
            env.name()
        );
        return;
    }
    repo_backend_working_for(env, CROSS_ORG_DEST_REPO, "xorg-work").await;
}

pub async fn run_repo_backend_reviewing_cross_org(env: &IntegrationTestEnv) {
    if env.fork_owner().is_none() {
        eprintln!(
            "[{}] Skipping run_repo_backend_reviewing_cross_org: not a GitHub repo backend",
            env.name()
        );
        return;
    }
    repo_backend_reviewing_for(env, CROSS_ORG_DEST_REPO, "xorg-review").await;
}

pub async fn run_repo_backend_merging_cross_org(env: &IntegrationTestEnv) {
    if env.fork_owner().is_none() {
        eprintln!(
            "[{}] Skipping run_repo_backend_merging_cross_org: not a GitHub repo backend",
            env.name()
        );
        return;
    }
    repo_backend_merging_for(env, CROSS_ORG_DEST_REPO, "xorg-merge").await;
}

// ---------------------------------------------------------------------------
// Confirm flag
// ---------------------------------------------------------------------------

/// Verify that a stage change with `confirm=true` triggers an automatic pause.
pub async fn run_cli_confirm_flag(env: &IntegrationTestEnv) {
    let task_id = env
        .create_task_with_confirm("Confirm test", "desc", Stage::Pending, true)
        .await;

    env.update_task_stage(task_id, Stage::Planning).await;

    let task = env.get_task(task_id).await;
    assert!(
        task.pause,
        "[{}] task should be paused after stage change with confirm",
        env.name()
    );
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

async fn repo_backend_planning_for(env: &IntegrationTestEnv, target: &str, suffix: &str) {
    let task_id = env
        .create_task(
            "Repo backend planning",
            "Dummy task description",
            Stage::Pending,
        )
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-{suffix}-test");
    env.update_task_branches(task_id, target, "main", &work_branch)
        .await;
    env.prepare_workspace_via_repo_backend(task_id, target, &work_branch)
        .await;

    env.run_stage(task_id, Stage::Planning, scenarios::planning_scenario())
        .await;

    let task = env.get_task(task_id).await;
    assert_eq!(
        task.signal,
        Some(Signal::GoWork),
        "[{}] Planner should emit go_work after posting plan",
        env.name()
    );
}

async fn repo_backend_working_for(env: &IntegrationTestEnv, target: &str, suffix: &str) {
    let task_id = env
        .create_task(
            "Repo backend working",
            "Dummy task description",
            Stage::Working,
        )
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-{suffix}-test");
    env.update_task_branches(task_id, target, "main", &work_branch)
        .await;
    env.prepare_workspace_via_repo_backend(task_id, target, &work_branch)
        .await;

    env.run_stage(task_id, Stage::Working, scenarios::working_scenario())
        .await;

    let task = env.get_task(task_id).await;
    assert_eq!(
        task.signal,
        Some(Signal::GoReview),
        "[{}] Worker should emit go_review",
        env.name()
    );
}

async fn repo_backend_reviewing_for(env: &IntegrationTestEnv, target: &str, suffix: &str) {
    let task_id = env
        .create_task(
            "Repo backend reviewing",
            "Dummy task description",
            Stage::Reviewing,
        )
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-{suffix}-test");
    env.update_task_branches(task_id, target, "main", &work_branch)
        .await;
    env.prepare_workspace_via_repo_backend(task_id, target, &work_branch)
        .await;

    env.run_stage(task_id, Stage::Reviewing, scenarios::reviewing_scenario())
        .await;

    let task = env.get_task(task_id).await;
    assert_eq!(
        task.signal,
        Some(Signal::GoPlan),
        "[{}] Reviewer should emit go_plan to route to planner",
        env.name()
    );
}

async fn repo_backend_merging_for(env: &IntegrationTestEnv, target: &str, suffix: &str) {
    let task_id = env
        .create_task(
            "Repo backend merging",
            "Dummy task description",
            Stage::Merging,
        )
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-{suffix}-test");
    env.update_task_branches(task_id, target, "main", &work_branch)
        .await;
    env.prepare_workspace_via_repo_backend(task_id, target, &work_branch)
        .await;

    env.run_stage(
        task_id,
        Stage::Merging,
        scenarios::merging_scenario("report"),
    )
    .await;

    let comments = env.get_comments(task_id).await;
    assert!(
        comments.iter().any(|c| c.text.contains("Merger complete.")),
        "[{}] Merger report not found in discussion",
        env.name()
    );
}

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

/// Validate that the PR reference stored in the task parameters is correct:
/// for the FS backend the `pr_url` is the work directory path.
async fn assert_pr_url_points_to_branch(
    env: &IntegrationTestEnv,
    task: &zbobr_dispatcher::Task,
    expected_branch: &str,
) {
    let pr_url = task
        .parameters
        .get(&Parameter::PrUrl)
        .unwrap_or_else(|| panic!("[{}] pr_url not found in task parameters", env.name()));

    // For the GitHub backend the pr_url is an https:// URL — skip git checks.
    if pr_url.starts_with("http") {
        return;
    }

    let pr_path = PathBuf::from(pr_url);
    assert!(
        pr_path.join(".git").exists(),
        "[{}] pr_url '{}' is not a git repository",
        env.name(),
        pr_url
    );

    let current = git_output(&pr_path, &["branch", "--show-current"]).await;
    assert_eq!(
        current.trim(),
        expected_branch,
        "[{}] pr_url '{}' is not on the expected branch",
        env.name(),
        pr_url
    );
}

/// Verify the work branch in the pr_url directory has at least one commit
/// ahead of `origin/main`.
async fn assert_pr_has_commits(
    env: &IntegrationTestEnv,
    task: &zbobr_dispatcher::Task,
    dest_branch: &str,
) {
    let pr_url = match task.parameters.get(&Parameter::PrUrl) {
        Some(u) => u,
        None => return,
    };

    if pr_url.starts_with("http") {
        assert_github_pr_has_commits(env, pr_url, dest_branch).await;
        return;
    }

    let pr_path = PathBuf::from(pr_url);
    let log = git_output(
        &pr_path,
        &["log", &format!("origin/{}..HEAD", dest_branch), "--oneline"],
    )
    .await;

    assert!(
        !log.trim().is_empty(),
        "[{}] pr_url '{}' work branch has no commits ahead of origin/{} — expected at least one",
        env.name(),
        pr_url,
        dest_branch
    );
}

async fn assert_github_pr_has_commits(env: &IntegrationTestEnv, pr_url: &str, dest_branch: &str) {
    let parts: Vec<&str> = pr_url.trim_end_matches('/').splitn(8, '/').collect();
    assert!(
        parts.len() >= 7 && parts[5] == "pull",
        "[{}] Cannot parse GitHub PR URL: {pr_url}",
        env.name()
    );
    let (owner, repo, pr_number) = (parts[3], parts[4], parts[6]);

    let api_path = format!("repos/{owner}/{repo}/pulls/{pr_number}");
    let out = tokio::process::Command::new("gh")
        .args(["api", &api_path])
        .output()
        .await
        .unwrap_or_else(|e| panic!("[{}] failed to run `gh api`: {e}", env.name()));

    assert!(
        out.status.success(),
        "[{}] `gh api {api_path}` failed:\n{}",
        env.name(),
        String::from_utf8_lossy(&out.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("[{}] failed to parse gh api response: {e}", env.name()));

    let commits = json["commits"].as_u64().unwrap_or(0);
    assert!(
        commits > 0,
        "[{}] GitHub PR {pr_url} has 0 commits — expected at least one",
        env.name()
    );

    let base_ref = json["base"]["ref"].as_str().unwrap_or("unknown");
    assert_eq!(
        base_ref,
        dest_branch,
        "[{}] GitHub PR {pr_url} targets branch '{}', expected '{dest_branch}'",
        env.name(),
        base_ref
    );
}

// ---------------------------------------------------------------------------
// Entry signal / conflict clearing tests
// ---------------------------------------------------------------------------

/// Rule 1: entering a non-Merger stage clears the triggering signal.
///
/// Strategy: pre-set GoReview (a "wrong" signal) on a Working task, then run
/// the Worker with an unchecked checklist item.  If the signal is cleared on
/// entry, Rule 2.3 fires and sets GoWork (has_unchecked=true).  If the signal
/// were *not* cleared, Rule 2 would see `signal.is_some()` and preserve the
/// pre-set GoReview instead.
pub async fn run_entry_clears_signal_for_worker(env: &IntegrationTestEnv) {
    let repo_path = env.create_git_repo("repo_entry_clear_worker").await;
    let dest_repo = env
        .target_repo
        .as_deref()
        .map(|r| format!("https://github.com/{r}"))
        .unwrap_or_else(|| repo_path.to_string_lossy().to_string());
    let task_id = env
        .create_task(
            "Entry Clear Worker",
            "Dummy task description",
            Stage::Working,
        )
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-entry-test");
    env.update_task_branches(task_id, &dest_repo, "main", &work_branch)
        .await;

    // Pre-set a GoReview signal — the Worker should clear this on entry.
    env.update_task_signal(task_id, "go_review").await;

    env.run_stage(
        task_id,
        Stage::Working,
        scenarios::working_scenario_with_unchecked_item(),
    )
    .await;

    let task = env.get_task(task_id).await;
    assert_eq!(
        task.signal,
        Some(Signal::GoWork),
        "[{}] Entry should have cleared GoReview; exit with unchecked item should set GoWork",
        env.name()
    );
}

/// Rule 1: entering Merging clears the conflict flag but NOT the signal.
///
/// Verify that after a Merger session:
///   - conflict == false (cleared on entry)
///   - signal == Some(GoWork) (preserved; Merger entry must not clear it)
pub async fn run_entry_clears_conflict_preserves_signal_for_merger(
    env: &IntegrationTestEnv,
) {
    let repo_path = env.create_git_repo("repo_entry_clear_merger").await;
    let dest_repo = env
        .target_repo
        .as_deref()
        .map(|r| format!("https://github.com/{r}"))
        .unwrap_or_else(|| repo_path.to_string_lossy().to_string());
    let repo_name = dest_repo
        .rsplit('/')
        .next()
        .unwrap_or(&dest_repo)
        .to_string();
    let task_id = env
        .create_task(
            "Entry Clear Merger",
            "Dummy task description",
            Stage::Merging,
        )
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-merger-entry-test");
    env.update_task_branches(task_id, &dest_repo, "main", &work_branch)
        .await;

    if let Some(target) = env.target_repo.as_deref() {
        env.prepare_workspace_via_repo_backend(task_id, target, &work_branch)
            .await;
    } else {
        env.prepare_workspace(task_id, &repo_path, &work_branch)
            .await;
    }

    // Add a placeholder commit so the PR can be created.
    let work_dir = env
        .workspaces_dir
        .join(format!("task#{task_id}"))
        .join(&repo_name);
    write_and_commit(
        &work_dir,
        "ZBOBR_PLACEHOLDER.md",
        &format!("placeholder for task #{task_id}\n"),
        "chore: add placeholder for PR",
    )
    .await;

    // Set both conflict=true and GoWork signal — Merger must clear conflict only.
    env.set_task_conflict(task_id, true).await;
    env.update_task_signal(task_id, "go_work").await;

    env.run_stage(
        task_id,
        Stage::Merging,
        scenarios::merging_scenario("report"),
    )
    .await;

    let task = env.get_task(task_id).await;
    assert!(
        !task.conflict,
        "[{}] Entry to Merging must clear the conflict flag",
        env.name()
    );
    assert_eq!(
        task.signal,
        Some(Signal::GoWork),
        "[{}] Entry to Merging must NOT clear the signal",
        env.name()
    );
}

/// Rule 2.2: Planner exit sets GoWork when no signal is already set.
///
/// Runs Planning from a clean state (no pre-set signal) and verifies that
/// GoWork is emitted afterwards.  This is a focused test for the fix to the
/// Planner exit path (it previously emitted no signal at all).
pub async fn run_planner_sets_go_work_on_exit(env: &IntegrationTestEnv) {
    let repo_path = env.create_git_repo("repo_planner_exit").await;
    let dest_repo = env
        .target_repo
        .as_deref()
        .map(|r| format!("https://github.com/{r}"))
        .unwrap_or_else(|| repo_path.to_string_lossy().to_string());
    let task_id = env
        .create_task(
            "Planner Exit Test",
            "Dummy task description",
            Stage::Preparing,
        )
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-test");
    env.update_task_branches(task_id, &dest_repo, "main", &work_branch)
        .await;

    // No signal pre-set; Planner should emit GoWork on exit.
    env.run_stage(task_id, Stage::Planning, scenarios::planning_scenario())
        .await;

    let task = env.get_task(task_id).await;
    assert_eq!(
        task.signal,
        Some(Signal::GoWork),
        "[{}] Planner exit (Rule 2.2) must set GoWork when no signal is already present",
        env.name()
    );
}

/// Rule 2: if the agent already set a signal, the exit logic must not override it.
///
/// Uses the Preparator with `ask_planner` (which sets GoPlan signal via the
/// retry mechanism) to verify that, even though Preparator's default exit
/// signal is also GoPlan, the pre-set signal is respected and preserved.
///
/// A more direct test: pre-set a higher-priority signal (GoPrepare) on a
/// Planning task.  If Rule 2 works, GoPrepare is preserved; if the exit logic
/// erroneously overrides it, GoWork appears instead.
pub async fn run_exit_preserves_agent_set_signal(env: &IntegrationTestEnv) {
    let repo_path = env.create_git_repo("repo_exit_preserve").await;
    let dest_repo = env
        .target_repo
        .as_deref()
        .map(|r| format!("https://github.com/{r}"))
        .unwrap_or_else(|| repo_path.to_string_lossy().to_string());
    let task_id = env
        .create_task(
            "Exit Preserve Test",
            "Dummy task description",
            Stage::Preparing,
        )
        .await;
    let work_branch = format!("zbobr_fix-{task_id}-exit-preserve");
    env.update_task_branches(task_id, &dest_repo, "main", &work_branch)
        .await;

    // Use planning_scenario which calls report_results — it does NOT set a
    // signal.  We manually pre-set GoPrepare AFTER the stage entry so it
    // simulates the agent having set it mid-session.
    //
    // To keep this test self-contained (and avoid the need for a custom
    // scenario that calls a signal-setting MCP tool), we instead rely on the
    // fact that report_error sets the retry signal.  Run the planning stage
    // with a report_error scenario and check that GoPlan (the Planner's retry
    // signal) is set, NOT GoWork (the normal Planner exit signal).
    env.run_stage(
        task_id,
        Stage::Planning,
        scenarios::planning_report_error_scenario(),
    )
    .await;

    let task = env.get_task(task_id).await;
    assert_eq!(
        task.signal,
        Some(Signal::GoPlan),
        "[{}] Agent-set signal (GoPlan via report_error retry) must not be overridden by exit logic",
        env.name()
    );
    assert!(
        task.pause,
        "[{}] report_error must still set the pause flag",
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
