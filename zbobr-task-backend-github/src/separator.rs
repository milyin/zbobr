use std::collections::HashMap;

use anyhow::Result;
use zbobr_api::{
    Comment,
    context::{parse_context, serialize_context},
    task::TaskContext,
};

// -- Context parsing and serialization helpers --

pub(crate) const PARAMETERS_SEPARATOR: &str = "\n\n---PARAMETERS---\n";
pub(crate) const STATUS_SEPARATOR: &str = "\n\n---STATUS---\n";
pub(crate) const CONTEXT_SEPARATOR: &str = "\n\n---CONTEXT---\n";
pub(crate) const DEAD_CONTEXT_SEPARATOR: &str = "\n\n---DEAD_CONTEXT---\n";

/// Parse parameters from the PARAMETERS section.
/// Returns a map of parameter names to values.
pub(crate) fn parse_parameters(params_text: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();
    for line in params_text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(colon_pos) = line.find(':') {
            let key = line[..colon_pos].trim().to_string();
            let value = line[colon_pos + 1..].trim().to_string();
            params.insert(key, value);
        }
    }
    params
}

/// Serialize parameters into the PARAMETERS section format.
pub(crate) fn serialize_parameters(params: &HashMap<String, String>) -> String {
    let mut result = String::new();
    for (key, value) in params {
        result.push_str(&format!("{}: {}\n", key, value));
    }
    result
}

/// Parse a task description into (description, parameters, status, context, dead_context).
/// Section order: description → PARAMETERS → STATUS → CONTEXT → DEAD_CONTEXT.
#[allow(clippy::type_complexity)]
pub(crate) fn parse_description_full(
    full_text: &str,
) -> Result<(String, HashMap<String, String>, Option<String>, TaskContext, String)> {
    // Normalize line endings so separators match regardless of \r\n vs \n.
    let normalized = if full_text.contains("\r\n") {
        full_text.replace("\r\n", "\n")
    } else {
        full_text.to_string()
    };

    // Split off DEAD_CONTEXT (must come last)
    let dead_parts: Vec<&str> = normalized.split(DEAD_CONTEXT_SEPARATOR).collect();
    let (before_dead, dead_context_text) = match dead_parts.len() {
        1 => (dead_parts[0], ""),
        _ => (dead_parts[0], dead_parts[1]),
    };

    // Split by context separator
    let parts: Vec<&str> = before_dead.split(CONTEXT_SEPARATOR).collect();

    let (before_context, context_text) = match parts.len() {
        1 => (parts[0], ""),
        _ => (parts[0], parts[1]),
    };

    // Split by status separator
    let status_parts: Vec<&str> = before_context.split(STATUS_SEPARATOR).collect();
    let (before_status, status_text) = match status_parts.len() {
        1 => (status_parts[0], None),
        _ => {
            let text = status_parts[1].trim();
            (
                status_parts[0],
                if text.is_empty() { None } else { Some(text) },
            )
        }
    };

    // Now split by parameters separator
    let param_parts: Vec<&str> = before_status.split(PARAMETERS_SEPARATOR).collect();
    let (description, params_text) = match param_parts.len() {
        1 => (param_parts[0].to_string(), ""),
        _ => (param_parts[0].to_string(), param_parts[1].trim()),
    };

    // Parse parameters
    let parameters = parse_parameters(params_text);

    let status = status_text.map(|s| s.to_string());

    // Parse context using shared format
    let context = parse_context(context_text)?;

    let dead_context = dead_context_text.to_string();

    Ok((description, parameters, status, context, dead_context))
}

