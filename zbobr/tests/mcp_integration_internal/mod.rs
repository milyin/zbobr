pub mod env;
pub mod stage;
mod mcp_tester_scenarios;

// re-export the most common helpers so the outer test module can import them
#[cfg(test)]
pub use env::IntegrationTestEnv;
#[cfg(test)]
pub use stage::{
    PreparationScenario,
    PlanningScenario,
    preparation_scenario,
    planning_scenario,
};
