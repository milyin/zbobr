The reviewer found that the new linting workflow wiring is implemented but not covered by tests. Add tests to zbobr/src/init.rs that verify:
1. main pipeline contains "linting" stage
2. "linting" appears before "testing" in main pipeline stages order
3. "linting" stage uses role "linter"
4. role "linter" uses prompt "linter.md" and tool "drudge"
5. default_config_toml() contains the "drudge" tool entry