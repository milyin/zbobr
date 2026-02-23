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

- name: Pull work
  operation:
    type: tool_call
    tool: {PULL_WORK}
  store_result: pull_work_result
  assertions:
    - type: success
"#,
        GET_DESCRIPTION = zbobr_dispatcher::mcp::merger_tools::GET_DESCRIPTION,
        GET_DISCUSSION = zbobr_dispatcher::mcp::merger_tools::GET_DISCUSSION,
        GET_PARAM_DESTINATION_BRANCH = zbobr_dispatcher::mcp::merger_tools::GET_PARAM_DESTINATION_BRANCH,
        GET_PARAM_WORK_BRANCH = zbobr_dispatcher::mcp::merger_tools::GET_PARAM_WORK_BRANCH,
        PULL_WORK = zbobr_dispatcher::mcp::merger_tools::PULL_WORK,
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
      message: "Merger complete. PULL_WORK_RETURN_VALUE=${{pull_work_result}}"
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

#[tokio::test]
async fn test_merging() {
    let Some(env) = IntegrationTestEnv::get().await else {
        return;
    };

    let repo_path = env.create_git_repo("repo_merging").await;
    let repo_path_str = repo_path.to_string_lossy().to_string();

    let scenario_report = merging_scenario("report");
    let scenario_ask = merging_scenario("ask");
    let scenario_path_report = env.create_scenario("merging_report", &scenario_report).await;
    let scenario_path_ask = env.create_scenario("merging_ask", &scenario_ask).await;

    // Test report ending
    let task_id_report = env
        .create_task("Dummy Task", "Dummy task description", Stage::Merging)
        .await;
    let work_branch_report = format!("zbobr_fix-{task_id_report}-test");
    let task_id_str_report = task_id_report.to_string();
    env.update_task_branches(task_id_report, &repo_path_str, "main", &work_branch_report).await;

    // Test report ending explicit
    env.run_stage(task_id_report, Stage::Merging, scenario_report.clone()).await;
    let output = env.show_task(task_id_report).await;
    assert!(
        output.contains("Merger complete."),
        "Merger report message was not recorded in discussion"
    );
    assert!(
        output.contains("Signal:      go_work"),
        "Merger follow-up signal should be GO_WORK after merge resolution"
    );

    // Test report ending process
    // env.run_zbobr("task", &["update", &task_id_str_report, "--stage", "PENDING", "--signal", "go_merge"]).await;
    // env.run_zbobr("task", &["process", &task_id_str_report, "--executor-mcp-tester-merging", &scenario_path_report]).await;
    let output = env.show_task(task_id_report).await;
    assert!(
        output.contains("Merger complete."),
        "Merger report message was not recorded in discussion"
    );
    assert!(
        output.contains("Signal:      go_work"),
        "Merger follow-up signal should be GO_WORK after merge resolution"
    );

    // The pull_work checks for report
    let mut pull_work_return_value = None;
    for line in output.lines() {
        if let Some(idx) = line.find("PULL_WORK_RETURN_VALUE=") {
            let val = line[idx + "PULL_WORK_RETURN_VALUE=".len()..].trim();
            let val = val.trim_end_matches('\'');
            pull_work_return_value = Some(val.to_string());
            break;
        }
    }
    let pull_work_return_value =
        pull_work_return_value.expect("PULL_WORK_RETURN_VALUE not found in task output");

    let parsed: serde_json::Value = serde_json::from_str(&pull_work_return_value)
        .expect("Failed to parse PULL_WORK_RETURN_VALUE as JSON");
    let path_str = parsed
        .get("result")
        .and_then(|v| v.as_str())
        .expect("result field not found or not a string");

    let cloned_repo_path = std::path::PathBuf::from(path_str);
    assert!(cloned_repo_path.exists(), "Cloned repo path does not exist");
    assert!(
        cloned_repo_path.starts_with(&env.workspaces_dir),
        "Cloned repo path is not inside workspaces_dir"
    );
    assert!(
        cloned_repo_path.join(".git").exists(),
        "Cloned repo is not a git repository"
    );

    let branches_output = tokio::process::Command::new("git")
        .arg("branch")
        .current_dir(&cloned_repo_path)
        .output()
        .await
        .unwrap();
    let branches_str = String::from_utf8_lossy(&branches_output.stdout);

    assert!(
        branches_str.contains("main"),
        "Destination branch 'main' not found in cloned repo"
    );
    assert!(
        branches_str.contains(&work_branch_report),
        "Work branch '{work_branch_report}' not found in cloned repo"
    );

    let current_branch_output = tokio::process::Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(&cloned_repo_path)
        .output()
        .await
        .unwrap();
    let current_branch = String::from_utf8_lossy(&current_branch_output.stdout)
        .trim()
        .to_string();

    assert_eq!(
        current_branch, work_branch_report,
        "Current branch is not the work branch"
    );

    // Test ask ending
    let task_id_ask = env
        .create_task("Dummy Task", "Dummy task description", Stage::Merging)
        .await;
    let work_branch_ask = format!("zbobr_fix-{task_id_ask}-test");
    let task_id_str_ask = task_id_ask.to_string();
    env.update_task_branches(task_id_ask, &repo_path_str, "main", &work_branch_ask).await;

    // Test ask ending explicit
    env.run_stage(task_id_ask, Stage::Merging, scenario_ask.clone()).await;
    let output = env.show_task(task_id_ask).await;
    assert!(
        output.contains("Need guidance on merge"),
        "Ask user message was not recorded in discussion"
    );
    assert!(
        output.contains("Signal:      go_ask"),
        "Merger should set signal to GO_ASK when asking user"
    );

    // Test ask ending process
    // env.run_zbobr("task", &["update", &task_id_str_ask, "--stage", "PENDING", "--signal", "go_merge"]).await;
    // env.run_zbobr("task", &["process", &task_id_str_ask, "--executor-mcp-tester-merging", &scenario_path_ask]).await;
    let output = env.show_task(task_id_ask).await;
    assert!(
        output.contains("Need guidance on merge"),
        "Ask user message was not recorded in discussion"
    );
    assert!(
        output.contains("Signal:      go_ask"),
        "Merger should set signal to GO_ASK when asking user"
    );
}

