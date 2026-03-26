use anyhow::{Context as _, Result, bail};

use crate::task::{
    Comment, ContextRecord, ContextRecordType, Stage, StageContext, StageInfo, TaskContext,
};

/// Serialize a `TaskContext` into markdown format, optionally interspersing
/// user comments (placed by timestamp).
///
/// When `for_prompt` is true, prompt links are omitted from stage headers.
///
/// `report_url` converts a report filename into a display URL for the link.
/// Pass `None` to use the filename as-is.
pub fn serialize_context(
    ctx: &TaskContext,
    comments: &[Comment],
    for_prompt: bool,
    report_url: Option<&dyn Fn(&str) -> String>,
) -> String {
    if ctx.stages.is_empty() && comments.is_empty() {
        return String::new();
    }

    let mut result = String::new();

    // Build a timeline of (timestamp, event) where event is either a stage or a comment
    enum Event<'a> {
        Stage(&'a StageContext),
        Comment(&'a Comment),
    }

    let mut events: Vec<(String, Event)> = Vec::new();
    for stage in &ctx.stages {
        events.push((stage.info.timestamp.clone(), Event::Stage(stage)));
    }
    for comment in comments {
        events.push((comment.timestamp.clone(), Event::Comment(comment)));
    }

    // Sort by timestamp (stable sort preserves insertion order for equal timestamps)
    events.sort_by(|a, b| a.0.cmp(&b.0));

    for (_ts, event) in &events {
        match event {
            Event::Stage(stage) => {
                serialize_stage(&mut result, stage, for_prompt, report_url);
            }
            Event::Comment(comment) => {
                serialize_user_comment(&mut result, comment);
            }
        }
    }

    result
}

fn serialize_stage(
    out: &mut String,
    stage: &StageContext,
    for_prompt: bool,
    report_url: Option<&dyn Fn(&str) -> String>,
) {
    // Visible stage header as a top-level list item.
    // Format: YYYY-MM-DD HH:MM:SS pipeline:run_id:**stage** `tool` `model` <sub>[prompt](...)</sub>
    out.push_str("- ");
    out.push_str(&display_timestamp(&stage.info.timestamp));
    out.push(' ');
    out.push_str(&format!(
        "{}:{}:**{}**",
        stage.info.pipeline, stage.info.run_id, stage.info.stage,
    ));

    // Show tool and model visibly
    if let Some(tool) = &stage.info.tool {
        out.push_str(&format!(" `{}`", tool));
    }
    if let Some(model) = &stage.info.model {
        out.push_str(&format!(" `{}`", model));
    }

    // Prompt link as visible <sub> element
    if !for_prompt && let Some(prompt_link) = &stage.info.prompt_link {
        let url = if prompt_link.starts_with("http://") || prompt_link.starts_with("https://") {
            prompt_link.clone()
        } else {
            match report_url {
                Some(f) => f(prompt_link),
                None => prompt_link.clone(),
            }
        };
        out.push_str(&format!(" <sub>[prompt]({})</sub>", url));
    }

    out.push('\n');

    // Records (indented as sub-items)
    for record in &stage.records {
        out.push_str("  ");
        serialize_record(out, record, report_url);
    }

    out.push('\n');
}

fn display_timestamp(ts: &str) -> String {
    // Try to parse as ISO 8601 UTC and convert to local timezone for display.
    if let Ok(utc) = ts.parse::<chrono::DateTime<chrono::Utc>>() {
        let local = utc.with_timezone(&chrono::Local);
        return local.format("%Y-%m-%d %H:%M:%S %z").to_string();
    }
    ts.to_string()
}

fn parse_title_timestamp(date: &str, time: &str, tz: Option<&str>) -> Result<String> {
    if date.len() != 10
        || &date[4..5] != "-"
        || &date[7..8] != "-"
        || time.len() != 8
        || &time[2..3] != ":"
        || &time[5..6] != ":"
    {
        bail!("Invalid stage timestamp, expected YYYY-MM-DD HH:MM:SS");
    }
    // If timezone offset is provided, parse as local time and convert to UTC.
    if let Some(tz_str) = tz {
        let local_str = format!("{} {} {}", date, time, tz_str);
        if let Ok(local) = chrono::DateTime::parse_from_str(&local_str, "%Y-%m-%d %H:%M:%S %z") {
            return Ok(local.to_utc().format("%Y-%m-%dT%H:%M:%SZ").to_string());
        }
    }
    // No timezone — assume UTC (backward compatibility).
    Ok(format!("{}T{}Z", date, time))
}

fn serialize_record(
    out: &mut String,
    record: &ContextRecord,
    report_url: Option<&dyn Fn(&str) -> String>,
) {
    // Type prefix (all records are list items for proper nesting)
    match &record.record_type {
        ContextRecordType::Checkbox(false) => out.push_str("- [ ] "),
        ContextRecordType::Checkbox(true) => out.push_str("- [x] "),
        ContextRecordType::Success => out.push_str("- ✅ "),
        ContextRecordType::Failure => out.push_str("- ❌ "),
        ContextRecordType::Comment => out.push_str("- 💬 "),
        ContextRecordType::Question => out.push_str("- ❓ "),
    }

    // Brief description
    out.push_str(&record.brief);

    // Record ID suffix (with optional report link)
    let id_tag = format_record_id(record.id);
    if let Some(filename) = &record.report_link {
        let url = if filename.starts_with("http://") || filename.starts_with("https://") {
            // Already a full URL (e.g. from a previous serialization roundtrip)
            filename.clone()
        } else {
            match report_url {
                Some(f) => f(filename),
                None => filename.clone(),
            }
        };
        out.push_str(&format!(" <sub>{}</sub>\n", format_link(&id_tag, &url)));
    } else {
        out.push_str(&format!(" <sub>{}</sub>\n", id_tag));
    }
}

fn format_record_id(id: u64) -> String {
    format!("ctx_rec_{}", id)
}

fn parse_record_id(s: &str) -> Option<u64> {
    s.strip_prefix("ctx_rec_")?.parse().ok()
}

fn format_link(text: &str, url: &str) -> String {
    format!("[{}]({})", text, url)
}

fn serialize_user_comment(out: &mut String, comment: &Comment) {
    // User comments as blockquotes with timestamp
    out.push_str(&format!("> **[{}]** ", comment.timestamp));
    for (i, line) in comment.text.lines().enumerate() {
        if i > 0 {
            out.push_str("\n> ");
        }
        out.push_str(line);
    }
    out.push_str("\n\n");
}

/// Parse markdown-formatted context back into a `TaskContext`.
///
/// Blockquote lines (user comments) are ignored during parsing.
/// Returns `Err` on any parse failure.
pub fn parse_context(text: &str) -> Result<TaskContext> {
    let mut stages: Vec<StageContext> = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();

        // Skip empty lines
        if trimmed.is_empty() {
            continue;
        }

        // Skip blockquote lines (user comments)
        if trimmed.starts_with('>') {
            continue;
        }

        // Parse record lines (may be indented as sub-items in new format)
        if let Some(record) = parse_record_line(trimmed)? {
            let current_stage = stages.last_mut().ok_or_else(|| {
                anyhow::anyhow!("Context record found before any stage header: {}", trimmed)
            })?;
            current_stage.records.push(record);
            continue;
        }

        // Parse stage title line.
        if trimmed.starts_with("- ") {
            let stage = parse_stage_title(trimmed)
                .with_context(|| format!("Failed to parse stage title: {}", trimmed))?;
            stages.push(stage);
            continue;
        }

        bail!("Unrecognized line in context: {}", trimmed);
    }

    Ok(TaskContext { stages })
}

