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
    // Visible stage header as a top-level list item
    out.push_str(&format!(
        "- **{} #{} {}**",
        stage.info.pipeline, stage.info.run_id, stage.info.stage,
    ));

    // Show tool and model visibly
    if let Some(tool) = &stage.info.tool {
        out.push_str(&format!(" `{}`", tool));
    }
    if let Some(model) = &stage.info.model {
        out.push_str(&format!(" `{}`", model));
    }

    // HTML comment with full metadata for parsing
    out.push_str(&format!(
        " <!-- Stage: {} #{} {} [{}]",
        stage.info.pipeline, stage.info.run_id, stage.info.stage, stage.info.timestamp,
    ));
    if let Some(tool) = &stage.info.tool {
        out.push_str(&format!(" tool={}", tool));
    }
    if let Some(model) = &stage.info.model {
        out.push_str(&format!(" model={}", model));
    }
    if !for_prompt {
        if let Some(prompt_link) = &stage.info.prompt_link {
            out.push_str(&format!(" prompt={}", prompt_link));
        }
    }
    out.push_str(" -->\n");

    // Records (indented as sub-items)
    for record in &stage.records {
        out.push_str("  ");
        serialize_record(out, record, report_url);
    }

    out.push('\n');
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
        let url = match report_url {
            Some(f) => f(filename),
            None => filename.clone(),
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

        // Parse stage header — look for <!-- Stage: anywhere in the line
        // (new format has visible text before the HTML comment)
        if let Some(pos) = trimmed.find("<!-- Stage: ") {
            let header = &trimmed[pos + "<!-- Stage: ".len()..];
            let stage = parse_stage_header(header)
                .with_context(|| format!("Failed to parse stage header: {}", trimmed))?;
            stages.push(stage);
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

        // Skip other HTML comments
        if trimmed.starts_with("<!--") {
            continue;
        }

        bail!("Unrecognized line in context: {}", trimmed);
    }

    Ok(TaskContext { stages })
}

/// Parse a stage header after the `<!-- Stage: ` prefix.
/// Expected format: `{pipeline} #{run_id} {stage} [{timestamp}] [key=value...] -->`
fn parse_stage_header(header: &str) -> Result<StageContext> {
    let header = header
        .strip_suffix("-->")
        .map(|s| s.trim())
        .ok_or_else(|| anyhow::anyhow!("Stage header missing closing -->"))?;

    // Split into tokens, but we need to handle the [timestamp] specially
    let hash_pos = header
        .find('#')
        .ok_or_else(|| anyhow::anyhow!("Missing # in stage header"))?;

    let pipeline_str = header[..hash_pos].trim();
    let after_hash = &header[hash_pos + 1..];

    // run_id is the next token
    let space_after_run = after_hash
        .find(' ')
        .ok_or_else(|| anyhow::anyhow!("Missing stage name after run_id"))?;
    let run_id: u64 = after_hash[..space_after_run]
        .trim()
        .parse()
        .context("Invalid run_id")?;

    let after_run = &after_hash[space_after_run + 1..];

    // Find timestamp in brackets
    let bracket_open = after_run
        .find('[')
        .ok_or_else(|| anyhow::anyhow!("Missing timestamp brackets"))?;
    let bracket_close = after_run
        .find(']')
        .ok_or_else(|| anyhow::anyhow!("Missing closing timestamp bracket"))?;

    let stage_str = after_run[..bracket_open].trim();
    let timestamp = after_run[bracket_open + 1..bracket_close].trim().to_string();

    // Parse optional key=value pairs after the timestamp
    let remainder = after_run[bracket_close + 1..].trim();
    let mut tool = None;
    let mut model = None;
    let mut prompt_link = None;

    for token in remainder.split_whitespace() {
        if let Some(val) = token.strip_prefix("tool=") {
            tool = Some(val.parse().context("Invalid tool value")?);
        } else if let Some(val) = token.strip_prefix("model=") {
            model = Some(val.parse().context("Invalid model value")?);
        } else if let Some(val) = token.strip_prefix("prompt=") {
            prompt_link = Some(val.to_string());
        }
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
    // Determine record type from prefix (supports both old and new formats)
    let (record_type, rest) = if let Some(rest) = line
        .strip_prefix("- [x] ")
        .or_else(|| line.strip_prefix("- [X] "))
    {
        (ContextRecordType::Checkbox(true), rest)
    } else if let Some(rest) = line.strip_prefix("- [ ] ") {
        (ContextRecordType::Checkbox(false), rest)
    } else if let Some(rest) = line
        .strip_prefix("- ✅ ")
        .or_else(|| line.strip_prefix("✅ "))
    {
        (ContextRecordType::Success, rest)
    } else if let Some(rest) = line
        .strip_prefix("- ❌ ")
        .or_else(|| line.strip_prefix("❌ "))
    {
        (ContextRecordType::Failure, rest)
    } else if let Some(rest) = line
        .strip_prefix("- 💬 ")
        .or_else(|| line.strip_prefix("💬 "))
    {
        (ContextRecordType::Comment, rest)
    } else if let Some(rest) = line
        .strip_prefix("- ❓ ")
        .or_else(|| line.strip_prefix("❓ "))
    {
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

        // Visible stage header as list item
        assert!(output.contains("- **main #1 planning** `claude` `claude-opus-4.6`"));
        // HTML comment with full metadata
        assert!(output.contains("<!-- Stage: main #1 planning [2024-01-01T00:00:00Z]"));
        assert!(output.contains("tool=claude"));
        assert!(output.contains("model=claude-opus-4.6"));
        assert!(output.contains("prompt=prompts/plan.md"));
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

        assert!(!output.contains("prompt="));
        // Other metadata should still be present
        assert!(output.contains("tool=claude"));
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
- **main #1 working** <!-- Stage: main #1 working [2024-01-01T00:00:00Z] -->
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
- **main #1 working** <!-- Stage: main #1 working [2024-01-01T00:00:00Z] -->
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
        let stage_pos = output.find("<!-- Stage:").unwrap();
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
}
