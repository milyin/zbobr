mod mcp_integration;

use mcp_integration::IntegrationTestEnv;
use zbobr_dispatcher::Stage;

fn common_merging_steps() -> String {
    format!(
        r#"name: Merger Comprehensive Test
description: Verify core MERGING MCP functions
timeout: 60
stop_on_failure: true

steps:
- name: Get task description
  operation:
    type: tool_call
    tool: {GET_DESCRIPTION}
  assertions:
    - type: success
    - type: contains
      path: result
      value: "Dummy task description"

- name: Get task discussion
  operation:
    type: tool_call
    tool: {GET_DISCUSSION}
  assertions:
    - type: success

- name: Get destination branch
  operation:
    type: tool_call
    tool: {GET_PARAM_DESTINATION_BRANCH}
  assertions:
    - type: success
    - type: equals
      path: result
      value: "main"

- name: Get work branch
  operation:
    type: tool_call
    tool: {GET_PARAM_WORK_BRANCH}
  assertions:
    - type: success
    - type: contains
      path: result
      value: "test"
"#,
        GET_DESCRIPTION = zbobr_dispatcher::mcp::merger_tools::GET_DESCRIPTION,
        GET_DISCUSSION = zbobr_dispatcher::mcp::merger_tools::GET_DISCUSSION,
        GET_PARAM_DESTINATION_BRANCH = zbobr_dispatcher::mcp::merger_tools::GET_PARAM_DESTINATION_BRANCH,
        GET_PARAM_WORK_BRANCH = zbobr_dispatcher::mcp::merger_tools::GET_PARAM_WORK_BRANCH,
    )
}

fn merging_ending_steps(ending: &str) -> String {
    match ending {
        "report" => format!(
            r#"
- name: Report results and finish
  operation:
    type: tool_call
    tool: {REPORT_RESULTS}
    arguments:
      message: "Merger complete."
  assertions:
    - type: success
"#,
            REPORT_RESULTS = zbobr_dispatcher::mcp::merger_tools::REPORT_RESULTS,
        ),
        "ask" => format!(
            r#"
- name: Ask user
  operation:
    type: tool_call
    tool: {ASK_USER}
    arguments:
      message: "Need guidance on merge"
  assertions:
    - type: success
"#,
            ASK_USER = zbobr_dispatcher::mcp::merger_tools::ASK_USER,
        ),
        _ => panic!("unknown ending"),
    }
}

fn merging_scenario(ending: &str) -> String {
    format!("{}{}", common_merging_steps(), merging_ending_steps(ending))
}

