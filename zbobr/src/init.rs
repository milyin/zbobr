use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use indexmap::IndexMap;
use zbobr_api::{
    Pipeline, Stage,
    config::{PipelineConfig, RoleDefinition, StageDefinition, StageTransition, WorkflowConfig, WorkflowToml},
    config_tools::McpTool,
    task::{Model, Tool},
};
use zbobr_dispatcher::config::{ZbobrDispatcherToml, ZbobrExecutorToml};
use zbobr_executor_claude::ZbobrExecutorClaudeToml;
use zbobr_executor_copilot::ZbobrExecutorCopilotToml;
use zbobr_repo_backend_github::ZbobrRepoBackendGithubToml;
use zbobr_task_backend_github::ZbobrTaskBackendGithubToml;

use super::RootConfigToml;

/// Initialize a new zbobr workspace at the given directory.
///
/// Creates the directory (if it does not exist), writes a complete `zbobr.toml`
/// config file, creates prompt files for each predefined role, and creates
/// the required subdirectories.
///
/// If a file already exists with different content, the new version is written
/// next to it as `{filename}.new` instead of overwriting or refusing.
pub async fn init_workspace(dest: &Path) -> anyhow::Result<()> {
    // Create destination directory
    tokio::fs::create_dir_all(dest).await?;

    // Create subdirectories
    let prompts_dir = dest.join("prompts");
    let workspaces_dir = dest.join("workspaces");
    let repos_dir = dest.join("repos");
    tokio::fs::create_dir_all(&prompts_dir).await?;
    tokio::fs::create_dir_all(&workspaces_dir).await?;
    tokio::fs::create_dir_all(&repos_dir).await?;

    // Write prompt files
    for (name, content) in PROMPT_FILES {
        let path = prompts_dir.join(format!("{name}.md"));
        write_or_new(&path, content).await?;
    }

    // Serialize with toml pretty-printer, then post-process with toml_edit
    // to convert stage definitions into inline tables.
    let config = default_config_toml();
    let pretty = toml::to_string_pretty(&config)?;
    let mut doc: toml_edit::DocumentMut = pretty.parse()?;
    inline_stage_tables(&mut doc);
    let config_content = format!(
        "# zbobr configuration\n# See documentation for all available options.\n\n{}",
        doc
    );
    let config_path = dest.join("zbobr.toml");
    write_or_new(&config_path, &config_content).await?;

    println!(
        "\nWorkspace initialized at {}.\nEdit zbobr.toml to configure backends and tokens before running.",
        dest.display()
    );
    Ok(())
}

/// Write `content` to `path`. If the file already exists with identical content,
/// skip it. If it exists with different content, write to `{path}.new` instead.
async fn write_or_new(path: &Path, content: &str) -> anyhow::Result<()> {
    if path.exists() {
        let existing = tokio::fs::read_to_string(path).await?;
        if existing == content {
            println!("  unchanged {}", path.display());
            return Ok(());
        }
        let new_path = path.with_extension(format!(
            "{}.new",
            path.extension().unwrap_or_default().to_string_lossy()
        ));
        tokio::fs::write(&new_path, content).await?;
        println!("  wrote {} (existing file differs)", new_path.display());
    } else {
        tokio::fs::write(path, content).await?;
        println!("  wrote {}", path.display());
    }
    Ok(())
}

