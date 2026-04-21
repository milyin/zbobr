//! Context serialization/deserialization module.
//!
//! Each stage is serialized as an independent `<!-- zbobr-ctx-v1-stage ... -->`
//! JSON record placed immediately before its markdown rendering.  The JSON is
//! the authoritative source of truth; the markdown below is the human-readable
//! view regenerated on every write.
//!
//! Agent prompts (`for_prompt = true`) never see the JSON blocks — only the
//! rendered markdown — so models don't get fed the raw structured payload.

mod json;
mod stage_title;

use std::{borrow::Cow, fmt};

use anyhow::Result;
use chrono::{DateTime, FixedOffset};
use stage_title::MdStageTitle;
pub use stage_title::format_timestamp;

use crate::task::{Comment, ContextRecord, ContextRecordType, StageContext, TaskContext};

// ────────────────────────────────────────────────────────────────────────────────
// Markdown record type
// ────────────────────────────────────────────────────────────────────────────────

/// Record type as it appears in markdown (prefix marker).
#[derive(Debug, Clone, PartialEq, Eq)]
enum MdRecordType {
    CheckboxUnchecked,
    CheckboxChecked,
    Success,
    Failure,
    Comment,
    Question,
}

impl MdRecordType {
    fn prefix(&self) -> &'static str {
        match self {
            Self::CheckboxUnchecked => "- [ ] ",
            Self::CheckboxChecked => "- [x] ",
            Self::Success => "- ✅ ",
            Self::Failure => "- ❌ ",
            Self::Comment => "- 💬 ",
            Self::Question => "- ❓ ",
        }
    }
}

impl fmt::Display for MdRecordType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.prefix())
    }
}

impl From<&ContextRecordType> for MdRecordType {
    fn from(t: &ContextRecordType) -> Self {
        match t {
            ContextRecordType::Checkbox(false) => Self::CheckboxUnchecked,
            ContextRecordType::Checkbox(true) => Self::CheckboxChecked,
            ContextRecordType::Success => Self::Success,
            ContextRecordType::Failure => Self::Failure,
            ContextRecordType::Comment => Self::Comment,
            ContextRecordType::Question => Self::Question,
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────────
// Markdown record
// ────────────────────────────────────────────────────────────────────────────────

/// A context record as it appears in a single markdown line.
///
/// Format: `- [ ] brief text <sub>ctx_rec_1</sub>`
/// or with report link: `- ✅ brief <sub>[ctx_rec_1](url)</sub>`
#[derive(Debug, Clone)]
struct MdRecord {
    record_type: MdRecordType,
    brief: String,
    id: u64,
    report_link: Option<String>,
    for_prompt: bool,
}

impl fmt::Display for MdRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let brief = (if self.brief.len() <= COMPACT_COMMENT_MAX_LEN {
            Cow::Borrowed(&self.brief)
        } else {
            Cow::Owned(
                self.brief
                    .chars()
                    .take(COMPACT_COMMENT_MAX_LEN)
                    .collect::<String>(),
            )
        })
        .lines()
        .collect::<Vec<_>>()
        .join(" ");

        write!(f, "{}{}", self.record_type.prefix(), brief)?;
        let id_tag = format!("ctx_rec_{}", self.id);
        let is_interactive = matches!(
            self.record_type,
            MdRecordType::CheckboxUnchecked | MdRecordType::CheckboxChecked
        ) || self.report_link.is_some();
        if self.for_prompt {
            if is_interactive {
                write!(f, " [{}]", id_tag)
            } else {
                Ok(())
            }
        } else if let Some(url) = &self.report_link {
            write!(f, " <sub>[{}]({})</sub>", id_tag, url)
        } else {
            write!(f, " <sub>{}</sub>", id_tag)
        }
    }
}

impl MdRecord {
    /// Convert from a domain `ContextRecord`, optionally transforming report URLs.
    fn from_context_record(
        r: &ContextRecord,
        for_prompt: bool,
        report_url: Option<&dyn Fn(&str) -> String>,
    ) -> Self {
        let report_link = r.report_link.as_ref().map(|filename| {
            if filename.starts_with("http://") || filename.starts_with("https://") {
                filename.clone()
            } else {
                match report_url {
                    Some(f) => f(filename),
                    None => filename.clone(),
                }
            }
        });
        MdRecord {
            record_type: MdRecordType::from(&r.record_type),
            brief: r.brief.clone(),
            id: r.id,
            report_link,
            for_prompt,
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────────
// Compact comment (shared prompt/no-prompt mode)
// ────────────────────────────────────────────────────────────────────────────────

/// Maximum character length for the truncated comment text in compact form.
const COMPACT_COMMENT_MAX_LEN: usize = 80;

/// A compact (single-line) representation of a comment used in serialized context.
///
/// Format: `- comment text `YYYY-MM-DD HH:MM:SS +HHMM` <sub>[link](url)</sub>`
#[derive(Debug, Clone)]
struct MdCompactComment {
    text: String,
    timestamp: DateTime<FixedOffset>,
    url: Option<String>,
    for_prompt: bool,
}

impl MdCompactComment {
    fn from_comment(c: &Comment, for_prompt: bool) -> Self {
        let username = if c.username.is_empty() {
            "unknown"
        } else {
            &c.username
        };

        let text = if for_prompt {
            format!("user {}: {}", username, c.body)
        } else if c.body.len() <= COMPACT_COMMENT_MAX_LEN {
            let joined = c.body.lines().collect::<Vec<_>>().join(" ");
            format!("user:**{}** {}", username, joined)
        } else {
            let truncated = c
                .body
                .chars()
                .take(COMPACT_COMMENT_MAX_LEN)
                .collect::<String>();
            let joined = truncated.lines().collect::<Vec<_>>().join(" ");
            format!("user:**{}** {}...", username, joined)
        };

        MdCompactComment {
            text,
            timestamp: c.timestamp,
            url: c.url.clone(),
            for_prompt,
        }
    }
}

impl fmt::Display for MdCompactComment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.for_prompt {
            let mut lines = self.text.split('\n');
            let first = lines.next().unwrap_or("");
            write!(f, "- {}", first)?;
            for line in lines {
                write!(f, "\n  {}", line)?;
            }
            return Ok(());
        }
        write!(f, "- {} `{}`", self.text, format_timestamp(&self.timestamp))?;
        if let Some(url) = &self.url {
            write!(f, " <sub>[link]({})</sub>", url)?;
        }
        Ok(())
    }
}

// ────────────────────────────────────────────────────────────────────────────────
// Markdown stage
// ────────────────────────────────────────────────────────────────────────────────

/// A complete stage as it appears in markdown: title line followed by indented records.
#[derive(Debug, Clone)]
struct MdStage {
    title: MdStageTitle,
    records: Vec<MdRecord>,
    for_prompt: bool,
}

impl fmt::Display for MdStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.for_prompt {
            writeln!(f, "- {}", self.title.stage)?;
        } else {
            writeln!(f, "- {}", self.title)?;
        }

