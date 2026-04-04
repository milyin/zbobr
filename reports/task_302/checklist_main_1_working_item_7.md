The current uncommitted changes in zbobr-macros/src/lib.rs and zbobr-api/src/config.rs already implement the is_map_type check and WorkflowToml extend-based merge. Need to:
1. Add behavior-oriented tests for map merge (partially overlapping named sections)
2. Build and verify everything compiles
3. Run tests
4. Commit the changes