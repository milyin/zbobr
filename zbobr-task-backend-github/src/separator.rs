use std::collections::{HashMap, HashSet};

use anyhow::Result;
use chrono::{DateTime, FixedOffset};
use zbobr_api::{
    Comment,
    context::{parse_context, serialize_context},
    task::{ContextRecord, Pipeline, Stage, StageContext, TaskContext},
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
/// Section order: description → PARAMETERS → STATUS → DEAD_CONTEXT → CONTEXT.
///
/// Both `context` and `dead_context` use the same `zbobr-ctx-v1` + markdown
/// envelope; the only difference is that `dead_context` is never passed to
/// agent prompts.
#[allow(clippy::type_complexity)]
pub(crate) fn parse_description_full(
    full_text: &str,
) -> Result<(
    String,
    HashMap<String, String>,
    Option<String>,
    TaskContext,
    TaskContext,
)> {
    // Normalize line endings so separators match regardless of \r\n vs \n.
    let normalized = if full_text.contains("\r\n") {
        full_text.replace("\r\n", "\n")
    } else {
        full_text.to_string()
    };

    // Split off CONTEXT (comes last)
    let ctx_parts: Vec<&str> = normalized.split(CONTEXT_SEPARATOR).collect();
    let (before_context, context_text) = match ctx_parts.len() {
        1 => (ctx_parts[0], ""),
        _ => (ctx_parts[0], ctx_parts[1]),
    };

    // Split off DEAD_CONTEXT (comes just before CONTEXT)
    let dead_parts: Vec<&str> = before_context.split(DEAD_CONTEXT_SEPARATOR).collect();
    let (before_dead, dead_context_text) = match dead_parts.len() {
        1 => (dead_parts[0], ""),
        _ => (dead_parts[0], dead_parts[1]),
    };

    // Split by status separator
    let status_parts: Vec<&str> = before_dead.split(STATUS_SEPARATOR).collect();
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
    let dead_context = parse_context(dead_context_text)?;

    Ok((description, parameters, status, context, dead_context))
}

/// Partition `comments` into (live, dead) groups based on which section's
/// stages "own" each comment on the task timeline.
///
/// A comment is owned by the latest stage (across both contexts) whose
/// timestamp is ≤ the comment's timestamp. When the owning stage lives in
/// `dead_context`, the comment goes to the dead bucket; otherwise to live.
/// Comments preceding every stage go to the live bucket.
fn partition_comments<'a>(
    live: &TaskContext,
    dead: &TaskContext,
    comments: &'a [Comment],
) -> (Vec<&'a Comment>, Vec<&'a Comment>) {
    // Build a timeline of (stage_timestamp, is_dead) entries, sorted ascending.
    let mut timeline: Vec<(DateTime<FixedOffset>, bool)> = Vec::new();
    for stage in &live.stages {
        timeline.push((stage.info.timestamp, false));
    }
    for stage in &dead.stages {
        timeline.push((stage.info.timestamp, true));
    }
    timeline.sort_by_key(|&(ts, _)| ts);

    let mut live_comments = Vec::new();
    let mut dead_comments = Vec::new();
    for c in comments {
        // Find the last stage with timestamp ≤ comment.timestamp.
        let owner_is_dead = timeline
            .iter()
            .rev()
            .find(|(ts, _)| *ts <= c.timestamp)
            .map(|(_, is_dead)| *is_dead)
            .unwrap_or(false);
        if owner_is_dead {
            dead_comments.push(c);
        } else {
            live_comments.push(c);
        }
    }
    (live_comments, dead_comments)
}

