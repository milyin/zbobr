# Add ReportIntermediate to Planner MCP Tools

## File
`zbobr/src/init.rs`, lines 233–246

## Change
In the `"planner"` `RoleDefinition`, add `ReportIntermediate` to the `mcp` vec:

```rust
(
    "planner".into(),
    RoleDefinition {
        mcp: vec![
            StopWithError,
            StopWithQuestion,
            ReportSuccess,
            ReportIntermediate,   // <-- add this
            AddChecklistItem,
            DeleteCtxRec,
        ],
        prompt: Some(PathBuf::from("planner.md")),
        default_tool: None,
        default_model: None,
    },
),
```

`ReportIntermediate` is already imported and used in the worker and reviewer role definitions in the same file, so no new imports are needed.
