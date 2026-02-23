mod mcp_integration_internal;

use mcp_integration_internal::{
    setup_test_env,
    create_test_repo,
    test_preparation,
    test_planning,
    verify_planning,
};

#[tokio::test]
async fn test_preparation_and_planning() {
    let Some(env) = setup_test_env().await else {
        return;
    };

    // high-level orchestration only; details live in helper functions above
    let repo_path = create_test_repo(&env).await;
    test_preparation(&env, &repo_path).await;
    test_planning(&env).await;
    verify_planning(&env).await;
}
