extern crate self as zbobr_api;

pub mod backend;
pub mod config;
pub mod config_tools;
pub mod context;
pub mod prompt;
pub mod task;
pub mod tool_executor;

pub use backend::{
    ERROR_PREFIX, PAUSE_PREFIX, QUESTION_PREFIX, TaskBackend, TaskBackendExt, TaskMut, TaskWeak,
    WorktreeBackend, format_status,
};
pub use config::{
    Config, PipelineConfig, StageDefinition, StageTransition, WorkflowArgs, WorkflowConfig,
    WorkflowToml, ZbobrDispatcherArgs, ZbobrDispatcherConfig, ZbobrDispatcherToml,
};
pub use context::format_timestamp;
pub use task::{
    Comment, CommentTag, ContextRecord, ContextRecordType, HistoryRecordType, Model, Pipeline,
    Role, Signal, StackEntry, Stage, StageContext, StageInfo, State, Task, TaskContext,
    TaskIdentity, Tool, classify_comment, comment_tag, extract_repo_name, extract_summary,
    filter_comments_for_run,
};
pub use tool_executor::{ExecutorOutput, ToolExecutor, format_command_for_log};
