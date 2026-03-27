The `cleanup_legacy_token_config` function silently ignores failures from git commands. It should return Result and log issues.

Fix: Change the return type to `Result<()>`, log warnings for individual unset failures, and return the overall result.