/// Serialize description, parameters, status, context, and dead_context back into the full format.
/// Section order: description → PARAMETERS → STATUS → CONTEXT → DEAD_CONTEXT.
/// `comments` are interspersed into the context section as compact titles for user display.
pub(crate) fn serialize_description_full(
    original_description: &str,
    parameters: &HashMap<String, String>,
    status: &Option<String>,
    context: &TaskContext,
    comments: &[Comment],
    report_url: Option<&dyn Fn(&str) -> String>,
    dead_context: &str,
) -> String {
    // Strip everything from the description first
    let clean_description = parse_description_full(original_description)
        .map(|(desc, _, _, _, _)| desc)
        .unwrap_or_else(|_| original_description.to_string());

    let mut result = clean_description;

    // Add parameters if present
    if !parameters.is_empty() {
        result.push_str(PARAMETERS_SEPARATOR);
        result.push_str(&serialize_parameters(parameters));
    }

    // Add status if present
    if let Some(status_msg) = status {
        result.push_str(STATUS_SEPARATOR);
        result.push_str(status_msg);
        result.push('\n');
    }

    // Add context if non-empty
    let context_str = serialize_context(context, comments, false, report_url);
    if !context_str.is_empty() {
        result.push_str(CONTEXT_SEPARATOR);
        result.push_str(&context_str);
    }

    // Add dead_context if non-empty (always last)
    if !dead_context.is_empty() {
        result.push_str(DEAD_CONTEXT_SEPARATOR);
        result.push_str(dead_context);
    }

    result
}

/// Merge concurrent updates to a task description.
///
/// This function handles the case where two concurrent updates have been made to different
/// sections of the task description (description, parameters, status, context).
///
/// Given:
/// - `original`: The description as it was when we first read it
/// - `current`: The description as it exists now (after someone else modified it)
/// - `our_new`: The description we want to write
///
/// This function extracts what parts we modified vs what parts someone else modified,
/// and merges them intelligently:
/// - If we both modified the same section, our change wins (last write wins, simplified)
/// - If only one of us modified a section, that modification is preserved
///
/// The strategy is to parse all three versions, detect what changed in each,
/// and prefer newer values while preserving non-conflicting changes.
///
/// Returns `Err` if any of the three descriptions fail to parse.
pub(crate) fn merge_concurrent_description_updates(
    original: &str,
    current: &str,
    our_new: &str,
) -> Result<String> {
    // Parse all three versions
    let (orig_desc, orig_params, orig_status, orig_context, orig_dead) =
        parse_description_full(original)?;
    let (curr_desc, curr_params, curr_status, curr_context, curr_dead) =
        parse_description_full(current)?;
    let (new_desc, new_params, new_status, new_context, new_dead) =
        parse_description_full(our_new)?;

    // Determine what we changed
    let we_changed_desc = new_desc != orig_desc;
    let we_changed_params = new_params != orig_params;
    let we_changed_status = new_status != orig_status;
    let we_changed_context = serde_json::to_string(&new_context).unwrap_or_default()
        != serde_json::to_string(&orig_context).unwrap_or_default();
    let we_changed_dead = new_dead != orig_dead;

    // Merge: prefer our changes if we made them, otherwise prefer their changes
    let merged_desc = if we_changed_desc { new_desc } else { curr_desc };
    let merged_params = if we_changed_params {
        new_params
    } else {
        curr_params
    };
    let merged_status = if we_changed_status {
        new_status
    } else {
        curr_status
    };
    let merged_context = if we_changed_context {
        new_context
    } else {
        curr_context
    };
    let merged_dead = if we_changed_dead { new_dead } else { curr_dead };

    // Serialize back with the merged content (no URL builder — will be re-serialized by caller)
    // No compact comments during merge — they are re-added when the caller re-serializes.
    Ok(serialize_description_full(
        &merged_desc,
        &merged_params,
        &merged_status,
        &merged_context,
        &[],
        None,
        &merged_dead,
    ))
}

#[cfg(test)]
mod tests {
    use zbobr_api::task::{
        ContextRecord, ContextRecordType, Pipeline, Stage, StageContext, StageInfo,
    };

    use super::*;