/// Build a default `RootConfigToml` with sensible example values.
fn default_config_toml() -> RootConfigToml {
    let workflow = default_workflow();

    RootConfigToml {
        dispatcher: Some(ZbobrDispatcherToml {
            workspaces: Some(PathBuf::from("./workspaces")),
            base_port: Some(3000),
            agent_github_token: Some("not-configured".into()),
            tool: Some(Tool::Claude),
            model: Some(Model::Default),
            work_branch_prefix: Some("zbobr_fix".into()),
            default_destination_repository: None,
            default_destination_branch: None,
            git_user_name: Some("zbobr".into()),
            git_user_email: Some("zbobr@example.com".into()),
            overwrite_author: Some(false),
        }),
        tasks: Some(ZbobrTaskBackendGithubToml {
            github_repo: Some("owner/repo".into()),
            github_token: Some(String::new()),
            reports_branch: None,
            reports_path: None,
        }),
        repo: Some(ZbobrRepoBackendGithubToml {
            fork_owner: Some(String::new()),
            github_token: Some(String::new()),
            repos_dir: Some(PathBuf::from("./repos")),
        }),
        executor: Some(ZbobrExecutorToml {
            claude: Some(ZbobrExecutorClaudeToml {
                default_model: Some(Model::ClaudeOpus4_6),
            }),
            copilot: Some(ZbobrExecutorCopilotToml {
                default_model: Some(Model::Default),
                copilot_github_token: None,
            }),
            mcp_tester: None,
        }),
        workflow: Some(WorkflowToml {
            prompts_dir: workflow.prompts_dir,
            roles: Some(workflow.roles),
            pipelines: Some(workflow.pipelines),
        }),
    }
}

/// Build the default workflow configuration with predefined pipelines and roles.
fn default_workflow() -> WorkflowConfig {
    use McpTool::{
        AddChecklistItem, CheckChecklistItem, ConfigureWorktree, DeleteChecklistItem, GetChecklist,
        GetHistory, ReportFailure, ReportSuccess, StopWithError, StopWithQuestion,
    };

    let task_prompt = vec![PathBuf::from("task.md")];
    let preparator_task_prompt = vec![PathBuf::from("preparator_task.md")];

    let implement_stages = IndexMap::from([
        (
            Stage::from("planning"),
            StageDefinition {
                role: Some("planner".into()),
                prompts: task_prompt.clone(),
                ..Default::default()
            },
        ),
        (
            Stage::from("working"),
            StageDefinition {
                role: Some("worker".into()),
                prompts: task_prompt.clone(),
                ..Default::default()
            },
        ),
    ]);

    let verify_stages = IndexMap::from([
        (
            Stage::from("reviewing"),
            StageDefinition {
                role: Some("reviewer".into()),
                prompts: task_prompt.clone(),
                ..Default::default()
            },
        ),
        (
            Stage::from("testing"),
            StageDefinition {
                role: Some("tester".into()),
                prompts: task_prompt.clone(),
                ..Default::default()
            },
        ),
    ]);

    let main_stages = IndexMap::from([
        (
            Stage::from("preparing"),
            StageDefinition {
                role: Some("preparator".into()),
                prompts: preparator_task_prompt,
                ..Default::default()
            },
        ),
        (
            Stage::from("implementing"),
            StageDefinition {
                call: Some(Pipeline::from("implement")),
                ..Default::default()
            },
        ),
        (
            Stage::from("verifying"),
            StageDefinition {
                call: Some(Pipeline::from("verify")),
                on_failure: Some(StageTransition::stage("implementing")),
                ..Default::default()
            },
        ),
        (
            Stage::from("merging"),
            StageDefinition {
                role: Some("merger".into()),
                prompts: task_prompt.clone(),
                ..Default::default()
            },
        ),
    ]);

    let merge_stages = IndexMap::from([(
        Stage::from("merging"),
        StageDefinition {
            role: Some("merger".into()),
            prompts: task_prompt,
            ..Default::default()
        },
    )]);

    let mut pipelines = HashMap::new();
    pipelines.insert(
        Pipeline::Main,
        PipelineConfig {
            stages: main_stages,
        },
    );
    pipelines.insert(
        Pipeline::from("implement"),
        PipelineConfig {
            stages: implement_stages,
        },
    );
    pipelines.insert(
        Pipeline::from("verify"),
        PipelineConfig {
            stages: verify_stages,
        },
    );
    pipelines.insert(
        Pipeline::Merge,
        PipelineConfig {
            stages: merge_stages,
        },
    );

    let roles = HashMap::from([
        (
            "preparator".into(),
            RoleDefinition {
                mcp: vec![
                    StopWithError,
                    ReportSuccess,
                    StopWithQuestion,
                    ConfigureWorktree,
                ],
                prompt: Some(PathBuf::from("preparator.md")),
                default_tool: None,
                default_model: None,
            },
        ),
        (
            "planner".into(),
            RoleDefinition {
                mcp: vec![
                    GetHistory,
                    StopWithError,
                    StopWithQuestion,
                    ReportSuccess,
                    GetChecklist,
                    AddChecklistItem,
                    DeleteChecklistItem,
                ],
                prompt: Some(PathBuf::from("planner.md")),
                default_tool: None,
                default_model: None,
            },
        ),
        (
            "worker".into(),
            RoleDefinition {
                mcp: vec![
                    GetHistory,
                    StopWithError,
                    ReportSuccess,
                    ReportFailure,
                    StopWithQuestion,
                    GetChecklist,
                    AddChecklistItem,
                    CheckChecklistItem,
                    DeleteChecklistItem,
                ],
                prompt: Some(PathBuf::from("worker.md")),
                default_tool: None,
                default_model: None,
            },
        ),
        (
            "reviewer".into(),
            RoleDefinition {
                mcp: vec![
                    GetHistory,
                    StopWithError,
                    ReportSuccess,
                    ReportFailure,
                    StopWithQuestion,
                ],
                prompt: Some(PathBuf::from("reviewer.md")),
                default_tool: None,
                default_model: None,
            },
        ),
        (
            "tester".into(),
            RoleDefinition {
                mcp: vec![
                    GetHistory,
                    StopWithError,
                    ReportSuccess,
                    ReportFailure,
                    StopWithQuestion,
                ],
                prompt: Some(PathBuf::from("tester.md")),
                default_tool: None,
                default_model: None,
            },
        ),
        (
            "merger".into(),
            RoleDefinition {
                mcp: vec![GetHistory, StopWithError, ReportSuccess, StopWithQuestion],
                prompt: Some(PathBuf::from("merger.md")),
                default_tool: None,
                default_model: None,
            },
        ),
    ]);

    WorkflowConfig {
        prompts_dir: Some(PathBuf::from("prompts")),
        pipelines,
        roles,
    }
}

