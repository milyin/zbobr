use std::path::PathBuf;

use crate::{
    config::{ZbobrDispatcherArgs, ZbobrDispatcherConfig},
    task::Role,
};

/// Resolved prompt file paths for each role.
#[derive(Debug, Clone)]
pub struct Prompts {
    pub base_path: Option<PathBuf>,
    pub preparator: Vec<PathBuf>,
    pub analyser: Vec<PathBuf>,
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
    let analyser = args
        .analyser_prompts
        .clone()
        .unwrap_or_else(|| config.analyser_prompts.clone());
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
    let merger = args
        .merger_prompts
        .clone()
        .unwrap_or_else(|| config.merger_prompts.clone());
    let base_path = args
        .prompts_path
        .clone()
        .or_else(|| config.prompts_path.clone());

    Prompts {
        base_path,
        preparator,
        analyser,
        planner,
        worker,
        reviewer,
        merger,
    }
}

/// Load and concatenate multiple prompt files.
/// Relative paths are resolved relative to `base_path` if provided, otherwise cwd.
/// Returns an error if any file cannot be read.
pub fn load_prompts(paths: &[PathBuf], base_path: Option<&PathBuf>) -> anyhow::Result<String> {
    let mut combined = String::new();
    for path in paths.iter() {
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

        let content = std::fs::read_to_string(&resolved_path).map_err(|e| {
            anyhow::anyhow!(
                "Failed to read prompt file '{}': {}",
                resolved_path.display(),
                e
            )
        })?;

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
        Role::Analyser => crate::analyser_instructions(),
        Role::Planner => crate::planner_instructions(),
        Role::Worker => crate::worker_instructions(),
        Role::Reviewer => crate::reviewer_instructions(),
        Role::Merger => crate::merger_instructions(),
    };

    let api_docs = match role {
        Role::Preparator => crate::PreparatorMcp::generate_api_docs(),
        Role::Analyser => crate::AnalyserMcp::generate_api_docs(),
        Role::Planner => crate::PlannerMcp::generate_api_docs(),
        Role::Worker => crate::WorkerMcp::generate_api_docs(),
        Role::Reviewer => crate::ReviewerMcp::generate_api_docs(),
        Role::Merger => crate::MergerMcp::generate_api_docs(),
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

/// Validate that all specified prompt files exist.
/// Returns an error listing all missing files if any are not found.
pub fn validate_prompts(prompts: &Prompts) -> anyhow::Result<()> {
    let mut missing_files = Vec::new();

    for path in &prompts.preparator {
        if !file_exists(path, prompts.base_path.as_ref()) {
            missing_files.push(path.clone());
        }
    }
    for path in &prompts.analyser {
        if !file_exists(path, prompts.base_path.as_ref()) {
            missing_files.push(path.clone());
        }
    }
    for path in &prompts.planner {
        if !file_exists(path, prompts.base_path.as_ref()) {
            missing_files.push(path.clone());
        }
    }
    for path in &prompts.worker {
        if !file_exists(path, prompts.base_path.as_ref()) {
            missing_files.push(path.clone());
        }
    }
    for path in &prompts.reviewer {
        if !file_exists(path, prompts.base_path.as_ref()) {
            missing_files.push(path.clone());
        }
    }
    for path in &prompts.merger {
        if !file_exists(path, prompts.base_path.as_ref()) {
            missing_files.push(path.clone());
        }
    }

    if !missing_files.is_empty() {
        let missing_list = missing_files
            .iter()
            .map(|p| format!("  - {}", p.display()))
            .collect::<Vec<_>>()
            .join("\n");
        return Err(anyhow::anyhow!(
            "The following prompt files do not exist:\n{}",
            missing_list
        ));
    }

    Ok(())
}

/// Check if a file exists, resolving relative paths with base_path if provided.
fn file_exists(path: &PathBuf, base_path: Option<&PathBuf>) -> bool {
    let resolved_path = if let Some(base) = base_path {
        if path.is_relative() {
            base.join(path)
        } else {
            path.clone()
        }
    } else if path.is_relative() {
        match std::env::current_dir() {
            Ok(cwd) => cwd.join(path),
            Err(_) => return false,
        }
    } else {
        path.clone()
    };

    resolved_path.exists()
}

impl Prompts {
    /// Build the full prompt for the given role.
    pub fn build_prompt(&self, role: Role) -> anyhow::Result<String> {
        let base_prompt = match role {
            Role::Preparator => load_prompts(&self.preparator, self.base_path.as_ref())?,
            Role::Analyser => load_prompts(&self.analyser, self.base_path.as_ref())?,
            Role::Planner => load_prompts(&self.planner, self.base_path.as_ref())?,
            Role::Worker => load_prompts(&self.worker, self.base_path.as_ref())?,
            Role::Reviewer => load_prompts(&self.reviewer, self.base_path.as_ref())?,
            Role::Merger => load_prompts(&self.merger, self.base_path.as_ref())?,
        };
        Ok(build_full_prompt(&base_prompt, role))
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Write};

    use tempfile::TempDir;

    use super::*;

    fn write_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
        let path = dir.path().join(name);
        let mut f = fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        path
    }

    fn default_config() -> ZbobrDispatcherConfig {
        ZbobrDispatcherConfig {
            preparator_prompts: vec![],
            analyser_prompts: vec![],
            planner_prompts: vec![],
            worker_prompts: vec![],
            reviewer_prompts: vec![],
            merger_prompts: vec![],
            prompts_path: None,
            ..ZbobrDispatcherConfig::default()
        }
    }

    // --- load_prompts ---

    #[test]
    fn load_prompts_reads_existing_file() {
        let dir = TempDir::new().unwrap();
        let path = write_file(&dir, "p.md", "  hello world  ");
        let result = load_prompts(&[path], None).unwrap();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn load_prompts_errors_on_missing_file() {
        let result = load_prompts(&[PathBuf::from("/nonexistent/path/prompt.md")], None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Failed to read prompt file"));
    }

    #[test]
    fn load_prompts_concatenates_multiple_files() {
        let dir = TempDir::new().unwrap();
        let a = write_file(&dir, "a.md", "first");
        let b = write_file(&dir, "b.md", "second");
        let result = load_prompts(&[a, b], None).unwrap();
        assert_eq!(result, "first\n\nsecond");
    }

    #[test]
    fn load_prompts_skips_empty_files() {
        let dir = TempDir::new().unwrap();
        let empty = write_file(&dir, "empty.md", "   \n  \n");
        let real = write_file(&dir, "real.md", "content");
        let result = load_prompts(&[empty, real], None).unwrap();
        assert_eq!(result, "content");
    }

    #[test]
    fn load_prompts_resolves_relative_path_with_base_path() {
        let dir = TempDir::new().unwrap();
        write_file(&dir, "custom.md", "custom content");
        let relative = PathBuf::from("custom.md");
        let base = dir.path().to_path_buf();
        let result = load_prompts(&[relative], Some(&base)).unwrap();
        assert_eq!(result, "custom content");
    }

    #[test]
    fn load_prompts_absolute_path_ignores_base_path() {
        let dir = TempDir::new().unwrap();
        let abs = write_file(&dir, "abs.md", "absolute content");
        let fake_base = PathBuf::from("/nonexistent/base");
        let result = load_prompts(&[abs], Some(&fake_base)).unwrap();
        assert_eq!(result, "absolute content");
    }

    // --- resolve_prompts ---

    #[test]
    fn resolve_prompts_uses_config_when_args_empty() {
        let config = default_config();
        let args = ZbobrDispatcherArgs::default();
        let prompts = resolve_prompts(&args, &config);
        assert_eq!(prompts.preparator, config.preparator_prompts);
        assert_eq!(prompts.planner, config.planner_prompts);
        assert_eq!(prompts.worker, config.worker_prompts);
        assert_eq!(prompts.reviewer, config.reviewer_prompts);
        assert_eq!(prompts.merger, config.merger_prompts);
        assert_eq!(prompts.base_path, None);
    }

    #[test]
    fn resolve_prompts_args_override_config() {
        let config = default_config();
        let args = ZbobrDispatcherArgs {
            preparator_prompts: Some(vec![PathBuf::from("override.md")]),
            planner_prompts: Some(vec![PathBuf::from("plan_override.md")]),
            ..Default::default()
        };
        let prompts = resolve_prompts(&args, &config);
        assert_eq!(prompts.preparator, vec![PathBuf::from("override.md")]);
        assert_eq!(prompts.planner, vec![PathBuf::from("plan_override.md")]);
        // Other roles still use config
        assert_eq!(prompts.worker, config.worker_prompts);
    }

    #[test]
    fn resolve_prompts_merger_args_override_config() {
        let config = default_config();
        let args = ZbobrDispatcherArgs {
            merger_prompts: Some(vec![PathBuf::from("merger_override.md")]),
            ..Default::default()
        };
        let prompts = resolve_prompts(&args, &config);
        assert_eq!(prompts.merger, vec![PathBuf::from("merger_override.md")]);
    }

    #[test]
    fn resolve_prompts_base_path_from_args_overrides_config() {
        let mut config = default_config();
        config.prompts_path = Some(PathBuf::from("/config/prompts"));
        let args = ZbobrDispatcherArgs {
            prompts_path: Some(PathBuf::from("/args/prompts")),
            ..Default::default()
        };
        let prompts = resolve_prompts(&args, &config);
        assert_eq!(prompts.base_path, Some(PathBuf::from("/args/prompts")));
    }

    #[test]
    fn resolve_prompts_base_path_falls_back_to_config() {
        let mut config = default_config();
        config.prompts_path = Some(PathBuf::from("/config/prompts"));
        let args = ZbobrDispatcherArgs::default();
        let prompts = resolve_prompts(&args, &config);
        assert_eq!(prompts.base_path, Some(PathBuf::from("/config/prompts")));
    }

    // --- build_full_prompt ---

    #[test]
    fn build_full_prompt_includes_user_context() {
        let prompt = build_full_prompt("my custom instructions", Role::Worker);
        assert!(prompt.contains("my custom instructions"));
    }

    #[test]
    fn build_full_prompt_empty_context_omits_user_section() {
        let prompt_empty = build_full_prompt("", Role::Worker);
        let prompt_with = build_full_prompt("UNIQUE_MARKER", Role::Worker);
        assert!(!prompt_empty.contains("UNIQUE_MARKER"));
        // With context is longer (has the extra context section)
        assert!(prompt_with.len() > prompt_empty.len());
    }

    // --- Prompts::build_prompt ---

    #[test]
    fn prompts_build_prompt_loads_content_from_file() {
        let dir = TempDir::new().unwrap();
        let path = write_file(&dir, "worker.md", "do the work carefully");
        let prompts = Prompts {
            base_path: None,
            preparator: vec![],
            analyser: vec![],
            planner: vec![],
            worker: vec![path],
            reviewer: vec![],
            merger: vec![],
        };
        let result = prompts.build_prompt(Role::Worker).unwrap();
        assert!(result.contains("do the work carefully"));
    }

    #[test]
    fn prompts_build_prompt_no_custom_content_when_no_files() {
        let prompts = Prompts {
            base_path: None,
            preparator: vec![],
            analyser: vec![],
            planner: vec![],
            worker: vec![],
            reviewer: vec![],
            merger: vec![],
        };
        let result = prompts.build_prompt(Role::Worker).unwrap();
        // Result should equal build_full_prompt with empty context
        let expected = build_full_prompt("", Role::Worker);
        assert_eq!(result, expected);
    }

    #[test]
    fn prompts_build_prompt_uses_base_path_for_relative_files() {
        let dir = TempDir::new().unwrap();
        write_file(&dir, "reviewer.md", "review carefully");
        let prompts = Prompts {
            base_path: Some(dir.path().to_path_buf()),
            preparator: vec![],
            analyser: vec![],
            planner: vec![],
            worker: vec![],
            reviewer: vec![PathBuf::from("reviewer.md")],
            merger: vec![],
        };
        let result = prompts.build_prompt(Role::Reviewer).unwrap();
        assert!(result.contains("review carefully"));
    }

    // --- validate_prompts ---

    #[test]
    fn validate_prompts_succeeds_with_existing_files() {
        let dir = TempDir::new().unwrap();
        let worker_file = write_file(&dir, "worker.md", "content");
        let prompts = Prompts {
            base_path: None,
            preparator: vec![],
            analyser: vec![],
            planner: vec![],
            worker: vec![worker_file],
            reviewer: vec![],
            merger: vec![],
        };
        assert!(validate_prompts(&prompts).is_ok());
    }

    #[test]
    fn validate_prompts_succeeds_with_empty_prompts() {
        let prompts = Prompts {
            base_path: None,
            preparator: vec![],
            analyser: vec![],
            planner: vec![],
            worker: vec![],
            reviewer: vec![],
            merger: vec![],
        };
        assert!(validate_prompts(&prompts).is_ok());
    }

    #[test]
    fn validate_prompts_fails_with_missing_file() {
        let prompts = Prompts {
            base_path: None,
            preparator: vec![],
            analyser: vec![],
            planner: vec![],
            worker: vec![PathBuf::from("/nonexistent/worker.md")],
            reviewer: vec![],
            merger: vec![],
        };
        let result = validate_prompts(&prompts);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("do not exist"));
        assert!(err.contains("/nonexistent/worker.md"));
    }

    #[test]
    fn validate_prompts_lists_all_missing_files() {
        let prompts = Prompts {
            base_path: None,
            preparator: vec![PathBuf::from("/missing1.md")],
            analyser: vec![PathBuf::from("/missing2.md")],
            planner: vec![],
            worker: vec![],
            reviewer: vec![],
            merger: vec![],
        };
        let result = validate_prompts(&prompts);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("/missing1.md"));
        assert!(err.contains("/missing2.md"));
    }

    #[test]
    fn validate_prompts_resolves_relative_paths_with_base_path() {
        let dir = TempDir::new().unwrap();
        write_file(&dir, "worker.md", "content");
        let prompts = Prompts {
            base_path: Some(dir.path().to_path_buf()),
            preparator: vec![],
            analyser: vec![],
            planner: vec![],
            worker: vec![PathBuf::from("worker.md")],
            reviewer: vec![],
            merger: vec![],
        };
        assert!(validate_prompts(&prompts).is_ok());
    }

    #[test]
    fn validate_prompts_detects_missing_relative_paths_with_base_path() {
        let dir = TempDir::new().unwrap();
        let prompts = Prompts {
            base_path: Some(dir.path().to_path_buf()),
            preparator: vec![],
            analyser: vec![],
            planner: vec![],
            worker: vec![PathBuf::from("missing.md")],
            reviewer: vec![],
            merger: vec![],
        };
        let result = validate_prompts(&prompts);
        assert!(result.is_err());
    }
}
