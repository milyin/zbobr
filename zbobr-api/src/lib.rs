extern crate self as zbobr_api;

pub mod backend;
pub mod checklist_format;
pub mod config;
pub mod config_tools;
pub mod prompt;
pub mod task;
pub mod tool_executor;

pub use backend::{TaskBackend, TaskBackendExt, TaskMut, TaskWeak, WorktreeBackend};
pub use config::{
    Config, PipelineConfig, StageDefinition, StageTransition, WorkflowArgs, WorkflowConfig,
    WorkflowToml, ZbobrDispatcherArgs, ZbobrDispatcherConfig, ZbobrDispatcherToml,
};
pub use task::{
    ChecklistItem, Comment, CommentTag, FLAG_PREFIX, HistoryRecordType, Model, PIPELINE_PREFIX,
    Pipeline, Role, SIGNAL_PREFIX, STAGE_PREFIX, STATE_PREFIX, Signal, StackEntry, Stage, State,
    Task, TaskIdentity, Tool, classify_comment, comment_tag, extract_repo_name, extract_summary,
    filter_comments_for_run,
};
pub use tool_executor::{ToolExecutor, format_command_for_log};