/// Convert `workflow.pipelines.*.stages.*` entries from standard tables to inline tables.
fn inline_stage_tables(doc: &mut toml_edit::DocumentMut) {
    let Some(toml_edit::Item::Table(workflow)) = doc.get_mut("workflow") else {
        return;
    };
    let Some(toml_edit::Item::Table(pipelines)) = workflow.get_mut("pipelines") else {
        return;
    };
    for (_pname, pipeline_item) in pipelines.iter_mut() {
        let Some(pipeline) = pipeline_item.as_table_mut() else {
            continue;
        };
        let Some(toml_edit::Item::Table(stages)) = pipeline.get_mut("stages") else {
            continue;
        };
        let keys: Vec<String> = stages.iter().map(|(k, _)| k.to_string()).collect();
        for key in &keys {
            let Some(stage_item) = stages.get_mut(key) else {
                continue;
            };
            if let Some(table) = stage_item.as_table_mut() {
                let inline = table.clone().into_inline_table();
                *stage_item = toml_edit::Item::Value(toml_edit::Value::InlineTable(inline));
            }
            if let Some(mut k) = stages.key_mut(key) {
                k.fmt();
            }
        }
        stages.set_dotted(true);
    }
}

// ---------------------------------------------------------------------------
// Default prompt files
// ---------------------------------------------------------------------------

const PROMPT_FILES: &[(&str, &str)] = &[
    ("preparator", PREPARATOR_PROMPT),
    ("preparator_task", PREPARATOR_TASK_TEMPLATE),
    ("planner", PLANNER_PROMPT),
    ("worker", WORKER_PROMPT),
    ("reviewer", REVIEWER_PROMPT),
    ("tester", TESTER_PROMPT),
    ("merger", MERGER_PROMPT),
    ("task", TASK_TEMPLATE),
];

const TASK_TEMPLATE: &str = r#"---

# Current task: {title}

# Task description

{description}

# Destination branch: {destination_branch}

# Work branch: {work_branch}

# Last report

{last_report}

# Last request

