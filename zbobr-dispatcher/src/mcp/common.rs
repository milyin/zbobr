use std::collections::HashSet;

use crate::{
    ZbobrDispatcher,
    task::{Model, RoleSession, Tool},
};

/// Get the current hostname, or "unknown" if it cannot be determined.
pub fn get_hostname() -> String {
    hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown".to_string())
}

// -- Parameter types --

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct MessageParam {
    #[schemars(description = "The message to post")]
    pub message: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct GetHistoryRecordParam {
    #[schemars(description = "Position index of the record to retrieve (0 = task description)")]
    pub index: usize,
}

// -- Worktree configuration --

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct ConfigureWorktreeParam {
    #[schemars(description = "Destination repository (full git URL, local path, or owner/repo format)")]
    pub destination_repository: Option<String>,
    #[schemars(description = "Destination branch name (e.g. 'main')")]
    pub destination_branch: Option<String>,
    #[schemars(description = "Work branch postfix (e.g. 'implement-feature'). Combined with prefix and task ID to form the full branch name.")]
    pub work_branch_postfix: Option<String>,
}

// -- Checklist parameter types --

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct AddChecklistItemParam {
    #[schemars(description = "Unique identifier for the new checklist item")]
    pub id: String,
    #[schemars(description = "Checklist item text")]
    pub text: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct CheckChecklistItemParam {
    #[schemars(description = "ID of the checklist item to mark as checked")]
    pub id: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct DeleteChecklistItemParam {
    #[schemars(description = "ID of the checklist item to delete")]
    pub id: String,
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

/// Run the MCP HTTP server scoped to a role and task.
/// Returns the actual port that was assigned (spawns server in background).
pub async fn run_role_mcp_server(
    zbobr: std::sync::Arc<ZbobrDispatcher>,
    role_name: &str,
    task_id: u64,
    tool: Tool,
    model: Model,
    stage_name: String,
    allowed_tools: HashSet<String>,
    tool_tracker: std::sync::Arc<std::sync::Mutex<Option<String>>>,
    comment_buffer: crate::task::CommentBuffer,
) -> anyhow::Result<u16> {
    let base_port = zbobr.config().base_port;
    use rmcp::transport::streamable_http_server::{
        StreamableHttpService, session::local::LocalSessionManager,
    };

    let path = format!("/{}/{}", role_name, task_id);

    let session: RoleSession =
        zbobr.role_session_with_tracker(task_id, tool_tracker, comment_buffer);

    let role_name_owned = role_name.to_string();
    tracing::info!("Creating UnifiedMcp service for task {task_id} role '{role_name}' at path {path}");
    let svc = StreamableHttpService::new(
        move || {
            tracing::debug!("Creating new UnifiedMcp instance for task {task_id} role '{}'", role_name_owned);
            Ok(super::unified::UnifiedMcp::new(
                session.clone(),
                allowed_tools.clone(),
                role_name_owned.clone(),
                tool,
                model.clone(),
                stage_name.clone(),
            ))
        },
        std::sync::Arc::new(LocalSessionManager::default()),
        Default::default(),
    );
    let router = axum::Router::new().nest_service(&path, svc);

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
