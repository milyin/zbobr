mod mcp_integration_internal;

use mcp_integration_internal::{
    IntegrationTestEnv,
    preparation_scenario,
    planning_scenario,
};
use zbobr_dispatcher::Stage;

/// Test the preparation stage in isolation.
///
/// Creates its own task and git repo, runs the preparator scenario, and
/// verifies that the stage completes successfully.
#[tokio::test]
async fn test_preparation() {
    let Some(env) = IntegrationTestEnv::get().await else {
        return;
    };

    let repo_path = env.create_git_repo("repo_preparation").await;
    let task_id = env
        .create_task("Dummy Task", "Dummy task description", Stage::Preparation)
        .await;
    env.run_preparation(preparation_scenario(&repo_path.to_string_lossy()), task_id).await;
}

/// Test the planning stage end-to-end.
///
/// Uses preparation helpers to set up the task (giving the planner the
/// required repository parameters), then runs the planner scenario and
/// verifies the resulting workspace clone.
#[tokio::test]
async fn test_planning() {
    let Some(env) = IntegrationTestEnv::get().await else {
        return;
    };

    let repo_path = env.create_git_repo("repo_planning").await;
    let task_id = env
        .create_task("Dummy Task", "Dummy task description", Stage::Preparation)
        .await;

    // Run preparation first so that the task has the required parameters
    // (destination repository, destination branch, work-branch postfix).
    env.run_preparation(preparation_scenario(&repo_path.to_string_lossy()), task_id).await;

    // Now run the planning stage.
    env.run_planning(planning_scenario(), task_id).await;

    env.verify_planning(task_id).await;
}