/// Serialize description, parameters, status, context, and dead_context back into the full format.
/// Section order: description → PARAMETERS → STATUS → DEAD_CONTEXT → CONTEXT.
/// `comments` are partitioned between CONTEXT and DEAD_CONTEXT by timestamp
/// ownership: a comment follows the latest stage preceding it, wherever that
/// stage lives.
pub(crate) fn serialize_description_full(
    original_description: &str,
    parameters: &HashMap<String, String>,
    status: &Option<String>,
    context: &TaskContext,
    comments: &[Comment],
    report_url: Option<&dyn Fn(&str) -> String>,
    dead_context: &TaskContext,
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

    // Route comments to whichever section's stages own them on the timeline.
    let (live_comments, dead_comments) = partition_comments(context, dead_context, comments);
    let live_comments_owned: Vec<Comment> = live_comments.into_iter().cloned().collect();
    let dead_comments_owned: Vec<Comment> = dead_comments.into_iter().cloned().collect();

    // Always emit the DEAD_CONTEXT marker so the user can move context there.
    result.push_str(DEAD_CONTEXT_SEPARATOR);
    let dead_str = serialize_context(dead_context, &dead_comments_owned, false, report_url);
    if !dead_str.is_empty() {
        result.push_str(&dead_str);
    }

    // Add context if non-empty (always last)
    let context_str = serialize_context(context, &live_comments_owned, false, report_url);
    if !context_str.is_empty() {
        result.push_str(CONTEXT_SEPARATOR);
        result.push_str(&context_str);
    }

    result
}

/// Merge concurrent updates to a task description.
///
/// This function handles the case where two concurrent updates have been made
/// to different parts of the task description.
///
/// Given:
/// - `original`: the description as it was when we first read it.
/// - `current`: the description as it exists now (after someone else modified it).
/// - `our_new`: the description we want to write.
///
/// The top-level fields (`description`, `parameters`, `status`) apply a
/// coarse three-way rule: unchanged side yields to the other, both-changed →
/// ours wins. Contexts merge stage-by-stage so concurrent stage additions,
/// CONTEXT↔DEAD_CONTEXT movements, and record edits compose cleanly.
///
/// Returns `Err` if any of the three descriptions fail to parse.
pub(crate) fn merge_concurrent_description_updates(
    original: &str,
    current: &str,
    our_new: &str,
) -> Result<String> {
    let (orig_desc, orig_params, orig_status, orig_live, orig_dead) =
        parse_description_full(original)?;
    let (curr_desc, curr_params, curr_status, curr_live, curr_dead) =
        parse_description_full(current)?;
    let (new_desc, new_params, new_status, new_live, new_dead) =
        parse_description_full(our_new)?;

    let merged_desc = three_way_pick(&orig_desc, &curr_desc, &new_desc).clone();
    let merged_params = three_way_pick(&orig_params, &curr_params, &new_params).clone();
    let merged_status = three_way_pick(&orig_status, &curr_status, &new_status).clone();
    let (merged_live, merged_dead) = merge_contexts(
        &orig_live, &orig_dead, &curr_live, &curr_dead, &new_live, &new_dead,
    );

    // No compact comments during merge — they are re-added when the caller re-serializes.
    Ok(serialize_description_full(
        &merged_desc,
        &merged_params,
        &merged_status,
        &merged_live,
        &[],
        None,
        &merged_dead,
    ))
}

/// Generic three-way pick: if we didn't touch it, take current; if they
/// didn't touch it, take ours; otherwise ours wins.
fn three_way_pick<'a, T: PartialEq>(orig: &'a T, curr: &'a T, new: &'a T) -> &'a T {
    if new == orig {
        curr
    } else {
        new
    }
}

/// Identity key for locating the same stage execution across versions.
type StageKey = (String, Pipeline, Stage, DateTime<FixedOffset>);