        // Flatten output: all records on the same level
        // Reorder so first non-checkbox item is first in output
        let mut ordered = self.records.clone();
        if let Some(non_checkbox_idx) = ordered.iter().position(|r| {
            !matches!(
                r.record_type,
                MdRecordType::CheckboxUnchecked | MdRecordType::CheckboxChecked
            )
        }) && non_checkbox_idx != 0
        {
            let non_checkbox = ordered.remove(non_checkbox_idx);
            ordered.insert(0, non_checkbox);
        }

        for record in ordered {
            let indent = match record.record_type {
                MdRecordType::CheckboxUnchecked | MdRecordType::CheckboxChecked => "    ",
                _ => "  ",
            };
            writeln!(f, "{}{}", indent, record)?;
        }

        Ok(())
    }
}

impl MdStage {
    /// Convert from a domain `StageContext`, applying serialization options.
    fn from_stage_context(
        stage: &StageContext,
        for_prompt: bool,
        report_url: Option<&dyn Fn(&str) -> String>,
    ) -> Self {
        let mut title = MdStageTitle::from(&stage.info);

        // Transform prompt/output link URLs if needed
        for link in [&mut title.prompt_link, &mut title.output_link]
            .into_iter()
            .flatten()
        {
            if !link.starts_with("http://")
                && !link.starts_with("https://")
                && let Some(f) = report_url
            {
                *link = f(link);
            }
        }

        // Omit prompt and output links for agent prompts
        if for_prompt {
            title.prompt_link = None;
            title.output_link = None;
        }

        let records = stage
            .records
            .iter()
            .map(|r| MdRecord::from_context_record(r, for_prompt, report_url))
            .collect();

        MdStage {
            title,
            records,
            for_prompt,
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────────
// Markdown context document (prompt-mode only)
// ────────────────────────────────────────────────────────────────────────────────

/// An entry in the context document.
#[derive(Debug, Clone)]
enum MdEntry {
    Stage(MdStage),
    CompactComment(MdCompactComment),
}

/// The complete context document in markdown format.
///
/// Only used for `for_prompt = true` rendering. Non-prompt serialization
/// emits per-stage JSON blocks directly (see `serialize_context`).
#[derive(Debug, Clone)]
struct MdContext {
    entries: Vec<MdEntry>,
}

impl fmt::Display for MdContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for entry in &self.entries {
            match entry {
                MdEntry::Stage(stage) => {
                    write!(f, "{}", stage)?;
                }
                MdEntry::CompactComment(c) => {
                    writeln!(f, "{}", c)?;
                }
            }
        }
        Ok(())
    }
}

impl MdContext {
    /// Build from domain types, applying serialization options.
    fn from_task_context(
        ctx: &TaskContext,
        comments: &[Comment],
        for_prompt: bool,
        report_url: Option<&dyn Fn(&str) -> String>,
    ) -> Self {
        let mut events: Vec<(DateTime<FixedOffset>, MdEntry)> = Vec::new();

        for stage in &ctx.stages {
            let md_stage = MdStage::from_stage_context(stage, for_prompt, report_url);
            // When rendering for agent prompts, skip stages with no records (e.g. failed stages)
            if for_prompt && md_stage.records.is_empty() {
                continue;
            }
            events.push((stage.info.timestamp, MdEntry::Stage(md_stage)));
        }

        for comment in comments {
            let entry =
                MdEntry::CompactComment(MdCompactComment::from_comment(comment, for_prompt));
            events.push((comment.timestamp, entry));
        }

        // Sort by timestamp (stable sort preserves insertion order for equal timestamps)
        events.sort_by(|a, b| a.0.cmp(&b.0));

        MdContext {
            entries: events.into_iter().map(|(_, e)| e).collect(),
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────────
// Public API
// ────────────────────────────────────────────────────────────────────────────────

/// Serialize a `TaskContext` into markdown format, optionally interspersing
/// user comments (placed by timestamp).
///
/// When `for_prompt` is true, prompt links are omitted from stage headers and
/// no JSON blocks are emitted — the prompt sees only the rendered human view.
///
/// When `for_prompt` is false each stage is preceded by an independent
/// `<!-- zbobr-ctx-v1-stage ... -->` JSON record.  The records are the source
/// of truth; the markdown that follows them is the human-readable view.
/// Because every stage has its own self-contained record, the user can move
/// the `---CONTEXT---` separator to any line between two stage blocks and the
/// parser will correctly route everything above the separator to `DEAD_CONTEXT`
/// and everything below to the live `CONTEXT`.
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

    if for_prompt {
        let md = MdContext::from_task_context(ctx, comments, true, report_url);
        return md.to_string();
    }

    // Non-prompt: emit one JSON record per stage, interleaved with compact
    // comments, all sorted by timestamp.
    enum Event<'a> {
        Stage(&'a StageContext),
        Comment(&'a Comment),
    }

    let mut events: Vec<(DateTime<FixedOffset>, Event<'_>)> = Vec::new();
    for stage in &ctx.stages {
        events.push((stage.info.timestamp, Event::Stage(stage)));
    }
    for comment in comments {
        events.push((comment.timestamp, Event::Comment(comment)));
    }
    events.sort_by(|a, b| a.0.cmp(&b.0));

    let mut result = String::new();
    for (_, event) in events {
        match event {
            Event::Stage(stage_ctx) => {
                result.push_str(&json::serialize_stage_json_block(stage_ctx));
                result.push('\n');
                let md_stage =
                    MdStage::from_stage_context(stage_ctx, false, report_url);
                result.push_str(&md_stage.to_string());
            }
            Event::Comment(comment) => {
                let md_comment = MdCompactComment::from_comment(comment, false);
                result.push_str(&format!("{}\n", md_comment));
            }
        }
    }
    result
}

/// Parse context text back into a `TaskContext`.
///
/// Expects `<!-- zbobr-ctx-v1-stage ... -->` blocks (current format).
/// Returns an empty `TaskContext` when no blocks are present.
/// A present-but-malformed block is always an error.
pub fn parse_context(text: &str) -> Result<TaskContext> {
    match json::parse_stage_json_blocks(text) {
        Some(result) => result.map(|stages| TaskContext { stages }),
        None => Ok(TaskContext::default()),
    }
}

// ────────────────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{Pipeline, Stage, StageInfo};

    fn utc(s: &str) -> chrono::DateTime<chrono::FixedOffset> {
        s.parse::<chrono::DateTime<chrono::FixedOffset>>().unwrap()
    }

    fn sample_context() -> TaskContext {
        TaskContext {
            stages: vec![
                StageContext {
                    info: StageInfo {
                        instance: "default".to_string(),
                        pipeline: Pipeline::from("main"),
                        stage: Stage::new("planning"),
                        tool: Some("claude".to_string()),
                        model: Some("claude-opus-4.6".parse().unwrap()),
                        prompt_link: Some("prompts/plan.md".to_string()),
                        output_link: None,
                        timestamp: utc("2024-01-01T00:00:00Z"),
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
                        instance: "default".to_string(),
                        pipeline: Pipeline::from("main"),
                        stage: Stage::new("working"),
                        tool: None,
                        model: None,
                        prompt_link: None,
                        output_link: None,
                        timestamp: utc("2024-01-01T01:00:00Z"),
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

        assert!(output.contains("default:main:**planning** `claude` `claude-opus-4.6`"));
        assert!(
            output.contains("`2024-01-01 00:00:00 +0000` <sub>[prompt](prompts/plan.md)</sub>")
        );
        assert!(output.contains("  - [ ] Define API schema"));
        assert!(output.contains("  - [x] Review requirements"));
        assert!(
            output
                .contains("  - ✅ Plan completed <sub>[ctx_rec_3](reports/plan_success.md)</sub>")
        );
        assert!(
            output.contains("  - ❌ Build failed <sub>[ctx_rec_4](reports/build_fail.md)</sub>")
        );
        assert!(output.contains("  - 💬 Retrying with fix"));
        assert!(output.contains("  - ❓ Should we use async?"));
    }

    #[test]
    fn serialize_for_prompt_omits_prompt_link() {
        let ctx = sample_context();
        let output = serialize_context(&ctx, &[], true, None);

        assert!(output.contains("- planning\n"));
        assert!(output.contains("- working\n"));
        assert!(!output.contains("`claude`"));
        assert!(!output.contains("`claude-opus-4.6`"));
        assert!(!output.contains("](prompts/plan.md)"));
        assert!(output.contains("[ctx_rec_1]"), "checkbox should show ID");
        assert!(output.contains("[ctx_rec_2]"), "checkbox should show ID");
        assert!(
            output.contains("[ctx_rec_3]"),
            "success with link should show ID"
        );
        assert!(
            output.contains("[ctx_rec_4]"),
            "failure with link should show ID"
        );
        assert!(
            !output.contains("[ctx_rec_5]"),
            "comment without link should suppress ID in prompt mode"
        );
        assert!(
            !output.contains("[ctx_rec_6]"),
            "question without link should suppress ID in prompt mode"
        );
        assert!(
            output.contains("Retrying with fix"),
            "comment brief should appear"
        );
        assert!(
            output.contains("Should we use async?"),
            "question brief should appear"
        );
    }

    #[test]
    fn parse_basic() {
        let ctx = sample_context();
        let serialized = serialize_context(&ctx, &[], false, None);
        let parsed = parse_context(&serialized).unwrap();

        assert_eq!(parsed.stages.len(), 2);

        let s0 = &parsed.stages[0];
        assert_eq!(s0.info.pipeline, Pipeline::from("main"));
        assert_eq!(s0.info.stage, Stage::new("planning"));
        assert_eq!(s0.info.timestamp, utc("2024-01-01T00:00:00Z"));
        assert!(s0.info.prompt_link.as_deref() == Some("prompts/plan.md"));
        assert_eq!(s0.records.len(), 3);

        assert_eq!(s0.records[0].id, 1);
        assert_eq!(
            s0.records[0].record_type,
            ContextRecordType::Checkbox(false)
        );
        assert_eq!(s0.records[0].brief, "Define API schema");

        assert_eq!(s0.records[1].id, 2);
        assert_eq!(s0.records[1].record_type, ContextRecordType::Checkbox(true));

        assert_eq!(s0.records[2].id, 3);
        assert_eq!(s0.records[2].record_type, ContextRecordType::Success);
        assert_eq!(s0.records[2].brief, "Plan completed");
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
            assert_eq!(parsed_stage.info.stage, orig_stage.info.stage);
            assert_eq!(parsed_stage.info.timestamp, orig_stage.info.timestamp);
            assert_eq!(parsed_stage.info.tool, orig_stage.info.tool);
            assert_eq!(parsed_stage.info.model, orig_stage.info.model);
            assert_eq!(parsed_stage.info.prompt_link, orig_stage.info.prompt_link);
            assert_eq!(parsed_stage.records.len(), orig_stage.records.len());

            let expected_ids: Vec<u64> = orig_stage.records.iter().map(|r| r.id).collect();
            let parsed_ids: Vec<u64> = parsed_stage.records.iter().map(|r| r.id).collect();
            assert_eq!(parsed_ids, expected_ids);

            for parsed_rec in &parsed_stage.records {
                let orig_rec = orig_stage
                    .records
                    .iter()
                    .find(|r| r.id == parsed_rec.id)
                    .unwrap();
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

        assert!(serialized.contains("- planning\n"));
        assert!(!serialized.contains("prompts/plan.md"));
    }

    #[test]
    fn for_prompt_also_omits_output_link() {
        let mut ctx = sample_context();
        ctx.stages[0].info.output_link = Some("outputs/plan_output.md".to_string());
        let serialized = serialize_context(&ctx, &[], true, None);

        assert!(!serialized.contains("prompts/plan.md"));
        assert!(!serialized.contains("outputs/plan_output.md"));
        assert!(serialized.contains("- planning\n"));
    }

    #[test]
    fn output_link_url_mapped_via_report_url() {
        let ctx = TaskContext {
            stages: vec![StageContext {
                info: StageInfo {
                    instance: "default".to_string(),
                    pipeline: Pipeline::from("main"),
                    stage: Stage::new("working"),
                    tool: None,
                    model: None,
                    prompt_link: None,
                    output_link: Some("output_main_1_working_end.md".to_string()),
                    timestamp: utc("2024-01-01T00:00:00Z"),
                },
                records: vec![],
            }],
        };

        let prefix = "https://github.com/org/repo/blob/reports/reports/task_1/";
        let make_url = |filename: &str| -> String { format!("{prefix}{filename}") };
        let output = serialize_context(&ctx, &[], false, Some(&make_url));

        assert!(output.contains(&format!("[output]({prefix}output_main_1_working_end.md)")));
    }

    #[test]
    fn serialize_with_interspersed_comments() {
        let ctx = TaskContext {
            stages: vec![StageContext {
                info: StageInfo {
                    instance: "default".to_string(),
                    pipeline: Pipeline::from("main"),
                    stage: Stage::new("working"),
                    tool: None,
                    model: None,
                    prompt_link: None,
                    output_link: None,
                    timestamp: utc("2024-01-01T00:00:00Z"),
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
            timestamp: utc("2024-01-01T00:30:00Z"),
            username: String::new(),
            body: "Please hurry up!".to_string(),
            url: None,
        }];

        let output = serialize_context(&ctx, &comments, false, None);

        let stage_pos = output.find("default:main:**working**").unwrap();
        let comment_pos = output.find("Please hurry up!").unwrap();
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
        let ctx = TaskContext {
            stages: vec![StageContext {
                info: StageInfo {
                    instance: "default".to_string(),
                    pipeline: Pipeline::from("main"),
                    stage: Stage::new("working"),
                    tool: None,
                    model: None,
                    prompt_link: Some(
                        "https://github.com/org/repo/blob/reports/reports/task_1/prompt.md"
                            .to_string(),
                    ),
                    output_link: None,
                    timestamp: utc("2024-01-01T00:00:00Z"),
                },
                records: vec![ContextRecord {
                    id: 1,
                    record_type: ContextRecordType::Success,
                    brief: "Done".to_string(),
                    report_link: Some(
                        "https://github.com/org/repo/blob/reports/reports/task_1/report.md"
                            .to_string(),
                    ),
                }],
            }],
        };

        let prefix = "https://github.com/org/repo/blob/reports/reports/task_1/";
        let make_url = |filename: &str| -> String { format!("{prefix}{filename}") };
        let output = serialize_context(&ctx, &[], false, Some(&make_url));

        assert!(output.contains(
            "[ctx_rec_1](https://github.com/org/repo/blob/reports/reports/task_1/report.md)"
        ));
        assert!(
            !output.contains("https://github.com/org/repo/blob/reports/reports/task_1/https://")
        );
        assert!(
            output.contains("](https://github.com/org/repo/blob/reports/reports/task_1/prompt.md)")
        );
    }

    #[test]
    fn compact_comment_appears_as_list_item() {
        let ctx = TaskContext::default();
        let comments = vec![make_comment(
            "hello world",
            "2024-01-01T00:00:00Z",
            Some("https://example.com/comment/1"),
        )];
        let output = serialize_context(&ctx, &comments, false, None);
        assert!(output.contains("- user:**unknown** hello world `2024-01-01 00:00:00 +0000` <sub>[link](https://example.com/comment/1)</sub>"));
    }

    #[test]
    fn compact_comment_without_url() {
        let ctx = TaskContext::default();
        let comments = vec![make_comment("short text", "2024-01-01T00:00:00Z", None)];
        let output = serialize_context(&ctx, &comments, false, None);
        assert!(output.contains("- user:**unknown** short text `2024-01-01 00:00:00 +0000`"));
        assert!(!output.contains("<sub>"));
    }

    #[test]
    fn compact_comment_truncates_long_text() {
        let long_text = "a".repeat(100);
        let ctx = TaskContext::default();
        let comments = vec![make_comment(&long_text, "2024-01-01T00:00:00Z", None)];
        let output = serialize_context(&ctx, &comments, false, None);
        assert!(output.contains("..."));
        assert!(output.contains(&format!("{}...", "a".repeat(COMPACT_COMMENT_MAX_LEN))));
    }

    #[test]
    fn compact_comment_replaces_cr_to_space() {
        let ctx = TaskContext::default();
        let comments = vec![make_comment(
            "first line\nsecond line\nthird line",
            "2024-01-01T00:00:00Z",
            None,
        )];
        let output = serialize_context(&ctx, &comments, false, None);
        assert!(output.contains("- user:**unknown** first line second line third line"));
        // No per-stage blocks when context is empty; the whole output is just the comment.
        let rendered_lines: Vec<_> = output.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(rendered_lines.len(), 1);
    }

    #[test]
    fn compact_comment_no_extra_cr_after_comment() {
        let ctx = TaskContext::default();
        let comments = vec![make_comment("hello world", "2024-01-01T00:00:00Z", None)];
        let output = serialize_context(&ctx, &comments, false, None);
        let rendered_lines: Vec<_> = output.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(rendered_lines.len(), 1);
    }

    #[test]
    fn compact_comment_prefixes_user() {
        let ctx = TaskContext::default();
        let comments = vec![make_comment("hello world", "2024-01-01T00:00:00Z", None)];
        let output = serialize_context(&ctx, &comments, false, None);
        assert!(output.contains("- user:**unknown** hello world `2024-01-01 00:00:00 +0000`"));
    }

    #[test]
    fn stage_json_block_added_before_each_stage_with_compact_comments() {
        let ctx = sample_context();
        let comments = vec![make_comment("a comment", "2024-01-01T00:30:00Z", None)];
        let output = serialize_context(&ctx, &comments, false, None);
        assert!(!output.contains("<!-- stage -->"));
        assert!(
            output.contains("<!-- zbobr-ctx-v1-stage\n"),
            "each stage must have its own JSON block"
        );
        let block_count = output.matches("<!-- zbobr-ctx-v1-stage\n").count();
        assert_eq!(block_count, ctx.stages.len(), "one block per stage");
    }

    #[test]
    fn stage_json_block_added_without_comments() {
        let ctx = sample_context();
        let output = serialize_context(&ctx, &[], false, None);
        assert!(!output.contains("<!-- stage -->"));
        assert!(output.contains("<!-- zbobr-ctx-v1-stage\n"));
        let block_count = output.matches("<!-- zbobr-ctx-v1-stage\n").count();
        assert_eq!(block_count, ctx.stages.len());
    }

    #[test]
    fn stage_marker_not_added_in_prompt_mode() {
        let ctx = sample_context();
        let comments = vec![make_comment("a comment", "2024-01-01T00:30:00Z", None)];
        let output = serialize_context(&ctx, &comments, true, None);
        assert!(!output.contains("<!-- stage -->"));
        assert!(!output.contains("zbobr-ctx-v1"));
    }

    #[test]
    fn compact_comment_roundtrip_preserves_context() {
        let ctx = sample_context();
        let comments = vec![make_comment(
            "comment before stage 2",
            "2024-01-01T00:30:00Z",
            None,
        )];
        let serialized = serialize_context(&ctx, &comments, false, None);
        let parsed = parse_context(&serialized).unwrap();
        assert_eq!(parsed.stages.len(), ctx.stages.len());
        assert_eq!(parsed.stages[0].info.stage, ctx.stages[0].info.stage);
    }

    #[test]
    fn for_prompt_true_uses_compact_comment_format() {
        let ctx = TaskContext::default();
        let comments = vec![make_comment("a user comment", "2024-01-01T00:00:00Z", None)];
        let output = serialize_context(&ctx, &comments, true, None);
        assert!(!output.contains("> **["));
        assert!(output.contains("- user unknown: a user comment"));
        assert!(!output.contains("`2024-01-01 00:00:00 +0000`"));
    }

    #[test]
    fn for_prompt_true_does_not_truncate_long_comment_text() {
        let long_text = "a".repeat(100);
        let ctx = TaskContext::default();
        let comments = vec![make_comment(&long_text, "2024-01-01T00:00:00Z", None)];
        let output = serialize_context(&ctx, &comments, true, None);
        assert!(output.contains(&format!("- user unknown: {}", long_text)));
        assert!(!output.contains("..."));
    }

    // -- Display impl unit tests for for_prompt=true --

    #[test]
    fn md_record_display_for_prompt() {
        let record = MdRecord {
            record_type: MdRecordType::Success,
            brief: "Plan completed".to_string(),
            id: 7,
            report_link: Some("reports/plan.md".to_string()),
            for_prompt: true,
        };
        let rendered = record.to_string();
        assert_eq!(rendered, "- ✅ Plan completed [ctx_rec_7]");
        assert!(!rendered.contains("<sub>"));
        assert!(!rendered.contains("reports/plan.md"));
    }

    #[test]
    fn md_compact_comment_display_for_prompt() {
        let comment = Comment {
            timestamp: utc("2024-06-15T12:00:00Z"),
            username: "alice".to_string(),
            body: "please proceed".to_string(),
            url: Some("https://example.com/comment/1".to_string()),
        };
        let compact = MdCompactComment::from_comment(&comment, true);
        let rendered = compact.to_string();
        assert_eq!(rendered, "- user alice: please proceed");
        assert!(!rendered.contains("2024-06-15"));
        assert!(!rendered.contains("https://example.com"));
        assert!(!rendered.contains("<sub>"));
    }

    #[test]
    fn md_stage_display_for_prompt() {
        let stage = MdStage {
            title: MdStageTitle {
                instance: "default".to_string(),
                timestamp: utc("2024-01-01T00:00:00Z"),
                pipeline: Pipeline::from("main"),
                stage: Stage::new("planning"),
                tool: Some("claude".to_string()),
                model: Some("claude-opus-4.6".parse().unwrap()),
                prompt_link: Some("prompts/plan.md".to_string()),
                output_link: None,
            },
            records: vec![MdRecord {
                record_type: MdRecordType::Success,
                brief: "Done".to_string(),
                id: 1,
                report_link: None,
                for_prompt: true,
            }],
            for_prompt: true,
        };
        let rendered = stage.to_string();
        assert!(rendered.starts_with("- planning\n"));
        assert!(!rendered.contains("`claude`"));
        assert!(!rendered.contains("claude-opus-4.6"));
        assert!(!rendered.contains("2024-01-01"));
        assert!(!rendered.contains("prompts/plan.md"));
        assert!(
            !rendered.contains("ctx_rec_1"),
            "non-interactive success record should suppress ID in prompt mode"
        );
        assert!(rendered.contains("Done"), "record brief should appear");
    }

    // -- Empty stage filtering tests --

    #[test]
    fn for_prompt_filters_empty_stages() {
        let ctx = TaskContext {
            stages: vec![
                StageContext {
                    info: StageInfo {
                        instance: "default".to_string(),
                        pipeline: Pipeline::from("main"),
                        stage: Stage::new("planning"),
                        tool: Some("claude".to_string()),
                        model: Some("claude-opus-4.6".parse().unwrap()),
                        prompt_link: None,
                        output_link: None,
                        timestamp: utc("2024-01-01T00:00:00Z"),
                    },
                    records: vec![ContextRecord {
                        id: 1,
                        record_type: ContextRecordType::Success,
                        brief: "Plan completed".to_string(),
                        report_link: None,
                    }],
                },
                StageContext {
                    info: StageInfo {
                        instance: "default".to_string(),
                        pipeline: Pipeline::from("main"),
                        stage: Stage::new("working"),
                        tool: None,
                        model: None,
                        prompt_link: None,
                        output_link: None,
                        timestamp: utc("2024-01-01T01:00:00Z"),
                    },
                    records: vec![],
                },
                StageContext {
                    info: StageInfo {
                        instance: "default".to_string(),
                        pipeline: Pipeline::from("main"),
                        stage: Stage::new("reviewing"),
                        tool: None,
                        model: None,
                        prompt_link: None,
                        output_link: None,
                        timestamp: utc("2024-01-01T02:00:00Z"),
                    },
                    records: vec![ContextRecord {
                        id: 2,
                        record_type: ContextRecordType::Comment,
                        brief: "Looks good".to_string(),
                        report_link: None,
                    }],
                },
            ],
        };

        let prompt_output = serialize_context(&ctx, &[], true, None);
        assert!(
            prompt_output.contains("- planning\n"),
            "stage with records should appear"
        );
        assert!(
            !prompt_output.contains("working"),
            "empty stage should be filtered out in for_prompt mode"
        );
        assert!(
            prompt_output.contains("- reviewing\n"),
            "stage with records should appear"
        );

        let full_output = serialize_context(&ctx, &[], false, None);
        assert!(
            full_output.contains("**planning**"),
            "stage with records should appear"
        );
        assert!(
            full_output.contains("**working**"),
            "empty stage should NOT be filtered in normal mode"
        );
        assert!(
            full_output.contains("**reviewing**"),
            "stage with records should appear"
        );
    }

    // -- End-to-end prompt format validation --

    #[test]
    fn for_prompt_renders_complete_format() {
        let ctx = TaskContext {
            stages: vec![
                StageContext {
                    info: StageInfo {
                        instance: "skynet".to_string(),
                        pipeline: Pipeline::from("main"),
                        stage: Stage::new("planning"),
                        tool: Some("claude".to_string()),
                        model: Some("claude-opus-4.6".parse().unwrap()),
                        prompt_link: Some("prompts/plan.md".to_string()),
                        output_link: Some("outputs/plan_out.md".to_string()),
                        timestamp: utc("2024-06-01T10:00:00Z"),
                    },
                    records: vec![
                        ContextRecord {
                            id: 1,
                            record_type: ContextRecordType::Comment,
                            brief: "Plan ready for review".to_string(),
                            report_link: Some("reports/plan_review.md".to_string()),
                        },
                        ContextRecord {
                            id: 2,
                            record_type: ContextRecordType::Checkbox(true),
                            brief: "Define API schema".to_string(),
                            report_link: None,
                        },
                    ],
                },
                StageContext {
                    info: StageInfo {
                        instance: "skynet".to_string(),
                        pipeline: Pipeline::from("main"),
                        stage: Stage::new("working"),
                        tool: Some("copilot".to_string()),
                        model: Some("claude-sonnet-4.6".parse().unwrap()),
                        prompt_link: None,
                        output_link: None,
                        timestamp: utc("2024-06-01T11:00:00Z"),
                    },
                    records: vec![],
                },
                StageContext {
                    info: StageInfo {
                        instance: "skynet".to_string(),
                        pipeline: Pipeline::from("main"),
                        stage: Stage::new("reviewing"),
                        tool: Some("claude".to_string()),
                        model: Some("claude-opus-4.6".parse().unwrap()),
                        prompt_link: Some("prompts/review.md".to_string()),
                        output_link: None,
                        timestamp: utc("2024-06-01T13:00:00Z"),
                    },
                    records: vec![
                        ContextRecord {
                            id: 3,
                            record_type: ContextRecordType::Success,
                            brief: "Review passed".to_string(),
                            report_link: Some("reports/review_ok.md".to_string()),
                        },
                        ContextRecord {
                            id: 4,
                            record_type: ContextRecordType::Checkbox(true),
                            brief: "All tests green".to_string(),
                            report_link: None,
                        },
                    ],
                },
            ],
        };

        let comments = vec![
            Comment {
                timestamp: utc("2024-06-01T10:30:00Z"),
                username: "milyin".to_string(),
                body: "proceed with the plan".to_string(),
                url: Some("https://github.com/example/issues/1#comment-1".to_string()),
            },
            Comment {
                timestamp: utc("2024-06-01T12:00:00Z"),
                username: "milyin".to_string(),
                body: "looks good so far".to_string(),
                url: None,
            },
        ];

        let output = serialize_context(&ctx, &comments, true, None);

        assert!(
            !output.contains("<!-- stage -->"),
            "prompt output must not contain stage markers"
        );
        assert!(
            output.contains("- planning\n"),
            "planning stage should appear as plain name"
        );
        assert!(
            output.contains("- reviewing\n"),
            "reviewing stage should appear as plain name"
        );
        assert!(
            !output.contains("working"),
            "empty working stage should be filtered out"
        );
        assert!(!output.contains("`claude`"), "no tool metadata");
        assert!(!output.contains("claude-opus-4.6"), "no model metadata");
        assert!(
            !output.contains("2024-06-01 10:00:00"),
            "no timestamps in stage headers"
        );
        assert!(!output.contains("prompts/"), "no prompt links");
        assert!(!output.contains("outputs/"), "no output links");
        assert!(
            output.contains("[ctx_rec_1]"),
            "comment with report_link should have plain ctx_rec tag"
        );
        assert!(
            output.contains("[ctx_rec_2]"),
            "checkbox should have plain ctx_rec tag"
        );
        assert!(
            output.contains("[ctx_rec_3]"),
            "success with report_link should have plain ctx_rec tag"
        );
        assert!(
            output.contains("[ctx_rec_4]"),
            "checkbox should have plain ctx_rec tag"
        );
        assert!(!output.contains("<sub>"), "no <sub> tags in prompt output");
        assert!(
            !output.contains("reports/"),
            "no report URLs in prompt output"
        );
        assert!(
            output.contains("- user milyin: proceed with the plan"),
            "comment should use plain format"
        );
        assert!(
            output.contains("- user milyin: looks good so far"),
            "comment should use plain format"
        );
        assert!(
            !output.contains("user:**milyin**"),
            "no bold in prompt comments"
        );
        assert!(
            !output.contains("https://github.com/example"),
            "no URLs in prompt comments"
        );
        assert!(
            output.contains("  - 💬 Plan ready for review [ctx_rec_1]"),
            "non-checkbox record indented with 2 spaces"
        );
        assert!(
            output.contains("    - [x] Define API schema [ctx_rec_2]"),
            "checkbox record indented with 4 spaces"
        );

        let planning_pos = output.find("- planning").unwrap();
        let comment1_pos = output.find("proceed with the plan").unwrap();
        let comment2_pos = output.find("looks good so far").unwrap();
        let reviewing_pos = output.find("- reviewing").unwrap();
        assert!(
            planning_pos < comment1_pos
                && comment1_pos < comment2_pos
                && comment2_pos < reviewing_pos,
            "entries must be ordered chronologically"
        );
    }

    // -- Multi-line comment in for_prompt mode --

    #[test]
    fn for_prompt_preserves_multiline_comment_body() {
        let ctx = TaskContext::default();
        let multiline_body = "proceed with plan\nalso fix the bug\nand update docs";
        let comments = vec![Comment {
            timestamp: utc("2024-06-01T10:00:00Z"),
            username: "alice".to_string(),
            body: multiline_body.to_string(),
            url: Some("https://example.com/comment/1".to_string()),
        }];

        let prompt_output = serialize_context(&ctx, &comments, true, None);
        assert!(
            prompt_output.contains("proceed with plan"),
            "first line should appear in prompt mode"
        );
        assert!(
            prompt_output.contains("  also fix the bug"),
            "second line should be indented under the comment bullet"
        );
        assert!(
            prompt_output.contains("  and update docs"),
            "third line should be indented under the comment bullet"
        );
        assert!(
            prompt_output.starts_with(
                "- user alice: proceed with plan\n  also fix the bug\n  and update docs"
            ),
            "continuation lines must be indented so body bullets don't collide with stage titles"
        );

        let normal_output = serialize_context(&ctx, &comments, false, None);
        assert!(
            normal_output.contains("proceed with plan also fix the bug and update docs"),
            "lines should be joined with spaces in normal mode"
        );
    }

    // -- Unit tests for MdRecord non-interactive ID suppression in prompt mode --

    #[test]
    fn md_record_prompt_suppresses_id_for_success_without_link() {
        let record = MdRecord {
            record_type: MdRecordType::Success,
            brief: "Build passed".to_string(),
            id: 10,
            report_link: None,
            for_prompt: true,
        };
        let rendered = record.to_string();
        assert_eq!(rendered, "- ✅ Build passed");
        assert!(!rendered.contains("ctx_rec_"));
    }

    #[test]
    fn md_record_prompt_suppresses_id_for_failure_without_link() {
        let record = MdRecord {
            record_type: MdRecordType::Failure,
            brief: "Tests failed".to_string(),
            id: 11,
            report_link: None,
            for_prompt: true,
        };
        let rendered = record.to_string();
        assert_eq!(rendered, "- ❌ Tests failed");
        assert!(!rendered.contains("ctx_rec_"));
    }

    #[test]
    fn md_record_prompt_suppresses_id_for_comment_without_link() {
        let record = MdRecord {
            record_type: MdRecordType::Comment,
            brief: "Work in progress".to_string(),
            id: 12,
            report_link: None,
            for_prompt: true,
        };
        let rendered = record.to_string();
        assert_eq!(rendered, "- 💬 Work in progress");
        assert!(!rendered.contains("ctx_rec_"));
    }

    #[test]
    fn md_record_prompt_suppresses_id_for_question_without_link() {
        let record = MdRecord {
            record_type: MdRecordType::Question,
            brief: "Need clarification".to_string(),
            id: 13,
            report_link: None,
            for_prompt: true,
        };
        let rendered = record.to_string();
        assert_eq!(rendered, "- ❓ Need clarification");
        assert!(!rendered.contains("ctx_rec_"));
    }

    #[test]
    fn md_record_prompt_shows_id_for_checkbox_unchecked() {
        let record = MdRecord {
            record_type: MdRecordType::CheckboxUnchecked,
            brief: "Todo item".to_string(),
            id: 14,
            report_link: None,
            for_prompt: true,
        };
        let rendered = record.to_string();
        assert_eq!(rendered, "- [ ] Todo item [ctx_rec_14]");
    }

    #[test]
    fn md_record_prompt_shows_id_for_checkbox_checked() {
        let record = MdRecord {
            record_type: MdRecordType::CheckboxChecked,
            brief: "Done item".to_string(),
            id: 15,
            report_link: None,
            for_prompt: true,
        };
        let rendered = record.to_string();
        assert_eq!(rendered, "- [x] Done item [ctx_rec_15]");
    }

    #[test]
    fn md_record_prompt_shows_id_for_success_with_link() {
        let record = MdRecord {
            record_type: MdRecordType::Success,
            brief: "Reviewed".to_string(),
            id: 16,
            report_link: Some("reports/review.md".to_string()),
            for_prompt: true,
        };
        let rendered = record.to_string();
        assert_eq!(rendered, "- ✅ Reviewed [ctx_rec_16]");
        assert!(!rendered.contains("reports/review.md"));
        assert!(!rendered.contains("<sub>"));
    }

    #[test]
    fn md_record_prompt_shows_id_for_failure_with_link() {
        let record = MdRecord {
            record_type: MdRecordType::Failure,
            brief: "Build broke".to_string(),
            id: 17,
            report_link: Some("reports/build.md".to_string()),
            for_prompt: true,
        };
        let rendered = record.to_string();
        assert_eq!(rendered, "- ❌ Build broke [ctx_rec_17]");
        assert!(!rendered.contains("reports/build.md"));
    }

    #[test]
    fn md_record_prompt_shows_id_for_comment_with_link() {
        let record = MdRecord {
            record_type: MdRecordType::Comment,
            brief: "Plan ready".to_string(),
            id: 18,
            report_link: Some("reports/plan.md".to_string()),
            for_prompt: true,
        };
        let rendered = record.to_string();
        assert_eq!(rendered, "- 💬 Plan ready [ctx_rec_18]");
        assert!(!rendered.contains("reports/plan.md"));
    }

    #[test]
    fn md_record_normal_mode_always_shows_id() {
        for (record_type, prefix) in [
            (MdRecordType::Success, "- ✅ "),
            (MdRecordType::Failure, "- ❌ "),
            (MdRecordType::Comment, "- 💬 "),
            (MdRecordType::Question, "- ❓ "),
        ] {
            let record = MdRecord {
                record_type,
                brief: "test".to_string(),
                id: 99,
                report_link: None,
                for_prompt: false,
            };
            let rendered = record.to_string();
            assert!(
                rendered.contains("ctx_rec_99"),
                "normal mode should always show ID, got: {rendered} for prefix {prefix}"
            );
            assert!(
                rendered.contains("<sub>"),
                "normal mode should use <sub> tags, got: {rendered}"
            );
        }
    }

    // -- JSON block tests --

    #[test]
    fn serialize_includes_json_block() {
        let ctx = sample_context();
        let output = serialize_context(&ctx, &[], false, None);
        assert!(
            output.contains("<!-- zbobr-ctx-v1-stage\n"),
            "non-prompt output must embed per-stage JSON blocks"
        );
        assert!(output.contains("-->"), "JSON blocks must be closed");
        let block_count = output.matches("<!-- zbobr-ctx-v1-stage\n").count();
        assert_eq!(block_count, ctx.stages.len(), "one block per stage");
    }

    #[test]
    fn for_prompt_omits_json_block() {
        let ctx = sample_context();
        let output = serialize_context(&ctx, &[], true, None);
        assert!(
            !output.contains("zbobr-ctx-v1"),
            "agent prompts must never see JSON blocks"
        );
    }

    #[test]
    fn per_stage_json_block_wins_over_markdown_view() {
        let stage_ctx = StageContext {
            info: StageInfo {
                instance: "default".to_string(),
                pipeline: Pipeline::from("main"),
                stage: Stage::new("json_wins"),
                tool: None,
                model: None,
                prompt_link: None,
                output_link: None,
                timestamp: utc("2024-01-01T00:00:00Z"),
            },
            records: vec![ContextRecord {
                id: 42,
                record_type: ContextRecordType::Success,
                brief: "from json".to_string(),
                report_link: None,
            }],
        };
        let json_payload = serde_json::to_string_pretty(&stage_ctx).unwrap();
        let text = format!(
            "<!-- zbobr-ctx-v1-stage\n{json_payload}\n-->\n\
             - default:main:**markdown_wins** `2024-01-01 00:00:00 +0000`\n  \
             - [ ] from markdown\n"
        );

        let parsed = parse_context(&text).unwrap();
        assert_eq!(parsed.stages.len(), 1);
        assert_eq!(parsed.stages[0].info.stage, Stage::new("json_wins"));
        assert_eq!(parsed.stages[0].records[0].id, 42);
        assert_eq!(parsed.stages[0].records[0].brief, "from json");
    }

    #[test]
    fn corrupt_stage_json_block_is_hard_error() {
        let text = "\
<!-- zbobr-ctx-v1-stage
{not valid json}
-->
- default:main:**planning** `2024-01-01 00:00:00 +0000`
";
        assert!(parse_context(text).is_err());
    }

    #[test]
    fn json_roundtrip_preserves_original_record_order() {
        let ctx = sample_context();
        let serialized = serialize_context(&ctx, &[], false, None);
        let parsed = parse_context(&serialized).unwrap();
        for (orig, got) in ctx.stages.iter().zip(parsed.stages.iter()) {
            let orig_ids: Vec<u64> = orig.records.iter().map(|r| r.id).collect();
            let got_ids: Vec<u64> = got.records.iter().map(|r| r.id).collect();
            assert_eq!(got_ids, orig_ids);
        }
    }

    // -- End-to-end test with mixed interactive and non-interactive records --

    #[test]
    fn for_prompt_mixed_interactive_and_non_interactive_records() {
        let ctx = TaskContext {
            stages: vec![StageContext {
                info: StageInfo {
                    instance: "default".to_string(),
                    pipeline: Pipeline::from("main"),
                    stage: Stage::new("working"),
                    tool: Some("claude".to_string()),
                    model: Some("claude-opus-4.6".parse().unwrap()),
                    prompt_link: None,
                    output_link: None,
                    timestamp: utc("2024-01-01T00:00:00Z"),
                },
                records: vec![
                    ContextRecord {
                        id: 1,
                        record_type: ContextRecordType::Checkbox(false),
                        brief: "Implement feature".to_string(),
                        report_link: None,
                    },
                    ContextRecord {
                        id: 2,
                        record_type: ContextRecordType::Checkbox(true),
                        brief: "Write tests".to_string(),
                        report_link: None,
                    },
                    ContextRecord {
                        id: 3,
                        record_type: ContextRecordType::Success,
                        brief: "All tests passed".to_string(),
                        report_link: Some("reports/tests.md".to_string()),
                    },
                    ContextRecord {
                        id: 4,
                        record_type: ContextRecordType::Success,
                        brief: "Lint clean".to_string(),
                        report_link: None,
                    },
                    ContextRecord {
                        id: 5,
                        record_type: ContextRecordType::Failure,
                        brief: "Flaky test".to_string(),
                        report_link: None,
                    },
                    ContextRecord {
                        id: 6,
                        record_type: ContextRecordType::Failure,
                        brief: "Build error".to_string(),
                        report_link: Some("reports/build.md".to_string()),
                    },
                    ContextRecord {
                        id: 7,
                        record_type: ContextRecordType::Comment,
                        brief: "Retrying now".to_string(),
                        report_link: None,
                    },
                    ContextRecord {
                        id: 8,
                        record_type: ContextRecordType::Comment,
                        brief: "Plan ready".to_string(),
                        report_link: Some("reports/plan.md".to_string()),
                    },
                    ContextRecord {
                        id: 9,
                        record_type: ContextRecordType::Question,
                        brief: "Should we refactor?".to_string(),
                        report_link: None,
                    },
                ],
            }],
        };

        let output = serialize_context(&ctx, &[], true, None);

        assert!(output.contains("[ctx_rec_1]"), "unchecked checkbox should show ID");
        assert!(output.contains("[ctx_rec_2]"), "checked checkbox should show ID");
        assert!(output.contains("[ctx_rec_3]"), "success with link should show ID");
        assert!(output.contains("[ctx_rec_6]"), "failure with link should show ID");
        assert!(output.contains("[ctx_rec_8]"), "comment with link should show ID");

        assert!(!output.contains("[ctx_rec_4]"), "success without link should suppress ID");
        assert!(!output.contains("[ctx_rec_5]"), "failure without link should suppress ID");
        assert!(!output.contains("[ctx_rec_7]"), "comment without link should suppress ID");
        assert!(!output.contains("[ctx_rec_9]"), "question without link should suppress ID");

        assert!(output.contains("Implement feature"));
        assert!(output.contains("Write tests"));
        assert!(output.contains("All tests passed"));
        assert!(output.contains("Lint clean"));
        assert!(output.contains("Flaky test"));
        assert!(output.contains("Build error"));
        assert!(output.contains("Retrying now"));
        assert!(output.contains("Plan ready"));
        assert!(output.contains("Should we refactor?"));

        assert!(!output.contains("<sub>"), "no <sub> tags in prompt output");
        assert!(!output.contains("reports/"), "no report URLs in prompt output");

        let normal_output = serialize_context(&ctx, &[], false, None);
        for id in 1..=9 {
            assert!(
                normal_output.contains(&format!("ctx_rec_{}", id)),
                "normal mode should show ctx_rec_{id}"
            );
        }
    }

    fn make_comment(text: &str, ts: &str, url: Option<&str>) -> crate::task::Comment {
        crate::task::Comment {
            timestamp: utc(ts),
            username: String::new(),
            body: text.to_string(),
            url: url.map(str::to_string),
        }
    }
}