    fn sample_context() -> TaskContext {
        TaskContext {
            stages: vec![StageContext {
                info: StageInfo {
                    instance: "default".to_string(),
                    pipeline: Pipeline::from("main"),
                    run_id: 1,
                    stage: Stage::new("working"),
                    tool: Some("claude".to_string()),
                    model: Some("claude-opus-4.6".parse().unwrap()),
                    prompt_link: Some("prompts/work.md".to_string()),
                    output_link: None,
                    timestamp: "2024-01-01T00:00:00Z".parse().unwrap(),
                },
                records: vec![
                    ContextRecord {
                        id: 1,
                        record_type: ContextRecordType::Checkbox(false),
                        brief: "implement feature".to_string(),
                        report_link: None,
                    },
                    ContextRecord {
                        id: 2,
                        record_type: ContextRecordType::Checkbox(true),
                        brief: "write tests".to_string(),
                        report_link: None,
                    },
                    ContextRecord {
                        id: 3,
                        record_type: ContextRecordType::Success,
                        brief: "All done".to_string(),
                        report_link: Some("reports/success.md".to_string()),
                    },
                ],
            }],
        }
    }

    #[test]
    fn roundtrip_preserves_context() {
        let ctx = sample_context();

        let serialized =
            serialize_description_full("my task", &HashMap::new(), &None, &ctx, &[], None, "");
        let (desc, _, _, parsed_ctx, _) = parse_description_full(&serialized).unwrap();

        assert_eq!(desc, "my task");
        assert_eq!(parsed_ctx.stages.len(), 1);
        assert_eq!(parsed_ctx.stages[0].records.len(), 3);

        // Serialization reorders records (non-checkbox first), so look up by ID.
        let records = &parsed_ctx.stages[0].records;
        let find = |id: u64| records.iter().find(|r| r.id == id).unwrap();

        assert_eq!(find(1).record_type, ContextRecordType::Checkbox(false));
        assert_eq!(find(2).record_type, ContextRecordType::Checkbox(true));
        assert_eq!(find(3).record_type, ContextRecordType::Success);
        assert_eq!(find(3).report_link.as_deref(), Some("reports/success.md"));
    }

    #[test]
    fn empty_context_not_serialized() {
        let serialized = serialize_description_full(
            "description",
            &HashMap::new(),
            &None,
            &TaskContext::default(),
            &[],
            None,
            "",
        );
        let (desc, _, _, ctx, _) = parse_description_full(&serialized).unwrap();

        assert_eq!(desc, "description");
        assert!(ctx.stages.is_empty());
        assert!(!serialized.contains("---CONTEXT---"));
    }

    #[test]
    fn roundtrip_preserves_status_section() {
        let mut params = HashMap::new();
        params.insert("key".to_string(), "value".to_string());
        let status = Some("Something went wrong\ndetails here".to_string());
        let ctx = sample_context();

        let serialized =
            serialize_description_full("my task", &params, &status, &ctx, &[], None, "");
        let (desc, parsed_params, parsed_status, parsed_ctx, _) =
            parse_description_full(&serialized).unwrap();

        assert_eq!(desc, "my task");
        assert_eq!(parsed_params.get("key").unwrap(), "value");
        assert_eq!(parsed_status, status);
        assert_eq!(parsed_ctx.stages.len(), 1);
        assert_eq!(parsed_ctx.stages[0].records.len(), 3);

        // Verify section order in serialized output
        let params_pos = serialized.find("---PARAMETERS---").unwrap();
        let status_pos = serialized.find("---STATUS---").unwrap();
        let context_pos = serialized.find("---CONTEXT---").unwrap();
        assert!(params_pos < status_pos);
        assert!(status_pos < context_pos);
    }

    #[test]
    fn roundtrip_no_status_section() {
        let serialized = serialize_description_full(
            "desc",
            &HashMap::new(),
            &None,
            &TaskContext::default(),
            &[],
            None,
            "",
        );
        let (desc, _, status, ctx, _) = parse_description_full(&serialized).unwrap();

        assert_eq!(desc, "desc");
        assert_eq!(status, None);
        assert!(ctx.stages.is_empty());
        assert!(!serialized.contains("---STATUS---"));
    }