/// Parse stage title line.
/// Expected format:
/// `- YYYY-MM-DD HH:MM:SS pipeline:run_id:**stage** [\`tool\`] [\`model\`] [<sub>[prompt](url)</sub>]`
fn parse_stage_title(line: &str) -> Result<StageContext> {
    let body = line
        .strip_prefix("- ")
        .ok_or_else(|| anyhow::anyhow!("Stage title must start with '- '"))?
        .trim();

    let mut parts = body.splitn(4, ' ');
    let date = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("Missing stage date"))?;
    let time = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("Missing stage time"))?;
    let third = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("Missing stage pipeline/run/stage"))?
        .trim();

    // Third token may be a timezone offset (e.g. +0300) or the pipeline:run:**stage** part.
    let (tz, mut rest) = if third.starts_with('+') || third.starts_with('-') {
        let rest = parts
            .next()
            .ok_or_else(|| anyhow::anyhow!("Missing stage pipeline/run/stage after timezone"))?
            .trim();
        (Some(third), rest)
    } else {
        (None, third)
    };

    let timestamp = parse_title_timestamp(date, time, tz)?;

    let stage_marker = ":**";
    let marker_pos = rest
        .find(stage_marker)
        .ok_or_else(|| anyhow::anyhow!("Missing ':**' stage marker"))?;
    let pipeline_and_run = &rest[..marker_pos];
    rest = &rest[marker_pos + stage_marker.len()..];

    let sep_pos = pipeline_and_run
        .rfind(':')
        .ok_or_else(|| anyhow::anyhow!("Missing ':' between pipeline and run_id"))?;
    let pipeline_str = pipeline_and_run[..sep_pos].trim();
    let run_id: u64 = pipeline_and_run[sep_pos + 1..]
        .trim()
        .parse()
        .context("Invalid run_id")?;

    let stage_end = rest
        .find("**")
        .ok_or_else(|| anyhow::anyhow!("Missing closing '**' for stage"))?;
    let stage_str = rest[..stage_end].trim();
    rest = rest[stage_end + 2..].trim();

    let mut tool = None;
    let mut model = None;
    let mut prompt_link = None;

    while !rest.is_empty() {
        if let Some(after_tick) = rest.strip_prefix('`') {
            let tick_end = after_tick
                .find('`')
                .ok_or_else(|| anyhow::anyhow!("Unclosed backtick token in stage title"))?;
            let value = &after_tick[..tick_end];
            if tool.is_none() {
                tool = Some(value.parse().context("Invalid tool value")?);
            } else if model.is_none() {
                model = Some(value.parse().context("Invalid model value")?);
            }
            rest = after_tick[tick_end + 1..].trim();
            continue;
        }

        if let Some(after_sub_open) = rest.strip_prefix("<sub>") {
            let sub_end = after_sub_open
                .find("</sub>")
                .ok_or_else(|| anyhow::anyhow!("Unclosed <sub> token in stage title"))?;
            let inner = &after_sub_open[..sub_end];
            if let Some((before_link, after_link)) = inner.split_once("](") {
                let _label = before_link
                    .strip_prefix('[')
                    .ok_or_else(|| anyhow::anyhow!("Malformed prompt link label"))?;
                let url = after_link
                    .strip_suffix(')')
                    .ok_or_else(|| anyhow::anyhow!("Malformed prompt link URL"))?;
                prompt_link = Some(url.to_string());
            } else if !inner.trim().is_empty() {
                prompt_link = Some(inner.trim().to_string());
            }
            rest = after_sub_open[sub_end + "</sub>".len()..].trim();
            continue;
        }

        break;
    }

    Ok(StageContext {
        info: StageInfo {
            pipeline: pipeline_str.parse().unwrap(),
            run_id,
            stage: Stage::new(stage_str),
            tool,
            model,
            prompt_link,
            timestamp,
        },
        records: Vec::new(),
    })
}

