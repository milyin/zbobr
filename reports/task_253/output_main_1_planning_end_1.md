Plan finalized and sent to implementation. 6 checklist items covering:

1. **Remove preparator stage** from the default pipeline
2. **Auto-derive branch names** from task title (replaces preparator's role)
3. **Move `repository`/`branch`** config from dispatcher to repo backend (required fields)
4. **Remove fork support** from GitHub backend (`fork_owner`, `ensure_fork`, fork remote)
5. **Simplify `TaskIdentity`** — remove redundant repo/branch fields now owned by backend
6. **Update tests/docs/configs** to reflect the new schema

The filesystem backend serves as the design analog for the simplified GitHub backend throughout.