extern crate self as zbobr_api;

pub mod backend;
pub mod config;
pub mod prompt;
pub mod task;
pub mod tool_executor;

pub use backend::{TaskBackend, TaskBackendExt, TaskMut, TaskWeak, WorktreeBackend};
pub use config::{Config, ZbobrDispatcherArgs, ZbobrDispatcherConfig, ZbobrDispatcherToml};
pub use prompt::{
    MergerToolNames, PlannerToolNames, PreparatorToolNames, PromptBuilder, ReviewerToolNames,
    TesterToolNames, WorkerToolNames,
};
pub use task::{
    ChecklistItem, Comment, CommentTag, CommentType, HistoryChunk, Model, Role, Signal, Stage,
    Task, TaskIdentity, Tool, extract_history_chunk, extract_repo_name,
};
pub use tool_executor::{ToolExecutor, format_command_for_log};