    #[test]
    fn merge_preserves_non_conflicting_changes() {
        let original = serialize_description_full(
            "original desc",
            &HashMap::new(),
            &None,
            &TaskContext::default(),
            &[],
            None,
            "",
        );

        // They changed the status
        let current = serialize_description_full(
            "original desc",
            &HashMap::new(),
            &Some("their error".to_string()),
            &TaskContext::default(),
            &[],
            None,
            "",
        );

        // We changed the context
        let our_new = serialize_description_full(
            "original desc",
            &HashMap::new(),
            &None,
            &sample_context(),
            &[],
            None,
            "",
        );

        let merged = merge_concurrent_description_updates(&original, &current, &our_new).unwrap();
        let (desc, _, status, ctx, _) = parse_description_full(&merged).unwrap();

        assert_eq!(desc, "original desc");
        assert_eq!(status, Some("their error".to_string()));
        assert_eq!(ctx.stages.len(), 1);
    }

    #[test]
    fn merge_our_change_wins_on_conflict() {
        let ctx1 = TaskContext {
            stages: vec![StageContext {
                info: StageInfo {
                    instance: "default".to_string(),
                    pipeline: Pipeline::from("main"),
                    run_id: 1,
                    stage: Stage::new("planning"),
                    tool: None,
                    model: None,
                    prompt_link: None,
                    output_link: None,
                    timestamp: "2024-01-01T00:00:00Z".parse().unwrap(),
                },
                records: vec![ContextRecord {
                    id: 1,
                    record_type: ContextRecordType::Checkbox(false),
                    brief: "original item".to_string(),
                    report_link: None,
                }],
            }],
        };

        let original =
            serialize_description_full("desc", &HashMap::new(), &None, &ctx1, &[], None, "");

        // They changed the context
        let ctx_theirs = TaskContext {
            stages: vec![StageContext {
                info: StageInfo {
                    instance: "default".to_string(),
                    pipeline: Pipeline::from("main"),
                    run_id: 1,
                    stage: Stage::new("planning"),
                    tool: None,
                    model: None,
                    prompt_link: None,
                    output_link: None,
                    timestamp: "2024-01-01T00:00:00Z".parse().unwrap(),
                },
                records: vec![ContextRecord {
                    id: 1,
                    record_type: ContextRecordType::Checkbox(false),
                    brief: "their change".to_string(),
                    report_link: None,
                }],
            }],
        };
        let current =
            serialize_description_full("desc", &HashMap::new(), &None, &ctx_theirs, &[], None, "");

        // We also changed the context
        let ctx_ours = TaskContext {
            stages: vec![StageContext {
                info: StageInfo {
                    instance: "default".to_string(),
                    pipeline: Pipeline::from("main"),
                    run_id: 1,
                    stage: Stage::new("planning"),
                    tool: None,
                    model: None,
                    prompt_link: None,
                    output_link: None,
                    timestamp: "2024-01-01T00:00:00Z".parse().unwrap(),
                },
                records: vec![ContextRecord {
                    id: 1,
                    record_type: ContextRecordType::Checkbox(true),
                    brief: "our change".to_string(),
                    report_link: None,
                }],
            }],
        };
        let our_new =
            serialize_description_full("desc", &HashMap::new(), &None, &ctx_ours, &[], None, "");

        let merged = merge_concurrent_description_updates(&original, &current, &our_new).unwrap();
        let (_, _, _, ctx, _) = parse_description_full(&merged).unwrap();

        // Our change should win
        assert_eq!(ctx.stages[0].records[0].brief, "our change");
        assert_eq!(
            ctx.stages[0].records[0].record_type,
            ContextRecordType::Checkbox(true)
        );
    }
}
