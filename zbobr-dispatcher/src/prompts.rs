use std::path::PathBuf;

use crate::config::{ZbobrDispatcherArgs, ZbobrDispatcherConfig};
use crate::task::Role;

/// Resolved prompt file paths for each role.
#[derive(Debug, Clone)]
pub struct Prompts {
    pub base_path: Option<PathBuf>,
    pub preparator: Vec<PathBuf>,
    pub planner: Vec<PathBuf>,
    pub worker: Vec<PathBuf>,
    pub reviewer: Vec<PathBuf>,
    pub merger: Vec<PathBuf>,
}

/// Resolve prompt paths from CLI args and config.
/// CLI args take precedence over config values.
pub fn resolve_prompts(args: &ZbobrDispatcherArgs, config: &ZbobrDispatcherConfig) -> Prompts {
    let preparator = args
        .preparator_prompts
        .clone()
        .unwrap_or_else(|| config.preparator_prompts.clone());
    let planner = args
        .planner_prompts
        .clone()
        .unwrap_or_else(|| config.planner_prompts.clone());
    let worker = args
        .worker_prompts
        .clone()
        .unwrap_or_else(|| config.worker_prompts.clone());
    let reviewer = args
        .reviewer_prompts
        .clone()
        .unwrap_or_else(|| config.reviewer_prompts.clone());
    let merger = config.merger_prompts.clone();
    let base_path = args
        .prompts_path
        .clone()
        .or_else(|| config.prompts_path.clone());

    Prompts {
        base_path,
        preparator,
        planner,
        worker,
        reviewer,
        merger,
    }
}

/// Load and concatenate multiple prompt files.
/// Relative paths are resolved relative to `base_path` if provided, otherwise cwd.
/// Missing files are silently skipped.
pub fn load_prompts(paths: &[PathBuf], base_path: Option<&PathBuf>) -> anyhow::Result<String> {
    let mut combined = String::new();
    for path in paths.iter() {
        let resolved_path = if let Some(base) = base_path {
            if path.is_relative() { base.join(path) } else { path.clone() }
        } else if path.is_relative() {
            std::env::current_dir()?.join(path)
        } else {
            path.clone()
        };

        let content = match std::fs::read_to_string(&resolved_path) {
            Ok(c) => c,
            Err(_) => {
                tracing::debug!("Prompt file not found, skipping: {}", resolved_path.display());
                continue;
            }
        };

        let trimmed = content.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !combined.is_empty() {
            combined.push_str("\n\n");
        }
        combined.push_str(trimmed);
    }
    Ok(combined)
}

/// Build full prompt: role instructions + user context + auto-generated API docs.
pub fn build_full_prompt(user_context: &str, role: Role) -> String {
    let hardcoded = match role {
        Role::Preparator => crate::preparator_instructions(),
        Role::Planner => crate::planner_instructions(),
        Role::Worker => crate::worker_instructions(),
        Role::Reviewer => crate::reviewer_instructions(),
        Role::Merger => crate::merger_instructions(),
    };

    let api_docs = match role {
        Role::Preparator => crate::PreparatorMcp::generate_api_docs(),
        Role::Planner => crate::PlannerMcp::generate_api_docs(),
        Role::Worker => crate::WorkerMcp::generate_api_docs(),
        Role::Reviewer => crate::ReviewerMcp::generate_api_docs(),
        Role::Merger => crate::MergerMcp::generate_api_docs(),
    };

    if user_context.is_empty() {
        format!("{}\n\n---\n\n{}", hardcoded, api_docs)
    } else {
        format!("{}\n\n---\n\n{}\n\n---\n\n{}", hardcoded, user_context, api_docs)
    }
}

impl Prompts {
    /// Build the full prompt for the given role.
    pub fn build_prompt(&self, role: Role) -> anyhow::Result<String> {
        let base_prompt = match role {
            Role::Preparator => load_prompts(&self.preparator, self.base_path.as_ref())?,
            Role::Planner => load_prompts(&self.planner, self.base_path.as_ref())?,
            Role::Worker => load_prompts(&self.worker, self.base_path.as_ref())?,
            Role::Reviewer => load_prompts(&self.reviewer, self.base_path.as_ref())?,
            Role::Merger => load_prompts(&self.merger, self.base_path.as_ref())?,
        };
        Ok(build_full_prompt(&base_prompt, role))
    }
}
