use rmcp::handler::server::router::tool::ToolRouter;
use serde_json::Value;

use crate::{
    ZbobrDispatcher,
    backend::TaskBackend,
    task::{Model, Role, Tool},
};

// Custom deserializer for boolean that accepts both bool and string values
// This handles cases where HTTP clients stringify all parameters
fn deserialize_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Deserialize};

    #[derive(serde::Deserialize)]
    #[serde(untagged)]
    enum BoolOrString {
        Bool(bool),
        String(String),
    }

    match BoolOrString::deserialize(deserializer)? {
        BoolOrString::Bool(b) => Ok(b),
        BoolOrString::String(s) => match s.to_lowercase().as_str() {
            "true" | "1" | "yes" => Ok(true),
            "false" | "0" | "no" => Ok(false),
            _ => Err(de::Error::custom(format!("invalid bool value: {}", s))),
        },
    }
}

// Instruction shared across all role prompts explaining branch isolation rules.
pub fn branch_isolation_instruction() -> String {
    use planner_tools::{GET_PARAM_DESTINATION_BRANCH, GET_PARAM_WORK_BRANCH};
    format!(
        "Workspace branch isolation. Your working directory is already the repository with the \
        work branch checked out. Use ONLY the destination and work branches with names provided \
        by the MCP tools `{GET_PARAM_DESTINATION_BRANCH}`, `{GET_PARAM_WORK_BRANCH}`. \
        Do not make changes in the destination branch: this is for reference only. \
        Do NOT fetch or use any other branches. Do NOT look at branches other than the work \
        and destination branches. If you need temporary or experimental branches, prefix their \
        names with the work branch name to avoid interfering with other agents.",
    )
}

/// Get the current hostname, or "unknown" if it cannot be determined.
pub fn get_hostname() -> String {
    hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown".to_string())
}