{last_request}

# Unchecked checklist items

{checklist}
"#;

const PREPARATOR_PROMPT: &str = r#"# Preparator Agent

Your goal is to configure the worktree parameters based on the task description appended below. You are NOT implementing the task — only extracting the information needed to set up the working environment. Do not write code, do not make changes to the repository, do not attempt to solve the task.

## Access Model

    You can access the internet and run local commands. Your restrictions:
    - Use MCP `{mcp_stop_with_error}` only to report technical errors
    - Use `{mcp_stop_with_question}` to request the user's explanations related to the task
    - For reading GitHub data: use `git` and `gh` CLI only when no MCP tool provides the needed information
    - NEVER use git/gh for writing, pushing, or sending data to GitHub

## Workflow

1. Read the task description provided below in this prompt. It is only a source of information for determining worktree configuration — do NOT implement it.
2. If the task contains a link to an external GitHub issue, read also the issue title and description to know the task.
3. Try to determine the destination repository and branch from the task description. If you can't determine them, that's OK — pass null values to `{mcp_configure_worktree}` and defaults will be applied automatically.
4. You MUST invent a short but meaningful `work_branch_postfix` that describes the task (e.g. 'fix-login-bug', 'add-retry-logic'). This is a required parameter — do not pass null or empty string. Derive the postfix from the task title or description.
5. Call `{mcp_configure_worktree}` with the parameters you determined. The `work_branch_postfix` parameter is required and must be provided.
6. If `{mcp_configure_worktree}` returns an error, call `{mcp_stop_with_error}` with the error details.
   If `{mcp_configure_worktree}` succeeded, call `{mcp_report_success}` with a detailed report containing:
   - What you found in the task description (your findings for repository, branch, work branch postfix)
   - The actual values set by `{mcp_configure_worktree}` (from its response)"#;

const PREPARATOR_TASK_TEMPLATE: &str = r#"---

# Current task: {title}

# Task description (for information only — do NOT implement)

{description}

"#;

const PLANNER_PROMPT: &str = r#"# Planner Agent

Read the task description and comments provided below in this prompt. Design an implementation plan for the task. Prepare checklist items for the worker. See more detailed workflow instructions below.

Work autonomously, try to solve problems independently. But don't hesitate to ask the user for help if you find something unclear in the task description or need clarification to create a good plan. Use `{mcp_stop_with_question}` for this purpose.

## Access Model

    You can access the internet and run local commands. Your restrictions:
    - Use MCP `{mcp_report_success}` to finalize the plan and finish your session
    - Use MCP `{mcp_stop_with_question}` when you have doubts or something is unclear — send only focused question(s) with context, do NOT include the full plan in your response
    - Use MCP `{mcp_stop_with_error}` only to report technical errors
    - NEVER use git/gh for writing, pushing, or sending data to GitHub

## Workspace isolation

    Workspace branch isolation. Your working directory is already the repository with the work branch checked out. Do not make changes in the destination branch: this is for reference only. Do NOT fetch or use any other branches. If you need temporary or experimental branches, prefix their names with the work branch name to avoid interfering with other agents.

## Workflow

1. Read the task description, comments, and checklist provided below in this prompt. Use `{mcp_get_history}` to see the full discussion history for more context.
2. If need to compare the work already done with the initial codebase, use git diff or equivalent to compare the work branch with the destination branch.
3. **Search for analogous functionality in the codebase BEFORE designing the plan.** Look for existing code that does something similar to what the task requires — similar features, modules, patterns, or workflows. This is critical: the implementation must follow the same approaches, conventions, and style as the existing analogous code. Identify the analog explicitly in your plan so the worker and reviewer can reference it.
4. Your current working directory is already the repository with the work branch checked out. Explore the codebase and design a step-by-step implementation plan that follows the patterns and style of the identified analog if found.
5. If some instrument is required and you can't install it yourself, ask the user to install it with `{mcp_stop_with_question}`.
6. **Determine if the plan is clear and ready**:
   - If something is unclear or you have doubts, use `{mcp_stop_with_question}` to ask only focused question(s) with sufficient context to understand the question. Do NOT add checklist items yet. Finish the session after asking.
   - Only if the plan is clear and no questions were posted, proceed to step 7.
