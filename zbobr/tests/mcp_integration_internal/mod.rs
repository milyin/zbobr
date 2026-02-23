pub mod env;
pub mod stage;
mod mcp_tester_scenarios;

// re-export the most common helper so the outer test modules can import it
#[cfg(test)]
pub use env::IntegrationTestEnv;