// -- Parameter types --

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct DescriptionParam {
    #[schemars(description = "The task description/plan text")]
    pub description: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct MessageParam {
    #[schemars(description = "The message to post")]
    pub message: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct RepoParam {
    #[schemars(description = "Target repository (full git URL, local path, or owner/repo)")]
    pub repo: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct PathParam {
    #[schemars(description = "Local filesystem path to repository")]
    pub path: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct ShortNameParam {
    #[schemars(description = "Short name for the branch (e.g. 'implementation', 'fix-typo')")]
    pub short_name: String,
}

// -- Checklist parameter types --

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct InsertChecklistItemParam {
    #[schemars(description = "Unique identifier for the new checklist item")]
    pub id: String,
    #[schemars(description = "Checklist item text")]
    pub text: String,
    #[schemars(description = "Optional ID of the item to insert after (if omitted, adds to end)")]
    pub after_id: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct UpdateChecklistItemParam {
    #[schemars(description = "ID of the checklist item to update")]
    pub id: String,
    #[schemars(description = "New text for the checklist item")]
    pub text: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct CheckChecklistItemParam {
    #[schemars(description = "ID of the checklist item to check/uncheck")]
    pub id: String,
    #[schemars(description = "New checkbox state (true = checked, false = unchecked)")]
    #[serde(deserialize_with = "deserialize_bool")]
    pub checked: bool,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct DeleteChecklistItemParam {
    #[schemars(description = "ID of the checklist item to delete")]
    pub id: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct SetDestinationRepositoryParam {
    #[schemars(
        description = "Destination repository (full git URL, local path, or owner/repo format) (or null to unset)"
    )]
    pub value: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct SetDestinationBranchParam {
    #[schemars(
        description = "Work branch postfix (the final segment after prefix/task_id) (or null to unset)"
    )]
    pub value: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct SetWorkBranchParam {
    #[schemars(description = "Work branch name (or null to unset)")]
    pub value: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct GetHistoryParam {
    #[schemars(
        description = "History chunk index (0 = oldest, omitted = latest). Response includes current_chunk and last_chunk for navigation."
    )]
    pub offset: Option<usize>,
}

macro_rules! mcp_tools {
    ($mod_name:ident, $($name:ident = $val:expr),* $(,)?) => {
        pub mod $mod_name {
            $(pub const $name: &str = $val;)*
            pub const ALL_TOOLS: &[&str] = &[$($val),*];
        }
    }
}

mcp_tools! {
    preparator_tools,
    GET_HISTORY = "get_history",
    REPORT_ERROR = "report_error",
    SET_PARAM_DESTINATION_REPOSITORY = "set_param_destination_repository",
    SET_PARAM_DESTINATION_BRANCH = "set_param_destination_branch",
    SET_PARAM_WORK_BRANCH_POSTFIX = "set_param_work_branch_postfix",
    GET_PARAM_DESTINATION_REPOSITORY = "get_param_destination_repository",
    GET_PARAM_DESTINATION_BRANCH = "get_param_destination_branch",
    GET_PARAM_WORK_BRANCH = "get_param_work_branch",
    REPORT_RESULTS = "report_results",
}

mcp_tools! {
    planner_tools,
    GET_HISTORY = "get_history",
    POST_PLAN = "post_plan",
    GET_CHECKLIST = "get_checklist",
    INSERT_CHECKLIST_ITEM = "insert_checklist_item",
    UPDATE_CHECKLIST_ITEM = "update_checklist_item",
    DELETE_CHECKLIST_ITEM = "delete_checklist_item",
    REPORT_ERROR = "report_error",
    ASK_USER = "ask_user",
    GET_PARAM_DESTINATION_BRANCH = "get_param_destination_branch",
    GET_PARAM_WORK_BRANCH = "get_param_work_branch",
}

mcp_tools! {
    worker_tools,
    GET_HISTORY = "get_history",
    REPORT_ERROR = "report_error",
    ASK_USER = "ask_user",
    ASK_PLANNER = "ask_planner",
    GET_CHECKLIST = "get_checklist",
    INSERT_CHECKLIST_ITEM = "insert_checklist_item",
    UPDATE_CHECKLIST_ITEM = "update_checklist_item",
    CHECK_CHECKLIST_ITEM = "check_checklist_item",
    DELETE_CHECKLIST_ITEM = "delete_checklist_item",
    GET_PARAM_DESTINATION_BRANCH = "get_param_destination_branch",
    GET_PARAM_WORK_BRANCH = "get_param_work_branch",
    REPORT_RESULTS = "report_results",
}

mcp_tools! {
    reviewer_tools,
    GET_HISTORY = "get_history",
    REPORT_ERROR = "report_error",
    GET_PARAM_DESTINATION_BRANCH = "get_param_destination_branch",
    GET_PARAM_WORK_BRANCH = "get_param_work_branch",
    REVIEW_ACCEPT = "review_accept",
    REVIEW_REJECT = "review_reject",
}

mcp_tools! {
    tester_tools,
    GET_HISTORY = "get_history",
    REPORT_ERROR = "report_error",
    GET_PARAM_DESTINATION_BRANCH = "get_param_destination_branch",
    GET_PARAM_WORK_BRANCH = "get_param_work_branch",
    TEST_ACCEPT = "test_accept",
    TEST_REJECT = "test_reject",
}

mcp_tools! {
    merger_tools,
    GET_HISTORY = "get_history",
    REPORT_ERROR = "report_error",
    ASK_USER = "ask_user",
    GET_PARAM_DESTINATION_BRANCH = "get_param_destination_branch",
    GET_PARAM_WORK_BRANCH = "get_param_work_branch",
    REPORT_RESULTS = "report_results",
}

/// Generate concise API documentation from a tool router
pub(crate) fn generate_api_docs_from_router<T: Send + Sync + 'static>(
    router: &ToolRouter<T>,
    role_name: &str,
) -> String {
    let tools = router.list_all();

    let mut doc = format!("## {} MCP API\n\n", role_name);
    doc.push_str("Available tools (all pre-scoped to your task):\n\n");

    for tool in tools {
        doc.push_str(&format!("### `{}`\n\n", tool.name));
        doc.push_str(&format!(
            "{}\n\n",
            tool.description.as_deref().unwrap_or("No description")
        ));

        // Parameters
        let schema = &tool.input_schema;
        let properties_obj = schema.get("properties").and_then(|v: &Value| v.as_object());

        if let Some(properties) = properties_obj {
            if !properties.is_empty() {
                doc.push_str("**Parameters:**\n");
                for (name, prop_val) in properties {
                    let required_arr = schema.get("required").and_then(|v: &Value| v.as_array());
                    let required = required_arr
                        .map(|arr| {
                            arr.iter()
                                .any(|v: &Value| v.as_str() == Some(name.as_str()))
                        })
                        .unwrap_or(false);
                    let desc = match prop_val.get("description") {
                        Some(v) => v.as_str().unwrap_or(""),
                        None => "",
                    };
                    let type_str = match prop_val.get("type") {
                        Some(v) => v.as_str().unwrap_or("any"),
                        None => "any",
                    };
                    doc.push_str(&format!(
                        "- `{}` ({}{}) - {}\n",
                        name,
                        type_str,
                        if required { ", required" } else { "" },
                        desc
                    ));
                }
                doc.push('\n');
            } else {
                doc.push_str("**Parameters:** None\n\n");
            }
        } else {
            doc.push_str("**Parameters:** None\n\n");
        }

        doc.push_str("---\n\n");
    }

    doc
}

/// Bind to an available port starting from the given base port.
///
/// The old `find_available_port` implementation closed the listener immediately
/// after opening it, which raced with concurrent tests: if two processes both
/// scanned the same candidate port, the first might drop the listener before the
/// second tried to bind and then both would use the same port.  Returning the
/// actual listener guarantees the socket remains reserved until the server is
/// started.
///
/// The returned tuple contains the port number and the bound `TcpListener`.
pub(crate) async fn bind_available_port(
    base_port: u16,
) -> anyhow::Result<(u16, tokio::net::TcpListener)> {
    for port in base_port..=base_port + 100 {
        match tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await {
            Ok(listener) => {
                let actual = listener.local_addr()?.port();
                return Ok((actual, listener));
            }
            Err(_) => continue,
        }
    }
    anyhow::bail!(
        "Could not find available port in range {base_port}..{}",
        base_port + 100
    )
}

pub(crate) async fn serve_mcp(
    base_port: u16,
    path: &str,
    router: axum::Router,
) -> anyhow::Result<u16> {
    // Bind the listener up front to avoid a check-then-bind race when tests run
    // concurrently.  `bind_available_port` returns both the port and the
    // already-bound listener.
    let (port, listener) = bind_available_port(base_port).await?;
    tracing::info!("MCP server listening on http://127.0.0.1:{port}{path}");

    // Spawn the actual server in a background task
    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, router)
            .with_graceful_shutdown(async {
                tokio::signal::ctrl_c().await.ok();
            })
            .await
        {
            tracing::error!("Axum server error: {e}");
        }
    });

    Ok(port)
}

/// Run the MCP HTTP server scoped to a role (planner or worker) and task.
/// Returns the actual port that was assigned (spawns server in background).
pub async fn run_role_mcp_server(
    zbobr: ZbobrDispatcher,
    task_backend: std::sync::Arc<dyn TaskBackend>,
    role: Role,
    task_id: u64,
    tool: Tool,
    model: Model,
    stage_name: String,
) -> anyhow::Result<u16> {
    let base_port = zbobr.config().base_port;
    use rmcp::transport::streamable_http_server::{
        StreamableHttpService, session::local::LocalSessionManager,
    };

    let path = format!("/{}/{}", role, task_id);

    let router = match role {
        Role::Preparator => {
            tracing::info!("Creating PreparatorMcp service for task {task_id} at path {path}");
            let svc = StreamableHttpService::new(
                move || {
                    tracing::debug!("Creating new PreparatorMcp instance for task {task_id}");
                    Ok(super::preparator::PreparatorMcp::new(
                        zbobr.clone(),
                        task_backend.clone(),
                        task_id,
                        tool,
                        model.clone(),
                        stage_name.clone(),
                    ))
                },
                std::sync::Arc::new(LocalSessionManager::default()),
                Default::default(),
            );
            axum::Router::new().nest_service(&path, svc)
        }
        Role::Planner => {
            tracing::info!("Creating PlannerMcp service for task {task_id} at path {path}");
            let svc = StreamableHttpService::new(
                move || {
                    tracing::debug!("Creating new PlannerMcp instance for task {task_id}");
                    Ok(super::planner::PlannerMcp::new(
                        zbobr.clone(),
                        task_backend.clone(),
                        task_id,
                        tool,
                        model.clone(),
                        stage_name.clone(),
                    ))
                },
                std::sync::Arc::new(LocalSessionManager::default()),
                Default::default(),
            );
            axum::Router::new().nest_service(&path, svc)
        }
        Role::Worker => {
            tracing::info!("Creating WorkerMcp service for task {task_id} at path {path}");
            let svc = StreamableHttpService::new(
                move || {
                    tracing::debug!("Creating new WorkerMcp instance for task {task_id}");
                    Ok(super::worker::WorkerMcp::new(
                        zbobr.clone(),
                        task_backend.clone(),
                        task_id,
                        tool,
                        model.clone(),
                        stage_name.clone(),
                    ))
                },
                std::sync::Arc::new(LocalSessionManager::default()),
                Default::default(),
            );
            axum::Router::new().nest_service(&path, svc)
        }
        Role::Reviewer => {
            tracing::info!("Creating ReviewerMcp service for task {task_id} at path {path}");
            let svc = StreamableHttpService::new(
                move || {
                    tracing::debug!("Creating new ReviewerMcp instance for task {task_id}");
                    Ok(super::reviewer::ReviewerMcp::new(
                        zbobr.clone(),
                        task_backend.clone(),
                        task_id,
                        tool,
                        model.clone(),
                        stage_name.clone(),
                    ))
                },
                std::sync::Arc::new(LocalSessionManager::default()),
                Default::default(),
            );
            axum::Router::new().nest_service(&path, svc)
        }
        Role::Tester => {
            tracing::info!("Creating TesterMcp service for task {task_id} at path {path}");
            let svc = StreamableHttpService::new(
                move || {
                    tracing::debug!("Creating new TesterMcp instance for task {task_id}");
                    Ok(super::tester::TesterMcp::new(
                        zbobr.clone(),
                        task_backend.clone(),
                        task_id,
                        tool,
                        model.clone(),
                        stage_name.clone(),
                    ))
                },
                std::sync::Arc::new(LocalSessionManager::default()),
                Default::default(),
            );
            axum::Router::new().nest_service(&path, svc)
        }
        Role::Merger => {
            tracing::info!("Creating MergerMcp service for task {task_id} at path {path}");
            let svc = StreamableHttpService::new(
                move || {
                    tracing::debug!("Creating new MergerMcp instance for task {task_id}");
                    Ok(super::merger::MergerMcp::new(
                        zbobr.clone(),
                        task_backend.clone(),
                        task_id,
                        tool,
                        model.clone(),
                        stage_name.clone(),
                    ))
                },
                std::sync::Arc::new(LocalSessionManager::default()),
                Default::default(),
            );
            axum::Router::new().nest_service(&path, svc)
        }
    };

    serve_mcp(base_port, &path, router).await
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    // exercise port binder under concurrent load – each spawned task should
    // return a distinct port and hold onto the listener until the end of the
    // test, preventing races where two callers pick the same port.
    #[tokio::test]
    async fn bind_available_port_concurrent() {
        const BASE: u16 = 9000;

        let handles: Vec<_> = (0..10)
            .map(|_| tokio::spawn(async move { bind_available_port(BASE).await }))
            .collect();

        let mut entries = Vec::new();
        for h in handles {
            let (port, listener) = h.await.expect("task panicked").unwrap();
            entries.push((port, listener));
        }

        let ports: HashSet<_> = entries.iter().map(|(p, _)| *p).collect();
        assert_eq!(ports.len(), entries.len(), "ports must all be unique");
        // listeners are dropped when `entries` goes out of scope
    }
}