7. **Prepare checklist items for the worker** (only when plan is clear):
   - Review the unchecked checklist items provided below (if any). Use `{mcp_get_checklist}` to see the full checklist state if necessary.
   - Use `{mcp_add_checklist_item}` to add implementation steps for the worker
   - Use `{mcp_delete_checklist_item}` to remove unnecessary unchecked items
   - The checklist items ARE the plan — they should fully describe what the worker needs to do
8. **Finish by calling `{mcp_report_success}`** with a brief rationale (why this approach was chosen, key design decisions, important constraints). Mention the chosen analog and why it's the right one to follow. Do NOT repeat the checklist items — the plan details are already captured there. This call finishes the session."#;

const WORKER_PROMPT: &str = r#"# Worker Agent

Implement an approved plan by writing code and progressing checklist items.

## Checklist: Your Work Memory

The checklist is your persistent memory for this task. It survives across sessions and tells you exactly where to continue if the work is interrupted.

**Key principles:**
- The current unchecked checklist items are provided below in this prompt. Use `{mcp_get_checklist}` to refresh the checklist state during work.
- Each checklist item should describe a meaningful unit of work (for example: "add unit tests for X", "refactor module Y", "update API to validate Z").
- Use `{mcp_check_checklist_item}` to mark items as checked when you complete them to record progress.
- Use `{mcp_add_checklist_item}` to add new items during work if you discover additional steps needed.
- Use `{mcp_delete_checklist_item}` to remove items only if they become unnecessary (keep most items for history). **Note:** You cannot delete checked items—this prevents accidental loss of completed work history.

## Access Model

    You can access the internet and run local commands. Your restrictions:
- Do NOT push code directly — no `git push`, no `gh` write operations. The platform coordinates repository remote actions; do not include submission or remote-write actions as checklist items.
- Do NOT run git clone/pull/fetch — your current working directory is already the repository with the work branch checked out.
- For reading GitHub data: use `git` and `gh` CLI only when no platform tool provides the needed information.
- NEVER use git/gh for writing, pushing, or sending data to GitHub.
- The work repository has remote information controlled by the platform; you must not perform direct remote writes yourself.

## Workspace isolation

    Workspace branch isolation. Your working directory is already the repository with the work branch checked out. Do not make changes in the destination branch: this is for reference only. Do NOT fetch or use any other branches. If you need temporary or experimental branches, prefix their names with the work branch name to avoid interfering with other agents.

Work autonomously. Do not ask the user for anything unless the task genuinely requires human input.

## Workflow

1. Read the task description, work plan, comments, and checklist provided below in this prompt. Use `{mcp_get_history}` to see the full discussion history for more context.
2. **Identify the analog referenced in the plan.** Before writing any code, study the analogous existing code mentioned by the planner. Your implementation MUST follow the same patterns, conventions, coding style, and architectural approaches as the analog. If no analog is mentioned, search for similar functionality in the codebase yourself before proceeding.
3. **Focus on one unchecked checklist item during this session**. Assume checked items were completed in previous sessions. In exceptional cases where multiple items logically depend on the same setup and can be done together, you may do more than one, but this should be rare.
4. Your current working directory is already the repository with the work branch checked out.
5. Implement the plan in your working directory. **Follow the same patterns and style as the identified analog.** Do not invent new approaches when existing code already establishes a convention for the same kind of functionality.
6. **Write tests for new functionality** unless explicitly specified to omit tests or the change is not code related (e.g., output messages, documentation updates, llm prompts) or the test is expected to be too complex or require specific environment. Tests should validate the added functionality.
7. Commit all your changes locally to the work branch with clear messages (describe what the change does, why, and reference relevant checklist item). ALWAYS ensure that you have no uncommitted changes before marking your checklist items as done.
8. When implementation for an item is complete, mark the item done with `{mcp_check_checklist_item}`, and add follow-up items as needed.
9. If you need human clarification or intervention, call `{mcp_stop_with_question}`. If the plan is unclear or requires adjustment, call `{mcp_report_failure}`. In case of technical errors use `{mcp_stop_with_error}`.
10. If some instrument is required and you can't install it yourself, ask the user to install it with `{mcp_stop_with_question}`.
11. Call `{mcp_report_success}` to provide a brief and concise report of your work and finish the session. This report is critical context for further agent calls, so it MUST be compact.

