use std::path::PathBuf;


use zbobr_dispatcher::{
    task::Role,
    ZbobrDispatcherConfig,
};

/// Resolved prompt file paths for planner, worker, and merger.
///
/// This struct is passed around by the CLI and role session code so that it
/// can load the appropriate prompt text for the current role.  It used to live
/// in `main.rs`, but this module now owns all of the prompt-related logic so
/// that the responsibilities are clearly separated.
#[derive(Debug, Clone)]
pub(crate) struct Prompts {
    pub(crate) base_path: Option<PathBuf>,
    pub(crate) preparator: Vec<PathBuf>,
    pub(crate) planner: Vec<PathBuf>,
    pub(crate) worker: Vec<PathBuf>,
    pub(crate) reviewer: Vec<PathBuf>,
    pub(crate) merger: Vec<PathBuf>,
}

/// Resolve prompt paths: CLI arg > config values.
/// Paths are resolved relative to prompts_path if provided, otherwise relative to
/// current directory.
pub(crate) fn resolve_prompts(
    cli: &crate::Cli,
    config: &ZbobrDispatcherConfig,
) -> anyhow::Result<Prompts> {
    // Use CLI args if provided, otherwise use config (which came from TOML/env/defaults)
    let planner = cli
        .global
        .settings
        .dispatcher
        .planner_prompts
        .clone()
        .unwrap_or_else(|| config.planner_prompts.clone());

    let preparator = cli
        .global
        .settings
        .dispatcher
        .preparator_prompts
        .clone()
        .unwrap_or_else(|| config.preparator_prompts.clone());

    let worker = cli
        .global
        .settings
        .dispatcher
        .worker_prompts
        .clone()
        .unwrap_or_else(|| config.worker_prompts.clone());

    let reviewer = cli
        .global
        .settings
        .dispatcher
        .reviewer_prompts
        .clone()
        .unwrap_or_else(|| config.reviewer_prompts.clone());

    let merger = config.merger_prompts.clone();

    // CLI prompts_path > config.prompts_path (which came from TOML/env)
    let base_path = cli
        .global
        .settings
        .dispatcher
        .prompts_path
        .clone()
        .or_else(|| config.prompts_path.clone());

    Ok(Prompts {
        base_path,
        preparator,
        planner,
        worker,
        reviewer,
        merger,
    })
}

/// Load and concatenate multiple prompt files (additional user context).
/// If base_path is provided, relative paths are resolved relative to it.
/// Otherwise, relative paths are resolved relative to the current directory.
/// Missing files are silently skipped (they are optional additional context).
pub(crate) fn load_prompts(
    paths: &[PathBuf],
    base_path: Option<&PathBuf>,
) -> anyhow::Result<String> {
    let mut combined = String::new();
    for path in paths.iter() {
        // Resolve path relative to base_path if provided and path is relative
        let resolved_path = if let Some(base) = base_path {
            if path.is_relative() {
                base.join(path)
            } else {
                path.clone()
            }
        } else if path.is_relative() {
            std::env::current_dir()?.join(path)
        } else {
            path.clone()
        };

        let content = match std::fs::read_to_string(&resolved_path) {
            Ok(c) => c,
            Err(_) => {
                tracing::debug!(
                    "Prompt file not found, skipping: {}",
                    resolved_path.display()
                );
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

impl Prompts {
    /// Construct the full prompt text for the given role by loading any user-\
    /// supplied context files and combining them with the hardcoded
    /// instructions and generated MCP API documentation.
    pub(crate) fn build_prompt(&self, role: Role) -> anyhow::Result<String> {
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

/// Build full prompt: hardcoded instructions + user context files + auto-generated
/// API docs.
pub(crate) fn build_full_prompt(user_context: &str, role: Role) -> String {
    let hardcoded = match role {
        Role::Preparator => zbobr_dispatcher::preparator_instructions(),
        Role::Planner => zbobr_dispatcher::planner_instructions(),
        Role::Worker => zbobr_dispatcher::worker_instructions(),
        Role::Reviewer => zbobr_dispatcher::reviewer_instructions(),
        Role::Merger => zbobr_dispatcher::merger_instructions(),
    };

    let api_docs = match role {
        Role::Preparator => zbobr_dispatcher::PreparatorMcp::generate_api_docs(),
        Role::Planner => zbobr_dispatcher::PlannerMcp::generate_api_docs(),
        Role::Worker => zbobr_dispatcher::WorkerMcp::generate_api_docs(),
        Role::Reviewer => zbobr_dispatcher::ReviewerMcp::generate_api_docs(),
        Role::Merger => zbobr_dispatcher::MergerMcp::generate_api_docs(),
    };

    if user_context.is_empty() {
        format!("{}\n\n---\n\n{}", hardcoded, api_docs)
    } else {
        format!(
            "{}\n\n---\n\n{}\n\n---\n\n{}",
            hardcoded, user_context, api_docs
        )
    }
}


/*
#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn load_prompts_skips_missing_and_concatenates() {
        // create a temporary file with some content
        let mut tmp = env::temp_dir();
        tmp.push("zbobr_test_prompt.txt");
        let mut f = File::create(&tmp).expect("cannot create temp file");
        writeln!(f, "hello").unwrap();

        let result = load_prompts(&[tmp.clone(), PathBuf::from("nonexistent")], None)
            .expect("load_prompts failed");
        assert!(result.contains("hello"));

        // cleanup
        let _ = std::fs::remove_file(tmp);
    }

    #[test]
    fn build_full_prompt_contains_api_and_role_instructions() {
        let prompt = build_full_prompt("user context", Role::Preparator);
        // should include both the user context and some known phrase from dispatcher instructions
        assert!(prompt.contains("user context"));
        assert!(prompt.contains("\n\n---\n\n"));
    }

    #[test]
    fn prompts_build_prompt_delegates_and_loads_context() {
        // create temporary context file and configure Prompts to point to it
        let mut tmp = env::temp_dir();
        tmp.push("zbobr_context.txt");
        let mut f = File::create(&tmp).expect("cannot create temp file");
        writeln!(f, "context line").unwrap();

        let prompts = Prompts {
            base_path: None,
            preparator: vec![tmp.clone()],
            planner: vec![],
            worker: vec![],
            reviewer: vec![],
            merger: vec![],
        };

        let built = prompts.build_prompt(Role::Preparator).expect("build_prompt failed");
        assert!(built.contains("context line"));
        assert!(built.contains("---"));

        let _ = std::fs::remove_file(tmp);
    }
}
*/