async fn run_merging_test(env: &IntegrationTestEnv) {
    let repo_path = env.create_git_repo("repo_merging").await;
    let repo_path_str = repo_path.to_string_lossy().to_string();

    let scenario_report = merging_scenario("report");
    let scenario_ask = merging_scenario("ask");

    // Test report ending
    let task_id_report = env
        .create_task("Dummy Task", "Dummy task description", Stage::Merging)
        .await;
    let work_branch_report = format!("zbobr_fix-{task_id_report}-test");
    env.update_task_branches(task_id_report, &repo_path_str, "main", &work_branch_report).await;
    env.prepare_workspace(task_id_report, &repo_path, &work_branch_report).await;

    env.run_stage(task_id_report, Stage::Merging, scenario_report.clone()).await;
    let output = env.show_task(task_id_report).await;
    assert!(
        output.contains("Merger complete."),
        "[{}] Merger report message was not recorded in discussion",
        env.backend_name()
    );
    // In the new model, merger does not set a follow-up signal;
    // the original signal is preserved through the merger session.
    // Since this test starts with no signal, signal should be (none) after merger.
    assert!(
        output.contains("Signal:      (none)"),
        "[{}] Merger should not set a follow-up signal (original signal preserved)",
        env.backend_name()
    );

    let output = env.show_task(task_id_report).await;
    assert!(
        output.contains("Merger complete."),
        "[{}] Merger report message was not recorded in discussion",
        env.backend_name()
    );
    assert!(
        output.contains("Signal:      (none)"),
        "[{}] Merger should not set a follow-up signal (original signal preserved)",
        env.backend_name()
    );

    // verify the work directory exists and is set up correctly
    let cloned_repo_path_report = env.workspaces_dir
        .join(format!("task#{task_id_report}"))
        .join("repo_merging");

    assert!(
        cloned_repo_path_report.exists(),
        "[{}] Work directory does not exist",
        env.backend_name()
    );
    assert!(
        cloned_repo_path_report.join(".git").exists(),
        "[{}] Work directory is not a git repository",
        env.backend_name()
    );

    let branches_output = tokio::process::Command::new("git")
        .arg("branch")
        .current_dir(&cloned_repo_path_report)
        .output()
        .await
        .unwrap();
    let branches_str = String::from_utf8_lossy(&branches_output.stdout);

    assert!(
        branches_str.contains("main"),
        "[{}] Destination branch 'main' not found in cloned repo",
        env.backend_name()
    );
    assert!(
        branches_str.contains(work_branch_report.as_str()),
        "[{}] Work branch '{work_branch_report}' not found in cloned repo",
        env.backend_name()
    );

    let current_branch_output = tokio::process::Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(&cloned_repo_path_report)
        .output()
        .await
        .unwrap();
    let current_branch = String::from_utf8_lossy(&current_branch_output.stdout)
        .trim()
        .to_string();

    assert_eq!(
        current_branch, work_branch_report,
        "[{}] Current branch is not the work branch",
        env.backend_name()
    );

    // Test ask ending
    let task_id_ask = env
        .create_task("Dummy Task", "Dummy task description", Stage::Merging)
        .await;
    let work_branch_ask = format!("zbobr_fix-{task_id_ask}-test");
    env.update_task_branches(task_id_ask, &repo_path_str, "main", &work_branch_ask).await;
    env.prepare_workspace(task_id_ask, &repo_path, &work_branch_ask).await;

    env.run_stage(task_id_ask, Stage::Merging, scenario_ask.clone()).await;
    let output = env.show_task(task_id_ask).await;
    assert!(
        output.contains("Need guidance on merge"),
        "[{}] Ask user message was not recorded in discussion",
        env.backend_name()
    );
    assert!(
        output.contains("Pause:       true"),
        "[{}] Merger ask_user should set pause flag instead of go_ask signal",
        env.backend_name()
    );

    let output = env.show_task(task_id_ask).await;
    assert!(
        output.contains("Need guidance on merge"),
        "[{}] Ask user message was not recorded in discussion",
        env.backend_name()
    );
    assert!(
        output.contains("Pause:       true"),
        "[{}] Merger ask_user should set pause flag instead of go_ask signal",
        env.backend_name()
    );
}