fn stage_key(stage: &StageContext) -> StageKey {
    (
        stage.info.instance.clone(),
        stage.info.pipeline.clone(),
        stage.info.stage.clone(),
        stage.info.timestamp,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Live,
    Dead,
}

/// Merge the two (live, dead) context pairs by reconciling stages individually.
///
/// Each stage is identified by `StageKey`. For a given stage we independently
/// resolve (a) which side it lives on and (b) its record contents. That way a
/// user moving a stage from CONTEXT to DEAD_CONTEXT concurrent with our edits
/// to a different stage yields a consistent result instead of duplicating the
/// moved stage.
fn merge_contexts(
    orig_live: &TaskContext,
    orig_dead: &TaskContext,
    curr_live: &TaskContext,
    curr_dead: &TaskContext,
    new_live: &TaskContext,
    new_dead: &TaskContext,
) -> (TaskContext, TaskContext) {
    let index = |live: &TaskContext, dead: &TaskContext| -> HashMap<StageKey, (Side, StageContext)> {
        let mut m = HashMap::new();
        for s in &live.stages {
            m.insert(stage_key(s), (Side::Live, s.clone()));
        }
        for s in &dead.stages {
            m.insert(stage_key(s), (Side::Dead, s.clone()));
        }
        m
    };
    let orig = index(orig_live, orig_dead);
    let curr = index(curr_live, curr_dead);
    let new = index(new_live, new_dead);

    let mut keys: HashSet<StageKey> = HashSet::new();
    keys.extend(orig.keys().cloned());
    keys.extend(curr.keys().cloned());
    keys.extend(new.keys().cloned());

    let mut live_out: Vec<StageContext> = Vec::new();
    let mut dead_out: Vec<StageContext> = Vec::new();

    for key in keys {
        let orig_entry = orig.get(&key);
        let curr_entry = curr.get(&key);
        let new_entry = new.get(&key);

        // Pick side: three-way, then ours-wins on conflict.
        let side = match (orig_entry, curr_entry, new_entry) {
            (None, Some(c), None) => Some(c.0),
            (None, None, Some(n)) => Some(n.0),
            (None, Some(_), Some(n)) => Some(n.0),
            (Some(_), Some(c), None) => {
                // We removed; if they also removed or moved, no conflict. If
                // they kept it unchanged, ours (remove) still wins.
                if Some(c.0) != orig_entry.map(|o| o.0) {
                    // They moved sides; we removed. Ours wins: removed.
                    None
                } else {
                    None
                }
            }
            (Some(o), None, Some(n)) => {
                // They removed. We kept. Ours wins: keep on our side.
                // But if we didn't touch it (n == o), honour their removal.
                if n == o { None } else { Some(n.0) }
            }
            (Some(_), None, None) => None,
            (Some(o), Some(c), Some(n)) => {
                if n.0 == o.0 {
                    Some(c.0)
                } else if c.0 == o.0 {
                    Some(n.0)
                } else {
                    Some(n.0)
                }
            }
            (None, None, None) => None,
        };
        let Some(side) = side else { continue };

        let orig_stage = orig_entry.map(|(_, s)| s);
        let curr_stage = curr_entry.map(|(_, s)| s);
        let new_stage = new_entry.map(|(_, s)| s);

        // Merge the stage content itself. Records are merged per-id; info is
        // taken from whichever version actually has the stage (ours → theirs →
        // original).
        let merged = merge_stage(orig_stage, curr_stage, new_stage);
        let Some(merged) = merged else { continue };

        match side {
            Side::Live => live_out.push(merged),
            Side::Dead => dead_out.push(merged),
        }
    }

    live_out.sort_by_key(|s| s.info.timestamp);
    dead_out.sort_by_key(|s| s.info.timestamp);

    (
        TaskContext { stages: live_out },
        TaskContext { stages: dead_out },
    )
}

/// Three-way merge a single stage by reconciling its records by id. Returns
/// `None` when the stage should be dropped (both sides removed it).
fn merge_stage(
    orig: Option<&StageContext>,
    curr: Option<&StageContext>,
    new: Option<&StageContext>,
) -> Option<StageContext> {
    // Info rarely changes after stage creation, so pick whichever version is
    // present (preferring ours so record/info stay in sync on conflicts).
    let info = new
        .or(curr)
        .or(orig)
        .map(|s| s.info.clone())
        .expect("at least one version must be present");

    let empty: Vec<ContextRecord> = Vec::new();
    let orig_records = orig.map(|s| &s.records).unwrap_or(&empty);
    let curr_records = curr.map(|s| &s.records).unwrap_or(&empty);
    let new_records = new.map(|s| &s.records).unwrap_or(&empty);

    let records = merge_records(orig_records, curr_records, new_records);
    Some(StageContext { info, records })
}

fn merge_records(
    orig: &[ContextRecord],
    curr: &[ContextRecord],
    new: &[ContextRecord],
) -> Vec<ContextRecord> {
    let by_id = |xs: &[ContextRecord]| -> HashMap<u64, ContextRecord> {
        xs.iter().map(|r| (r.id, r.clone())).collect()
    };
    let o = by_id(orig);
    let c = by_id(curr);
    let n = by_id(new);

    let mut ids: HashSet<u64> = HashSet::new();
    ids.extend(o.keys().copied());
    ids.extend(c.keys().copied());
    ids.extend(n.keys().copied());
    let mut ids: Vec<u64> = ids.into_iter().collect();
    ids.sort();

    let mut out = Vec::new();
    for id in ids {
        let chosen = match (o.get(&id), c.get(&id), n.get(&id)) {
            (None, Some(r), None) => Some(r.clone()),
            (None, None, Some(r)) => Some(r.clone()),
            (None, Some(_), Some(r)) => Some(r.clone()),
            (Some(_), Some(_), None) => None,
            (Some(orig_r), None, Some(new_r)) => {
                if new_r == orig_r { None } else { Some(new_r.clone()) }
            }
            (Some(_), None, None) => None,
            (Some(orig_r), Some(curr_r), Some(new_r)) => {
                if new_r == orig_r {
                    Some(curr_r.clone())
                } else if curr_r == orig_r {
                    Some(new_r.clone())
                } else {
                    Some(new_r.clone())
                }
            }
            (None, None, None) => None,
        };
        if let Some(r) = chosen {
            out.push(r);
        }
    }
    out
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
            serialize_description_full("my task", &HashMap::new(), &None, &ctx, &[], None, &TaskContext::default());
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
            &TaskContext::default(),
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
            serialize_description_full("my task", &params, &status, &ctx, &[], None, &TaskContext::default());
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
            &TaskContext::default(),
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
            &TaskContext::default(),
        );

        // They changed the status
        let current = serialize_description_full(
            "original desc",
            &HashMap::new(),
            &Some("their error".to_string()),
            &TaskContext::default(),
            &[],
            None,
            &TaskContext::default(),
        );

        // We changed the context
        let our_new = serialize_description_full(
            "original desc",
            &HashMap::new(),
            &None,
            &sample_context(),
            &[],
            None,
            &TaskContext::default(),
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
            serialize_description_full("desc", &HashMap::new(), &None, &ctx1, &[], None, &TaskContext::default());

        // They changed the context
        let ctx_theirs = TaskContext {
            stages: vec![StageContext {
                info: StageInfo {
                    instance: "default".to_string(),
                    pipeline: Pipeline::from("main"),
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
            serialize_description_full("desc", &HashMap::new(), &None, &ctx_theirs, &[], None, &TaskContext::default());

        // We also changed the context
        let ctx_ours = TaskContext {
            stages: vec![StageContext {
                info: StageInfo {
                    instance: "default".to_string(),
                    pipeline: Pipeline::from("main"),
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
            serialize_description_full("desc", &HashMap::new(), &None, &ctx_ours, &[], None, &TaskContext::default());

        let merged = merge_concurrent_description_updates(&original, &current, &our_new).unwrap();
        let (_, _, _, ctx, _) = parse_description_full(&merged).unwrap();

        // Our change should win
        assert_eq!(ctx.stages[0].records[0].brief, "our change");
        assert_eq!(
            ctx.stages[0].records[0].record_type,
            ContextRecordType::Checkbox(true)
        );
    }

    fn stage(stage_name: &str, ts: &str, records: Vec<ContextRecord>) -> StageContext {
        StageContext {
            info: StageInfo {
                instance: "default".to_string(),
                pipeline: Pipeline::from("main"),
                stage: Stage::from(stage_name),
                tool: None,
                model: None,
                prompt_link: None,
                output_link: None,
                timestamp: ts.parse().unwrap(),
            },
            records,
        }
    }

    fn checkbox(id: u64, brief: &str, checked: bool) -> ContextRecord {
        ContextRecord {
            id,
            record_type: ContextRecordType::Checkbox(checked),
            brief: brief.to_string(),
            report_link: None,
        }
    }

    #[test]
    fn merge_handles_concurrent_stage_addition() {
        // Both sides add different stages — neither should be lost.
        let a = stage("a", "2024-01-01T00:00:00Z", vec![checkbox(1, "a1", false)]);
        let b = stage("b", "2024-01-02T00:00:00Z", vec![checkbox(2, "b1", false)]);
        let c = stage("c", "2024-01-03T00:00:00Z", vec![checkbox(3, "c1", false)]);

        let orig = TaskContext { stages: vec![a.clone()] };
        let theirs = TaskContext { stages: vec![a.clone(), b.clone()] };
        let ours = TaskContext { stages: vec![a, c.clone()] };

        let original = serialize_description_full("d", &HashMap::new(), &None, &orig, &[], None, &TaskContext::default());
        let current = serialize_description_full("d", &HashMap::new(), &None, &theirs, &[], None, &TaskContext::default());
        let our_new = serialize_description_full("d", &HashMap::new(), &None, &ours, &[], None, &TaskContext::default());

        let merged = merge_concurrent_description_updates(&original, &current, &our_new).unwrap();
        let (_, _, _, ctx, _) = parse_description_full(&merged).unwrap();
        let stages: Vec<&str> = ctx.stages.iter().map(|s| s.info.stage.as_ref()).collect();
        assert_eq!(stages, vec!["a", "b", "c"]);
    }

    #[test]
    fn merge_handles_user_move_to_dead_with_concurrent_edit() {
        // User moved stage X to DEAD_CONTEXT concurrent with us adding a new
        // stage Y to live. Result: X must not appear in both sections.
        let x = stage("x", "2024-01-01T00:00:00Z", vec![checkbox(1, "x1", false)]);
        let y = stage("y", "2024-01-02T00:00:00Z", vec![checkbox(2, "y1", false)]);

        let orig_live = TaskContext { stages: vec![x.clone()] };
        let orig_dead = TaskContext::default();

        // User moved X to dead, didn't touch live.
        let curr_live = TaskContext::default();
        let curr_dead = TaskContext { stages: vec![x.clone()] };

        // We (zbobr) added Y to live without knowing about the move.
        let new_live = TaskContext { stages: vec![x.clone(), y.clone()] };
        let new_dead = TaskContext::default();

        let original = serialize_description_full("d", &HashMap::new(), &None, &orig_live, &[], None, &orig_dead);
        let current = serialize_description_full("d", &HashMap::new(), &None, &curr_live, &[], None, &curr_dead);
        let our_new = serialize_description_full("d", &HashMap::new(), &None, &new_live, &[], None, &new_dead);

        let merged = merge_concurrent_description_updates(&original, &current, &our_new).unwrap();
        let (_, _, _, live, dead) = parse_description_full(&merged).unwrap();

        let live_stages: Vec<&str> = live.stages.iter().map(|s| s.info.stage.as_ref()).collect();
        let dead_stages: Vec<&str> = dead.stages.iter().map(|s| s.info.stage.as_ref()).collect();
        assert_eq!(live_stages, vec!["y"]);
        assert_eq!(dead_stages, vec!["x"]);
    }

    #[test]
    fn merge_unions_records_within_stage() {
        // We add record id=2 to a stage while they add record id=3 to the
        // same stage — both should survive.
        let orig = TaskContext {
            stages: vec![stage("s", "2024-01-01T00:00:00Z", vec![checkbox(1, "r1", false)])],
        };
        let theirs = TaskContext {
            stages: vec![stage(
                "s",
                "2024-01-01T00:00:00Z",
                vec![checkbox(1, "r1", false), checkbox(3, "r3", false)],
            )],
        };
        let ours = TaskContext {
            stages: vec![stage(
                "s",
                "2024-01-01T00:00:00Z",
                vec![checkbox(1, "r1", false), checkbox(2, "r2", false)],
            )],
        };

        let original = serialize_description_full("d", &HashMap::new(), &None, &orig, &[], None, &TaskContext::default());
        let current = serialize_description_full("d", &HashMap::new(), &None, &theirs, &[], None, &TaskContext::default());
        let our_new = serialize_description_full("d", &HashMap::new(), &None, &ours, &[], None, &TaskContext::default());

        let merged = merge_concurrent_description_updates(&original, &current, &our_new).unwrap();
        let (_, _, _, ctx, _) = parse_description_full(&merged).unwrap();
        let ids: Vec<u64> = ctx.stages[0].records.iter().map(|r| r.id).collect();
        assert_eq!(ids, vec![1, 2, 3]);
    }
}