/// Parse a single record line. Returns Ok(None) for unrecognized lines.
fn parse_record_line(line: &str) -> Result<Option<ContextRecord>> {
    // Determine record type from prefix
    let (record_type, rest) = if let Some(rest) = line
        .strip_prefix("- [x] ")
        .or_else(|| line.strip_prefix("- [X] "))
    {
        (ContextRecordType::Checkbox(true), rest)
    } else if let Some(rest) = line.strip_prefix("- [ ] ") {
        (ContextRecordType::Checkbox(false), rest)
    } else if let Some(rest) = line.strip_prefix("- ✅ ") {
        (ContextRecordType::Success, rest)
    } else if let Some(rest) = line.strip_prefix("- ❌ ") {
        (ContextRecordType::Failure, rest)
    } else if let Some(rest) = line.strip_prefix("- 💬 ") {
        (ContextRecordType::Comment, rest)
    } else if let Some(rest) = line.strip_prefix("- ❓ ") {
        (ContextRecordType::Question, rest)
    } else {
        return Ok(None);
    };

    // Extract <sub>...</sub> suffix containing the record ID (and optional report link)
    let sub_start = rest
        .rfind("<sub>")
        .ok_or_else(|| anyhow::anyhow!("Missing <sub> marker in: {}", line))?;
    let inner = rest[sub_start..]
        .strip_prefix("<sub>")
        .and_then(|s| s.strip_suffix("</sub>"))
        .ok_or_else(|| anyhow::anyhow!("Malformed <sub>...</sub> in: {}", line))?;

    // Inner is either a markdown link `[id_tag](url)` or a plain `id_tag`
    let (id_tag, report_link) = if let Some((before_link, after_link)) = inner.split_once("](") {
        // [id_tag](url)
        let tag = before_link
            .strip_prefix('[')
            .ok_or_else(|| anyhow::anyhow!("Malformed link in <sub> in: {}", line))?;
        let url = after_link
            .strip_suffix(')')
            .ok_or_else(|| anyhow::anyhow!("Malformed link in <sub> in: {}", line))?;
        (tag, Some(url.to_string()))
    } else {
        (inner, None)
    };

    let id = parse_record_id(id_tag)
        .ok_or_else(|| anyhow::anyhow!("Invalid record ID '{}' in: {}", id_tag, line))?;

    let brief = rest[..sub_start].trim().to_string();

    Ok(Some(ContextRecord {
        id,
        record_type,
        brief,
        report_link,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{Model, Pipeline};

    fn sample_context() -> TaskContext {
        TaskContext {
            stages: vec![
                StageContext {
                    info: StageInfo {
                        pipeline: Pipeline::from("main"),
                        run_id: 1,
                        stage: Stage::new("planning"),
                        tool: Some(crate::task::Tool::Claude),
                        model: Some(Model::ClaudeOpus4_6),
                        prompt_link: Some("prompts/plan.md".to_string()),
                        timestamp: "2024-01-01T00:00:00Z".to_string(),
                    },
                    records: vec![
                        ContextRecord {
                            id: 1,
                            record_type: ContextRecordType::Checkbox(false),
                            brief: "Define API schema".to_string(),
                            report_link: None,
                        },
                        ContextRecord {
                            id: 2,
                            record_type: ContextRecordType::Checkbox(true),
                            brief: "Review requirements".to_string(),
                            report_link: None,
                        },
                        ContextRecord {
                            id: 3,
                            record_type: ContextRecordType::Success,
                            brief: "Plan completed".to_string(),
                            report_link: Some("reports/plan_success.md".to_string()),
                        },
                    ],
            
                },
                StageContext {
                    info: StageInfo {
                        pipeline: Pipeline::from("main"),
                        run_id: 1,
                        stage: Stage::new("working"),
                        tool: None,
                        model: None,
                        prompt_link: None,
                        timestamp: "2024-01-01T01:00:00Z".to_string(),
                    },
                    records: vec![
                        ContextRecord {
                            id: 4,
                            record_type: ContextRecordType::Failure,
                            brief: "Build failed".to_string(),
                            report_link: Some("reports/build_fail.md".to_string()),
                        },
                        ContextRecord {
                            id: 5,
                            record_type: ContextRecordType::Comment,
                            brief: "Retrying with fix".to_string(),
                            report_link: None,
                        },
                        ContextRecord {
                            id: 6,
                            record_type: ContextRecordType::Question,
                            brief: "Should we use async?".to_string(),
                            report_link: None,
                        },
                    ],
            
                },
            ],
        }
    }

    #[test]
    fn serialize_basic() {
        let ctx = sample_context();
        let output = serialize_context(&ctx, &[], false, None);

        // Timestamp includes local timezone offset, so match the pipeline/stage part.
        assert!(output.contains("main:1:**planning** `claude` `claude-opus-4.6`"));
        // Prompt link visible as <sub> element
        assert!(output.contains("<sub>[prompt](prompts/plan.md)</sub>"));
        // Records indented as sub-items with list prefix
        assert!(output.contains("  - [ ] Define API schema"));
        assert!(output.contains("  - [x] Review requirements"));
        assert!(output.contains("  - ✅ Plan completed <sub>[ctx_rec_3](reports/plan_success.md)</sub>"));
        assert!(output.contains("  - ❌ Build failed <sub>[ctx_rec_4](reports/build_fail.md)</sub>"));
        assert!(output.contains("  - 💬 Retrying with fix"));
        assert!(output.contains("  - ❓ Should we use async?"));
    }

    #[test]
    fn serialize_for_prompt_omits_prompt_link() {
        let ctx = sample_context();
        let output = serialize_context(&ctx, &[], true, None);

        assert!(!output.contains("<sub>[prompt]"));
        assert!(output.contains("`claude` `claude-opus-4.6`"));
    }

    #[test]
    fn parse_basic() {
        let ctx = sample_context();
        let serialized = serialize_context(&ctx, &[], false, None);
        let parsed = parse_context(&serialized).unwrap();

        assert_eq!(parsed.stages.len(), 2);

        let s0 = &parsed.stages[0];
        assert_eq!(s0.info.pipeline, Pipeline::from("main"));
        assert_eq!(s0.info.run_id, 1);
        assert_eq!(s0.info.stage, Stage::new("planning"));
        assert_eq!(s0.info.timestamp, "2024-01-01T00:00:00Z");
        assert!(s0.info.prompt_link.as_deref() == Some("prompts/plan.md"));
        assert_eq!(s0.records.len(), 3);

        assert_eq!(s0.records[0].id, 1);
        assert_eq!(s0.records[0].record_type, ContextRecordType::Checkbox(false));
        assert_eq!(s0.records[0].brief, "Define API schema");

        assert_eq!(s0.records[1].id, 2);
        assert_eq!(s0.records[1].record_type, ContextRecordType::Checkbox(true));

        assert_eq!(s0.records[2].id, 3);
        assert_eq!(s0.records[2].record_type, ContextRecordType::Success);
        assert_eq!(
            s0.records[2].report_link.as_deref(),
            Some("reports/plan_success.md")
        );
    }

    #[test]
    fn roundtrip_preserves_data() {
        let original = sample_context();
        let serialized = serialize_context(&original, &[], false, None);
        let parsed = parse_context(&serialized).unwrap();

        assert_eq!(parsed.stages.len(), original.stages.len());
        for (orig_stage, parsed_stage) in original.stages.iter().zip(parsed.stages.iter()) {
            assert_eq!(parsed_stage.info.pipeline, orig_stage.info.pipeline);
            assert_eq!(parsed_stage.info.run_id, orig_stage.info.run_id);
            assert_eq!(parsed_stage.info.stage, orig_stage.info.stage);
            assert_eq!(parsed_stage.info.timestamp, orig_stage.info.timestamp);
            assert_eq!(parsed_stage.info.tool, orig_stage.info.tool);
            assert_eq!(parsed_stage.info.model, orig_stage.info.model);
            assert_eq!(parsed_stage.info.prompt_link, orig_stage.info.prompt_link);
            assert_eq!(parsed_stage.records.len(), orig_stage.records.len());
            for (orig_rec, parsed_rec) in
                orig_stage.records.iter().zip(parsed_stage.records.iter())
            {
                assert_eq!(parsed_rec.id, orig_rec.id);
                assert_eq!(parsed_rec.record_type, orig_rec.record_type);
                assert_eq!(parsed_rec.brief, orig_rec.brief);
                assert_eq!(parsed_rec.report_link, orig_rec.report_link);
            }
        }
    }

    #[test]
    fn roundtrip_for_prompt_loses_prompt_link() {
        let original = sample_context();
        let serialized = serialize_context(&original, &[], true, None);
        let parsed = parse_context(&serialized).unwrap();

        // prompt_link should be None since for_prompt=true omitted it
        assert!(parsed.stages[0].info.prompt_link.is_none());
    }

    #[test]
    fn parse_ignores_blockquote_comments() {
        let text = "\
- 2024-01-01 00:00:00 main:1:**working**
  - [ ] Do work <sub>ctx_rec_1</sub>

> **[2024-01-01T00:30:00Z]** User says hello
> second line of comment

  - ✅ Done <sub>[ctx_rec_2](r.md)</sub>
";
        let parsed = parse_context(text).unwrap();
        assert_eq!(parsed.stages.len(), 1);
        assert_eq!(parsed.stages[0].records.len(), 2);
        assert_eq!(parsed.stages[0].records[0].brief, "Do work");
        assert_eq!(parsed.stages[0].records[1].brief, "Done");
        assert_eq!(
            parsed.stages[0].records[1].report_link.as_deref(),
            Some("r.md")
        );
    }

    #[test]
    fn parse_error_on_record_before_stage() {
        let text = "  - [ ] orphan item <sub>ctx_rec_1</sub>\n";
        let result = parse_context(text);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("before any stage header"));
    }

    #[test]
    fn parse_error_on_missing_id() {
        let text = "\
- 2024-01-01 00:00:00 main:1:**working**
  - [ ] no id marker
";
        let result = parse_context(text);
        assert!(result.is_err());
    }

    #[test]
    fn serialize_with_interspersed_comments() {
        let ctx = TaskContext {
            stages: vec![StageContext {
                info: StageInfo {
                    pipeline: Pipeline::from("main"),
                    run_id: 1,
                    stage: Stage::new("working"),
                    tool: None,
                    model: None,
                    prompt_link: None,
                    timestamp: "2024-01-01T00:00:00Z".to_string(),
                },
                records: vec![ContextRecord {
                    id: 1,
                    record_type: ContextRecordType::Checkbox(false),
                    brief: "Task A".to_string(),
                    report_link: None,
                }],
        
            }],
        };

        let comments = vec![Comment {
            timestamp: "2024-01-01T00:30:00Z".to_string(),
            stage: String::new(),
            hostname: String::new(),
            tool: None,
            model: None,
            text: "Please hurry up!".to_string(),
            pipeline: String::new(),
            pipeline_run_id: 0,
            caller_pipeline: None,
            caller_pipeline_run_id: None,
            report_name: None,
            prompt_name: None,
        }];

        let output = serialize_context(&ctx, &comments, false, None);

        // Stage should come before comment (by timestamp)
        let stage_pos = output.find("main:1:**working**").unwrap();
        let comment_pos = output.find("> **[2024-01-01T00:30:00Z]**").unwrap();
        assert!(stage_pos < comment_pos);
        assert!(output.contains("Please hurry up!"));
    }

    #[test]
    fn empty_context() {
        let ctx = TaskContext::default();
        let output = serialize_context(&ctx, &[], false, None);
        assert_eq!(output, "");

        let parsed = parse_context("").unwrap();
        assert!(parsed.stages.is_empty());
    }

    #[test]
    fn full_url_not_prefixed_again() {
        // Simulate a roundtrip: report_link already contains a full URL
        // (as parsed from a previous serialization with report_url applied)
        let ctx = TaskContext {
            stages: vec![StageContext {
                info: StageInfo {
                    pipeline: Pipeline::from("main"),
                    run_id: 1,
                    stage: Stage::new("working"),
                    tool: None,
                    model: None,
                    prompt_link: Some("https://github.com/org/repo/blob/reports/reports/task_1/prompt.md".to_string()),
                    timestamp: "2024-01-01T00:00:00Z".to_string(),
                },
                records: vec![ContextRecord {
                    id: 1,
                    record_type: ContextRecordType::Success,
                    brief: "Done".to_string(),
                    report_link: Some("https://github.com/org/repo/blob/reports/reports/task_1/report.md".to_string()),
                }],
            }],
        };

        let prefix = "https://github.com/org/repo/blob/reports/reports/task_1/";
        let make_url = |filename: &str| -> String { format!("{prefix}{filename}") };
        let output = serialize_context(&ctx, &[], false, Some(&make_url));

        // The URL should appear exactly once, not doubled
        assert!(output.contains("[ctx_rec_1](https://github.com/org/repo/blob/reports/reports/task_1/report.md)"));
        assert!(!output.contains("https://github.com/org/repo/blob/reports/reports/task_1/https://"));
        // Prompt link should also not be doubled
        assert!(output.contains("[prompt](https://github.com/org/repo/blob/reports/reports/task_1/prompt.md)"));
    }
}