async fn run_merging_with_real_conflict_test(env: &IntegrationTestEnv) {
    let repo_path = env.create_git_repo("repo_merging_conflict").await;
    let repo_path_str = repo_path.to_string_lossy().to_string();

    // Set up the repository with conflicting changes
    // Create a file with initial content on main
    tokio::fs::write(repo_path.join("conflict_file.txt"), "line1\nline2\nline3\n").await.unwrap();
    let git_add = tokio::process::Command::new("git")
        .args(["add", "conflict_file.txt"])
        .current_dir(&repo_path)
        .status()
        .await
        .unwrap();
    assert!(git_add.success());
    let git_commit = tokio::process::Command::new("git")
        .args(["commit", "-m", "Initial commit with conflict_file.txt"])
        .current_dir(&repo_path)
        .status()
        .await
        .unwrap();
    assert!(git_commit.success());

    // Create work branch and modify the file
    let work_branch = "work_branch_conflict";
    let git_branch = tokio::process::Command::new("git")
        .args(["checkout", "-b", work_branch])
        .current_dir(&repo_path)
        .status()
        .await
        .unwrap();
    assert!(git_branch.success());
    tokio::fs::write(repo_path.join("conflict_file.txt"), "line1\nline2 work\nline3\n").await.unwrap();
    let git_add_work = tokio::process::Command::new("git")
        .args(["add", "conflict_file.txt"])
        .current_dir(&repo_path)
        .status()
        .await
        .unwrap();
    assert!(git_add_work.success());
    let git_commit_work = tokio::process::Command::new("git")
        .args(["commit", "-m", "Work changes on conflict_file.txt"])
        .current_dir(&repo_path)
        .status()
        .await
        .unwrap();
    assert!(git_commit_work.success());

    // Switch back to main and make conflicting changes
    let git_checkout_main = tokio::process::Command::new("git")
        .args(["checkout", "main"])
        .current_dir(&repo_path)
        .status()
        .await
        .unwrap();
    assert!(git_checkout_main.success());
    tokio::fs::write(repo_path.join("conflict_file.txt"), "line1\nline2 main\nline3\n").await.unwrap();
    let git_add_main = tokio::process::Command::new("git")
        .args(["add", "conflict_file.txt"])
        .current_dir(&repo_path)
        .status()
        .await
        .unwrap();
    assert!(git_add_main.success());
    let git_commit_main = tokio::process::Command::new("git")
        .args(["commit", "-m", "Main changes on conflict_file.txt"])
        .current_dir(&repo_path)
        .status()
        .await
        .unwrap();
    assert!(git_commit_main.success());

    // In the new model, the conflict is detected by the dispatcher during pull,
    // not by the agent's pull_work MCP tool. Create a task at MERGING stage directly
    // and test that the merger can access the work directory with conflict markers.
    let task_id = env
        .create_task("Task with merge conflict", "Test merging with real conflicts", Stage::Merging)
        .await;
    env.update_task_branches(task_id, &repo_path_str, "main", work_branch).await;

    // Set up the workspace manually to simulate what the dispatcher would do:
    // clone the repo and create a merge conflict in the work directory
    let workspace_dir = env.workspaces_dir.join(format!("task#{task_id}"));
    tokio::fs::create_dir_all(&workspace_dir).await.unwrap();
    let work_dir = workspace_dir.join("repo_merging_conflict");

    // Copy the repo to the workspace
    let cp_status = tokio::process::Command::new("cp")
        .args(["-r", &repo_path_str, work_dir.to_str().unwrap()])
        .status()
        .await
        .unwrap();
    assert!(cp_status.success(), "[{}] Failed to copy repo to workspace", env.backend_name());

    // Checkout work branch and attempt merge to create conflict markers
    let checkout_status = tokio::process::Command::new("git")
        .args(["checkout", work_branch])
        .current_dir(&work_dir)
        .status()
        .await
        .unwrap();
    assert!(checkout_status.success());

    let merge_output = tokio::process::Command::new("git")
        .args(["merge", "main", "--no-edit"])
        .current_dir(&work_dir)
        .output()
        .await
        .unwrap();
    // Merge should fail with conflict
    assert!(
        !merge_output.status.success(),
        "[{}] Expected merge conflict",
        env.backend_name()
    );

    // Create a scenario for the merger that accesses the conflicted repo
    let conflict_scenario = format!(
        r#"name: Merger Conflict Resolution Test
description: Test handling of real merge conflicts
timeout: 60
stop_on_failure: true

steps:
- name: Get task description
  operation:
    type: tool_call
    tool: {GET_DESCRIPTION}
  assertions:
    - type: success

- name: Report conflict resolution
  operation:
    type: tool_call
    tool: {REPORT_RESULTS}
    arguments:
      message: "Detected merge conflicts in conflict_file.txt."
  assertions:
    - type: success
"#,
        GET_DESCRIPTION = zbobr_dispatcher::mcp::merger_tools::GET_DESCRIPTION,
        REPORT_RESULTS = zbobr_dispatcher::mcp::merger_tools::REPORT_RESULTS,
    );

    // Run the merging stage
    env.run_stage(task_id, Stage::Merging, conflict_scenario).await;

    // Check that the merger was called and handled the conflict
    let output = env.show_task(task_id).await;
    println!("[{}] Task output after merging:\n{}", env.backend_name(), output);
    assert!(
        output.contains("Detected merge conflicts"),
        "[{}] Merger should have detected and reported conflicts",
        env.backend_name()
    );
    // After merger, conflict flag should be cleared
    assert!(
        output.contains("Conflict:    false"),
        "[{}] Merger should clear the conflict flag",
        env.backend_name()
    );
}

#[tokio::test]
async fn test_merging() {
    let envs = IntegrationTestEnv::get_all().await;
    if envs.is_empty() {
        return;
    }
    for env in &envs {
        run_merging_test(env).await;
    }
}

#[tokio::test]
async fn test_merging_with_real_conflict() {
    let envs = IntegrationTestEnv::get_all().await;
    if envs.is_empty() {
        return;
    }
    for env in &envs {
        run_merging_with_real_conflict_test(env).await;
    }
}
