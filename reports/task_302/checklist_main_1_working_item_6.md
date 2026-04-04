Add behavior-oriented tests that:
1. Merge two configs with partially overlapping named sections (providers, tools, roles, pipelines)
2. Assert that untouched entries from base are preserved
3. Assert that matching keys are overridden by the overlay
4. Test both macro-generated (dispatcher) and manual (workflow) merge paths