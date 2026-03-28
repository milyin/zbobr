
## What to change

In `zbobr-task-backend-github/src/separator.rs`:
- Rename `ERROR_SEPARATOR` constant → `STATUS_SEPARATOR`, change its value from `"\n\n---ERROR---\n"` to `"\n\n---STATUS---\n"`
- Rename all `error` parameters and variables in `parse_description_full` and `serialize_description_full` to `status`
- Update test names and assertions (e.g. `roundtrip_preserves_error_section` → `roundtrip_preserves_status_section`, `---ERROR---` string checks → `---STATUS---`)

Also update all callers in `zbobr-task-backend-github/src/github.rs` that pass an `error` argument or use these functions.

## Why

The section is no longer strictly for errors — it now holds the current task status (error or question). The separator name and section heading should match.

## No backward compat needed

The user explicitly stated: "do not make efforts to keep backward compatibility". Old tasks with `---ERROR---` sections will lose their status on the next save, which is acceptable.
