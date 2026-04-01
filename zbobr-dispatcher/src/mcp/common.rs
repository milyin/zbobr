use std::collections::HashSet;

use zbobr_api::config_tools::McpTool;

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
pub struct ReportParam {
    #[schemars(description = "Brief, concise summary of results (stored as comment)")]
    pub brief: String,
    #[schemars(
        description = "Full detailed report content (stored as a file in the task repository)"
    )]
    pub full_report: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct AddChecklistItemParam {
    #[schemars(
        description = "Brief, concise summary of the checklist item (stored as context record text)"
    )]
    pub brief: String,
    #[schemars(
        description = "Full detailed description of the checklist item (stored as a file in the task repository)"
    )]
    pub full_report: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct CheckChecklistItemParam {
    #[schemars(
        description = "Context record ID to check — either a numeric id or a string like 'ctx_rec_5'"
    )]
    pub id: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct DeleteCtxRecParam {
    #[schemars(
        description = "Context record ID to delete — either a numeric id or a string like 'ctx_rec_5'"
    )]
    pub id: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct GetCtxRecParam {
    #[schemars(
        description = "Context record ID — either a numeric id or a string like 'ctx_rec_5'"
    )]
    pub id: String,
}

/// Parse a context record ID from either a numeric string or `ctx_rec_N` format.
pub fn parse_ctx_rec_id(id: &str) -> Result<u64, String> {
    if let Ok(n) = id.parse::<u64>() {
        return Ok(n);
    }
    if let Some(n_str) = id.strip_prefix("ctx_rec_") {
        if let Ok(n) = n_str.parse::<u64>() {
            return Ok(n);
        }
    }
    Err(format!(
        "Invalid context record ID '{}': expected a number or 'ctx_rec_N'",
        id
    ))
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
    allowed_tools: HashSet<McpTool>,
    tool_tracker: std::sync::Arc<std::sync::Mutex<Option<McpTool>>>,
    pipeline_name: String,
    pipeline_run_id: u64,
) -> anyhow::Result<u16> {
    let base_port = zbobr.config().base_port;
    use rmcp::transport::streamable_http_server::{
        StreamableHttpService, session::local::LocalSessionManager,
    };

    let path = format!("/{}/{}", role_name, task_id);

    let session: RoleSession = zbobr.role_session_with_tracker(
        task_id,
        tool_tracker,
        pipeline_name.clone(),
        pipeline_run_id,
    );

    let role_name_owned = role_name.to_string();
    tracing::info!(
        "Creating UnifiedMcp service for task {task_id} role '{role_name}' at path {path}"
    );
    let svc = StreamableHttpService::new(
        move || {
            tracing::debug!(
                "Creating new UnifiedMcp instance for task {task_id} role '{}'",
                role_name_owned
            );
            Ok(super::unified::UnifiedMcp::new(
                session.clone(),
                allowed_tools.clone(),
                role_name_owned.clone(),
                tool,
                model.clone(),
                stage_name.clone(),
                pipeline_name.clone(),
                pipeline_run_id,
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