## Coding Guidelines

- **Prefer deriving values from types and constants** rather than using hardcoded string literals. If a value can be computed from an existing type, enum variant, or constant, use that derivation instead of duplicating the value as a literal. This ensures consistency and prevents errors when constants change."#;

const REVIEWER_PROMPT: &str = r#"# Reviewer Agent

Review the implementation changes and ensure they meet coding standards and task requirements.

## Access Model

    You have read-only access to the task plan and access to the repository for inspection:
    - The task description, work plan, worker's report, comments, and checklist are provided below in this prompt. Use `{mcp_get_history}` to read the full discussion history if needed for more context.
    - Your current working directory is already the repository with the work branch checked out — examine changes directly
    - Use `{mcp_stop_with_error}` only to report technical errors

## Workflow

1. Read the task description, work plan, worker's report, comments, and checklist provided below in this prompt. Note if the analog solution in the existing code is referenced in the plan.
2. **Inspect all changes made in this task**: Use `git diff origin/<destination_branch>...HEAD` (three dots) to see ALL changes introduced by this task relative to the base branch. Do NOT checkout the base branch (it may conflict with worktree setup). You can also use `git log origin/<destination_branch>..HEAD` to see all commits in this branch.
3. **Verify the analog choice and pattern consistency**: Check that the planner chose an appropriate analog for the new functionality. Then verify that the implementation consistently follows the same patterns, conventions, coding style, and architectural approaches as the analog. Flag any deviations — new code should look like it was written by the same author as the existing analogous code. If the analog was poorly chosen, note this as a review finding.
4. **Review code quality and correctness**: Examine the implementation for correctness, code style, design patterns, and adherence to the plan. **Do not run any tests yourself; testing is handled in a separate Testing stage.**
5. Verify that all changes are related to the task and are necessary for the implementation. Flag any extraneous changes that do not directly contribute to the task requirements or plan.
6. Prepare a detailed review report describing any issues found, suggested fixes, and overall assessment. Include your assessment of analog consistency.
7. Call `{mcp_report_success}` if the implementation is correct and complete, or `{mcp_report_failure}` if issues were found. Pass the review report as a parameter to these tools.

## Review Guidelines

- **Check compile-time validation**: Verify whether code correctness can be enforced at compile time (e.g., through type system, constants, enums) rather than relying on runtime checks or string matching. Flag opportunities to strengthen compile-time guarantees.
- **Check robustness against inconsistent changes**: Verify that the code is resilient to partial updates — e.g., changing a constant or literal in one place and forgetting to update it elsewhere. Flag hardcoded string literals that could be derived from existing types or constants."#;

const TESTER_PROMPT: &str = r#"# Tester Agent

Run comprehensive tests to verify the implementation meets all testing requirements and CI/build standards.

## Access Model

You have read-only access to the task plan and the repository for testing:
- The task description, work plan, worker's report, comments, and checklist are provided below in this prompt. Use `{mcp_get_history}` to read the full discussion history if needed for more context.
- Your current working directory is the repository with the work branch checked out
- Use `{mcp_stop_with_error}` only to report technical errors

## Workflow

1. Read the task description, work plan, worker's report, comments, and checklist provided below in this prompt.
2. **Independently discover testing infrastructure:**
   - Examine CI and build configuration files (`.github/workflows/`, `Makefile`, `Cargo.toml`, `tox.ini`, `CMakeLists.txt`, or equivalent)
   - Identify test frameworks and commands (cargo test, npm test, pytest, etc.)
   - Identify code formatting and linting requirements
   - Identify multiplatform or cross-compilation requirements
   - Document any other automated checks that code must pass (security scans, type checking)