#[tokio::test]
async fn test_merging_with_real_conflict() {
    let Some(env) = IntegrationTestEnv::get().await else {
        return;
    };

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

    // Now create a task for merging
    let task_id = env
        .create_task("Task with merge conflict", "Test merging with real conflicts", Stage::Merging)
        .await;
    env.update_task_branches(task_id, &repo_path_str, "main", work_branch).await;

    // Create a scenario that handles conflicts
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

- name: Pull work (should encounter conflicts)
  operation:
    type: tool_call
    tool: {PULL_WORK}
  store_result: pull_work_result
  assertions:
    - type: success

- name: Check for merge conflicts
  operation:
    type: tool_call
    tool: {REPORT_RESULTS}
    arguments:
      message: "Detected merge conflicts in conflict_file.txt. Resolving by choosing work changes."
  assertions:
    - type: success
"#,
        GET_DESCRIPTION = zbobr_dispatcher::mcp::merger_tools::GET_DESCRIPTION,
        PULL_WORK = zbobr_dispatcher::mcp::merger_tools::PULL_WORK,
        REPORT_RESULTS = zbobr_dispatcher::mcp::merger_tools::REPORT_RESULTS,
    );

    let scenario_path = env.create_scenario("merging_conflict", &conflict_scenario).await;

    // Run the merging stage
    env.run_stage(task_id, Stage::Merging, conflict_scenario).await;

    // Check that the merger was called and handled the conflict
    let output = env.show_task(task_id).await;
    assert!(
        output.contains("Detected merge conflicts"),
        "Merger should have detected and reported conflicts"
    );
    assert!(
        output.contains("Signal:      go_work"),
        "Merger should signal go_work after resolving conflicts"
    );

    // Verify the cloned repo has the resolved state
    let mut pull_work_path = None;
    for line in output.lines() {
        if let Some(idx) = line.find("PULL_WORK_RETURN_VALUE=") {
            let val = line[idx + "PULL_WORK_RETURN_VALUE=".len()..].trim();
            let val = val.trim_end_matches('\'');
            let parsed: serde_json::Value = serde_json::from_str(&val).unwrap();
            let path_str = parsed.get("result").unwrap().as_str().unwrap();
            pull_work_path = Some(std::path::PathBuf::from(path_str));
            break;
        }
    }
    let pull_work_path = pull_work_path.expect("PULL_WORK_RETURN_VALUE not found");

    // Check that the conflict was resolved (work branch should be checked out)
    let current_branch_output = tokio::process::Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(&pull_work_path)
        .output()
        .await
        .unwrap();
    let current_branch = String::from_utf8_lossy(&current_branch_output.stdout)
        .trim()
        .to_string();
    assert_eq!(current_branch, work_branch, "Should be on work branch");

    // Check that the conflict was detected (file should have conflict markers)
    let content = tokio::fs::read_to_string(pull_work_path.join("conflict_file.txt")).await.unwrap();
    assert!(
        content.contains("<<<<<<< HEAD") || content.contains("=======") || content.contains(">>>>>>> main"),
        "File should contain merge conflict markers. Content: {}",
        content
    );

    // And the task should have run the merger
    assert!(
        output.contains("Merger Conflict Resolution Test"),
        "Merger scenario should have been executed"
    );
}
