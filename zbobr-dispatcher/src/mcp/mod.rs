pub mod common;
pub mod traits;
pub mod instructions;

// Re-export commonly used tool name modules from `common` so they are
// available as `zbobr_dispatcher::mcp::<role>_tools` for tests and callers.
pub use common::preparator_tools;
pub use common::planner_tools;
pub use common::worker_tools;
pub use common::reviewer_tools;
pub use common::merger_tools;

pub mod preparator;
pub mod planner;
pub mod worker;
pub mod reviewer;
pub mod merger;

pub use preparator::PreparatorMcp;
pub use planner::PlannerMcp;
pub use worker::WorkerMcp;
pub use reviewer::ReviewerMcp;
pub use merger::MergerMcp;

pub use instructions::{
    preparator_instructions, planner_instructions, worker_instructions, reviewer_instructions,
    merger_instructions,
};

pub use common::{
    DescriptionParam, MessageParam, InsertChecklistItemParam, UpdateChecklistItemParam,
    CheckChecklistItemParam, DeleteChecklistItemParam, SetDestinationRepositoryParam,
    SetDestinationBranchParam, SetWorkBranchParam, run_role_mcp_server,
};