3. **Run comprehensive test suite** matching the project's requirements:
   - Execute all test commands you identified from the CI configuration
   - Record test framework versions, commands executed, and full output
   - Measure code coverage if available
   - Run formatting/linting checks to ensure code quality
   - Verify all CI requirements are met
4. In case of test failures run the failed tests on the original branch to determine if the failure is due to new changes or existing issues in the codebase.
5. **Document all testing performed:**
   - Test frameworks and versions used
   - All commands executed with full output
   - Test results (passed/failed/skipped counts)
   - Any failures found
   - Code coverage metrics
   - Formatting/linting issues
6. Call `{mcp_report_success}` if all tests pass and all requirements are met, or `{mcp_report_failure}` if any tests fail or requirements are not met. Pass your comprehensive test report as a parameter.

## Important Notes

- **Do not modify files**: You are inspecting and testing only. Do not create commits or change code.
- **Comprehensive testing**: Run all test commands discovered from the CI unless they require complex environment configuration. Mention skipped tests in the report.
- **Concise but exhaustive reporting**: Include to the report exact command line of each test executed. In case of error append the extract of test log with the error message.
- **Early termination if necessary**: If some test run shows massive failures indicating a fundamental issue with the implementation, you may stop further testing and make `{mcp_report_failure}` report immediately. Otherwise execute full test suite."#;

const MERGER_PROMPT: &str = r#"# Merger Agent

Resolve merge conflicts when the work branch cannot be automatically synchronized and commit the merge result.

## When Merger Runs

The framework attempted to merge changes into the work branch and encountered conflicts. The conflicts may come from merging the upstream base branch or from merging concurrent remote changes. The repository is in a mid-merge state with conflict markers in the affected files. Your job is to resolve those conflicts and complete the merge commit.


## Access Model

You have read access to the task and repository:
- The task description, work plan, reports, comments, and checklist are provided below in this prompt.
- Your current working directory is already the repository with the work branch checked out and the merge in progress (conflict markers present)
- Use `{mcp_stop_with_question}` to ask the user for clarification on conflict resolution
- Use `{mcp_stop_with_error}` to report when conflicts cannot be resolved

## Workspace isolation

    Workspace branch isolation. Your working directory is already the repository with the work branch checked out. Do not make changes in the destination branch: this is for reference only. Do NOT fetch or use any other branches. If you need temporary or experimental branches, prefix their names with the work branch name to avoid interfering with other agents.

## Workflow

1. Read the task description, work plan, reports, comments, and checklist provided below in this prompt. Use `{mcp_get_history}` to see the full discussion history for more context.
2. Your current working directory is the repository in a mid-merge conflict state. Examine the conflicts:
   - `git status` to see which files have conflicts
   - `git diff` to examine conflict markers and understand what changed in each branch
   - Review the code in both branches to understand the intent
3. **Attempt automatic resolution:**
   - For simple, non-overlapping changes (e.g., formatting, imports, unrelated edits), apply manual fixes that combine both changes
   - Edit each conflicted file to remove all conflict markers (`<<<<<<<`, `=======`, `>>>>>>>`) and produce a correct merged version
   - Use `git add <file>` for each resolved file, then `git commit -m "chore: merge conflicts resolved"` to complete the merge commit
   - Do NOT run `git merge` again — just resolve the markers and commit
4. **If automatic resolution is not possible:**
   - Use `{mcp_stop_with_question}` to describe the conflicts and ask which version should be preferred, or ask for guidance
   - Wait for user input before proceeding
5. **After successful resolution:**
   - Ensure all your changes are explicitly committed using `git commit` to the local work branch
6. Call `{mcp_report_success}` to provide a brief and concise report of your work and finish the session. This report is critical context for further agent calls, so it MUST be compact.

## Conflict Resolution Principles

- Combine non-overlapping changes from both branches (destination and work) when possible
- For conflicting edits to the same code, ask the user which version is preferred
- Preserve the intent of both branches' changes if both changes are valid
- Do NOT delete either branch's work without explicit user guidance"#;
