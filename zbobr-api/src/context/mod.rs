//! Context serialization/deserialization module.
//!
//! Provides a two-stage conversion between domain types and markdown format:
//! 1. Domain types (`TaskContext`, `Comment`) ↔ Markdown representation types (`MdContext`)
//! 2. Markdown representation types ↔ Markdown string (via `serde::Serialize`/`Deserialize`)
//!
//! The markdown representation types are local to this module and not exported.

mod stage_title;

use chrono::{DateTime, FixedOffset};
pub use stage_title::format_timestamp;

use std::fmt;
use std::str::FromStr;

use anyhow::{Result, bail};

use crate::task::{Comment, ContextRecord, ContextRecordType, StageContext, TaskContext};
use stage_title::MdStageTitle;

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

    /// Try to strip a record type prefix from the line.
    fn strip_prefix(line: &str) -> Option<(Self, &str)> {
        if let Some(rest) = line
            .strip_prefix("- [x] ")
            .or_else(|| line.strip_prefix("- [X] "))
        {
            Some((Self::CheckboxChecked, rest))
        } else if let Some(rest) = line.strip_prefix("- [ ] ") {
            Some((Self::CheckboxUnchecked, rest))
        } else if let Some(rest) = line.strip_prefix("- ✅ ") {
            Some((Self::Success, rest))
        } else if let Some(rest) = line.strip_prefix("- ❌ ") {
            Some((Self::Failure, rest))
        } else if let Some(rest) = line.strip_prefix("- 💬 ") {
            Some((Self::Comment, rest))
        } else if let Some(rest) = line.strip_prefix("- ❓ ") {
            Some((Self::Question, rest))
        } else {
            None
        }
    }
}

impl fmt::Display for MdRecordType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.prefix())
    }
}

impl FromStr for MdRecordType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "- [ ] " => Ok(Self::CheckboxUnchecked),
            "- [x] " | "- [X] " => Ok(Self::CheckboxChecked),
            "- ✅ " => Ok(Self::Success),
            "- ❌ " => Ok(Self::Failure),
            "- 💬 " => Ok(Self::Comment),
            "- ❓ " => Ok(Self::Question),
            _ => bail!("Invalid record type prefix: {}", s),
        }
    }
}

impl serde::Serialize for MdRecordType {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for MdRecordType {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
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

impl From<&MdRecordType> for ContextRecordType {
    fn from(t: &MdRecordType) -> Self {
        match t {
            MdRecordType::CheckboxUnchecked => Self::Checkbox(false),
            MdRecordType::CheckboxChecked => Self::Checkbox(true),
            MdRecordType::Success => Self::Success,
            MdRecordType::Failure => Self::Failure,
            MdRecordType::Comment => Self::Comment,
            MdRecordType::Question => Self::Question,
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
        write!(f, "{}{}", self.record_type.prefix(), self.brief)?;
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

impl FromStr for MdRecord {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let (record_type, rest) = MdRecordType::strip_prefix(s)
            .ok_or_else(|| anyhow::anyhow!("Not a record line: {}", s))?;

        let sub_start = rest
            .rfind("<sub>")
            .ok_or_else(|| anyhow::anyhow!("Missing <sub> marker in: {}", s))?;
        let inner = rest[sub_start..]
            .strip_prefix("<sub>")
            .and_then(|s| s.strip_suffix("</sub>"))
            .ok_or_else(|| anyhow::anyhow!("Malformed <sub>...</sub> in: {}", s))?;

        let (id_tag, report_link) = if let Some((before, after)) = inner.split_once("](") {
            let tag = before
                .strip_prefix('[')
                .ok_or_else(|| anyhow::anyhow!("Malformed link in <sub> in: {}", s))?;
            let url = after
                .strip_suffix(')')
                .ok_or_else(|| anyhow::anyhow!("Malformed link in <sub> in: {}", s))?;
            (tag, Some(url.to_string()))
        } else {
            (inner, None)
        };

        let id = id_tag
            .strip_prefix("ctx_rec_")
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| anyhow::anyhow!("Invalid record ID '{}' in: {}", id_tag, s))?;

        let brief = rest[..sub_start].trim().to_string();

        Ok(MdRecord {
            record_type,
            brief,
            id,
            report_link,
            for_prompt: false,
        })
    }
}

impl serde::Serialize for MdRecord {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for MdRecord {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl MdRecord {
    /// Try to parse a line as a record. Returns `Ok(None)` for non-record lines.
    fn try_parse(line: &str) -> Result<Option<Self>> {
        if MdRecordType::strip_prefix(line).is_none() {
            return Ok(None);
        }
        Ok(Some(line.parse()?))
    }

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

    /// Convert to a domain `ContextRecord`.
    fn into_context_record(self) -> ContextRecord {
        ContextRecord {
            id: self.id,
            record_type: ContextRecordType::from(&self.record_type),
            brief: self.brief,
            report_link: self.report_link,
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
            // For agent prompts: use plain format with full body
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
            return write!(f, "- {}", self.text);
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
///
/// Format:
/// ```text
/// - YYYY-MM-DD HH:MM:SS <sub>+HHMM</sub> pipeline:run_id:**stage** ...
///     - [ ] record 1 <sub>ctx_rec_1</sub>
///     - ✅ record 2 <sub>ctx_rec_2</sub>
/// ```
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

impl FromStr for MdStage {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let mut lines = s.lines();
        let first = lines.next().ok_or_else(|| anyhow::anyhow!("Empty stage"))?;
        let title: MdStageTitle = first.parse()?;
        let mut records = Vec::new();

        for line in lines {
            if line.trim().is_empty() {
                continue;
            }

            let trimmed = line.trim();

            if let Some(record) = MdRecord::try_parse(trimmed)? {
                records.push(record);
            }
        }
        Ok(MdStage {
            title,
            records,
            for_prompt: false,
        })
    }
}

impl serde::Serialize for MdStage {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for MdStage {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
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

    /// Convert to a domain `StageContext`.
    fn into_stage_context(self) -> StageContext {
        StageContext {
            info: self.title.into(),
            records: self
                .records
                .into_iter()
                .map(|r| r.into_context_record())
                .collect(),
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────────
// Markdown context document
// ────────────────────────────────────────────────────────────────────────────────

/// An entry in the context document.
#[derive(Debug, Clone)]
enum MdEntry {
    Stage(MdStage),
    CompactComment(MdCompactComment),
}

/// The complete context document in markdown format.
///
/// `Serialize`/`Deserialize` convert to/from the full markdown text.
#[derive(Debug, Clone)]
struct MdContext {
    entries: Vec<MdEntry>,
    for_prompt: bool,
}

impl fmt::Display for MdContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Add <!-- stage --> markers before stage entries when compact comments are present
        // so parsers can distinguish stage lines from compact comment lines.
        // In prompt mode, these markers are omitted to reduce noise.
        let has_compact = !self.for_prompt
            && self
                .entries
                .iter()
                .any(|e| matches!(e, MdEntry::CompactComment(_)));

        for entry in &self.entries {
            match entry {
                MdEntry::Stage(stage) => {
                    if has_compact {
                        writeln!(f, "<!-- stage -->")?;
                    }
                    // Stage display already ends with \n via writeln!
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

impl FromStr for MdContext {
    type Err = anyhow::Error;

    fn from_str(text: &str) -> Result<Self> {
        let mut entries: Vec<MdEntry> = Vec::new();
        let mut current_stage: Option<MdStage> = None;
        let mut after_stage_marker = false;

        for line in text.lines() {
            let trimmed = line.trim();

            if trimmed.is_empty() {
                continue;
            }

            // Track <!-- stage --> markers (inserted before stage titles in user-display mode)
            if trimmed == "<!-- stage -->" {
                after_stage_marker = true;
                continue;
            }

            // Try parsing as record (add to current stage)
            if let Some(record) = MdRecord::try_parse(trimmed)? {
                after_stage_marker = false;
                let stage = current_stage.as_mut().ok_or_else(|| {
                    anyhow::anyhow!("Context record found before any stage header: {}", trimmed)
                })?;
                stage.records.push(record);
                continue;
            }

            // Parse as stage title — try first to avoid flushing current_stage on failure
            if trimmed.starts_with("- ") {
                let was_after_marker = after_stage_marker;
                after_stage_marker = false;
                if was_after_marker {
                    // Preceded by <!-- stage -->: must parse as a valid stage title
                    let title = trimmed
                        .parse::<MdStageTitle>()
                        .map_err(|e| anyhow::anyhow!("Malformed stage title after <!-- stage --> marker: {e}"))?;
                    if let Some(stage) = current_stage.take() {
                        entries.push(MdEntry::Stage(stage));
                    }
                    current_stage = Some(MdStage {
                        title,
                        records: Vec::new(),
                        for_prompt: false,
                    });
                } else if let Ok(title) = trimmed.parse::<MdStageTitle>() {
                    // Valid stage title without marker: flush previous stage
                    if let Some(stage) = current_stage.take() {
                        entries.push(MdEntry::Stage(stage));
                    }
                    current_stage = Some(MdStage {
                        title,
                        records: Vec::new(),
                        for_prompt: false,
                    });
                }
                // else: compact comment line or unknown `- ` line — skip silently
                continue;
            }

            after_stage_marker = false;
            bail!("Unrecognized line in context: {}", trimmed);
        }

        // Flush remaining stage
        if let Some(stage) = current_stage {
            entries.push(MdEntry::Stage(stage));
        }

        Ok(MdContext {
            entries,
            for_prompt: false,
        })
    }
}

impl serde::Serialize for MdContext {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for MdContext {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
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
            for_prompt,
        }
    }

    /// Convert to domain `TaskContext` (comments are discarded).
    fn into_task_context(self) -> TaskContext {
        let stages = self
            .entries
            .into_iter()
            .filter_map(|e| match e {
                MdEntry::Stage(s) => Some(s.into_stage_context()),
                MdEntry::CompactComment(_) => None,
            })
            .collect();
        TaskContext { stages }
    }
}

// ────────────────────────────────────────────────────────────────────────────────
// Public API
// ────────────────────────────────────────────────────────────────────────────────

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
    let md = MdContext::from_task_context(ctx, comments, for_prompt, report_url);
    md.to_string()
}

/// Parse markdown-formatted context back into a `TaskContext`.
///
/// Blockquote lines (user comments) are parsed but discarded during conversion.
/// Returns `Err` on any parse failure.
pub fn parse_context(text: &str) -> Result<TaskContext> {
    let md: MdContext = text.parse()?;
    Ok(md.into_task_context())
}

// ────────────────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{Model, Pipeline, Stage, StageInfo};

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
                        run_id: 1,
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
                        run_id: 1,
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

        assert!(output.contains("default:main:1:**planning** `claude` `claude-opus-4.6`"));
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

        // Stage header should only contain the stage name
        assert!(output.contains("- planning\n"));
        assert!(output.contains("- working\n"));
        // Metadata (tool, model, timestamp, prompt link) should not appear
        assert!(!output.contains("`claude`"));
        assert!(!output.contains("`claude-opus-4.6`"));
        assert!(!output.contains("](prompts/plan.md)"));
        // Interactive records should use plain [ctx_rec_N] format
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
        // Non-interactive records must NOT have ctx_rec IDs
        assert!(
            !output.contains("[ctx_rec_5]"),
            "comment without link should suppress ID in prompt mode"
        );
        assert!(
            !output.contains("[ctx_rec_6]"),
            "question without link should suppress ID in prompt mode"
        );
        // Non-interactive record text should still appear
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
        assert_eq!(s0.info.run_id, 1);
        assert_eq!(s0.info.stage, Stage::new("planning"));
        assert_eq!(s0.info.timestamp, utc("2024-01-01T00:00:00Z"));
        assert!(s0.info.prompt_link.as_deref() == Some("prompts/plan.md"));
        assert_eq!(s0.records.len(), 3);

        // Output reorders first non-checkbox to the first slot.
        assert_eq!(s0.records[0].id, 3);
        assert_eq!(s0.records[0].record_type, ContextRecordType::Success);
        assert_eq!(s0.records[0].brief, "Plan completed");
        assert_eq!(
            s0.records[0].report_link.as_deref(),
            Some("reports/plan_success.md")
        );

        assert_eq!(s0.records[1].id, 1);
        assert_eq!(
            s0.records[1].record_type,
            ContextRecordType::Checkbox(false)
        );
        assert_eq!(s0.records[1].brief, "Define API schema");

        assert_eq!(s0.records[2].id, 2);
        assert_eq!(s0.records[2].record_type, ContextRecordType::Checkbox(true));
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

            let mut expected_ids: Vec<u64> = orig_stage.records.iter().map(|r| r.id).collect();
            if let Some(pos) = orig_stage
                .records
                .iter()
                .position(|r| !matches!(r.record_type, ContextRecordType::Checkbox(_)))
                && pos != 0
            {
                let id = expected_ids.remove(pos);
                expected_ids.insert(0, id);
            }

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

        // for_prompt output is not meant to be parsed back — verify the rendered format instead
        // Stage name only (no metadata)
        assert!(serialized.contains("- planning\n"));
        // prompt_link not present
        assert!(!serialized.contains("prompts/plan.md"));
    }

    #[test]
    fn for_prompt_also_omits_output_link() {
        let mut ctx = sample_context();
        ctx.stages[0].info.output_link = Some("outputs/plan_output.md".to_string());
        let serialized = serialize_context(&ctx, &[], true, None);

        // for_prompt output is not meant to be parsed back — verify the rendered format instead
        // Neither prompt nor output links should appear
        assert!(!serialized.contains("prompts/plan.md"));
        assert!(!serialized.contains("outputs/plan_output.md"));
        // Stage name only
        assert!(serialized.contains("- planning\n"));
    }

    #[test]
    fn output_link_url_mapped_via_report_url() {
        let ctx = TaskContext {
            stages: vec![StageContext {
                info: StageInfo {
                    instance: "default".to_string(),
                    pipeline: Pipeline::from("main"),
                    run_id: 1,
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
    fn parse_error_on_record_before_stage() {
        let text = "  - [ ] orphan item <sub>ctx_rec_1</sub>\n";
        let result = parse_context(text);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("before any stage header")
        );
    }

    #[test]
    fn parse_error_on_missing_id() {
        let text = "\
- default:main:1:**working** `2024-01-01 00:00:00 +0000`
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
                    instance: "default".to_string(),
                    pipeline: Pipeline::from("main"),
                    run_id: 1,
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

        // Stage should come before comment (by timestamp)
        let stage_pos = output.find("default:main:1:**working**").unwrap();
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
                    run_id: 1,
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

        // The URL should appear exactly once, not doubled
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

    // -- Serde roundtrip tests for wrapper types --

    #[test]
    fn md_record_display_roundtrip() {
        let record = MdRecord {
            record_type: MdRecordType::Success,
            brief: "All tests passed".to_string(),
            id: 42,
            report_link: Some("reports/test.md".to_string()),
            for_prompt: false,
        };
        let s = record.to_string();
        let parsed: MdRecord = s.parse().unwrap();
        assert_eq!(parsed.id, 42);
        assert_eq!(parsed.record_type, MdRecordType::Success);
        assert_eq!(parsed.brief, "All tests passed");
        assert_eq!(parsed.report_link.as_deref(), Some("reports/test.md"));
    }

    #[test]
    fn md_record_no_link_roundtrip() {
        let record = MdRecord {
            record_type: MdRecordType::CheckboxUnchecked,
            brief: "Todo item".to_string(),
            id: 1,
            report_link: None,
            for_prompt: false,
        };
        let s = record.to_string();
        let parsed: MdRecord = s.parse().unwrap();
        assert_eq!(parsed.id, 1);
        assert_eq!(parsed.record_type, MdRecordType::CheckboxUnchecked);
        assert_eq!(parsed.brief, "Todo item");
        assert!(parsed.report_link.is_none());
    }

    #[test]
    fn md_stage_display_roundtrip() {
        let stage = MdStage {
            title: MdStageTitle {
                instance: "default".to_string(),
                timestamp: utc("2024-01-01T00:00:00Z"),
                pipeline: Pipeline::from("main"),
                run_id: 1,
                stage: Stage::new("working"),
                tool: None,
                model: None,
                prompt_link: None,
                output_link: None,
            },
            records: vec![
                MdRecord {
                    record_type: MdRecordType::CheckboxUnchecked,
                    brief: "Item 1".to_string(),
                    id: 1,
                    report_link: None,
                    for_prompt: false,
                },
                MdRecord {
                    record_type: MdRecordType::Success,
                    brief: "Done".to_string(),
                    id: 2,
                    report_link: Some("r.md".to_string()),
                    for_prompt: false,
                },
            ],
            for_prompt: false,
        };
        let s = stage.to_string();
        let parsed: MdStage = s.parse().unwrap();
        assert_eq!(parsed.title, stage.title);
        assert_eq!(parsed.records.len(), 2);
        // Output reorders first non-checkbox record to the front
        assert_eq!(parsed.records[0].id, 2);
        assert_eq!(parsed.records[1].id, 1);
    }

    #[test]
    fn md_context_display_roundtrip() {
        let ctx = sample_context();
        let md = MdContext::from_task_context(&ctx, &[], false, None);
        let s = md.to_string();
        let parsed: MdContext = s.parse().unwrap();
        let result = parsed.into_task_context();
        assert_eq!(result.stages.len(), ctx.stages.len());
        for (orig, parsed) in ctx.stages.iter().zip(result.stages.iter()) {
            assert_eq!(parsed.info.pipeline, orig.info.pipeline);
            assert_eq!(parsed.info.run_id, orig.info.run_id);
            assert_eq!(parsed.info.stage, orig.info.stage);
            assert_eq!(parsed.records.len(), orig.records.len());
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
        // 80 chars of 'a' followed by '...'
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
        assert!(output.lines().count() == 1); // all on one line
    }

    #[test]
    fn compact_comment_prefixes_user() {
        let ctx = TaskContext::default();
        let comments = vec![make_comment("hello world", "2024-01-01T00:00:00Z", None)];
        let output = serialize_context(&ctx, &comments, false, None);
        assert!(output.contains("- user:**unknown** hello world `2024-01-01 00:00:00 +0000`"));
    }

    #[test]
    fn compact_comment_no_extra_cr_after_comment() {
        let ctx = TaskContext::default();
        let comments = vec![make_comment("hello world", "2024-01-01T00:00:00Z", None)];
        let output = serialize_context(&ctx, &comments, false, None);
        assert_eq!(output.lines().count(), 1);
    }

    #[test]
    fn stage_marker_added_before_stages_when_compact_comments_present() {
        let ctx = sample_context();
        let comments = vec![make_comment("a comment", "2024-01-01T00:30:00Z", None)];
        let output = serialize_context(&ctx, &comments, false, None);
        assert!(output.contains("<!-- stage -->"));
    }

    #[test]
    fn stage_marker_not_added_without_comments() {
        let ctx = sample_context();
        let output = serialize_context(&ctx, &[], false, None);
        assert!(!output.contains("<!-- stage -->"));
    }

    #[test]
    fn stage_marker_not_added_in_prompt_mode() {
        let ctx = sample_context();
        let comments = vec![make_comment("a comment", "2024-01-01T00:30:00Z", None)];
        // Even with compact comments present, prompt mode should NOT emit stage markers
        let output = serialize_context(&ctx, &comments, true, None);
        assert!(!output.contains("<!-- stage -->"));
    }

    #[test]
    fn compact_comment_roundtrip_preserves_context() {
        // When compact comments are present, the TaskContext should still be parsed correctly
        let ctx = sample_context();
        let comments = vec![make_comment(
            "comment before stage 2",
            "2024-01-01T00:30:00Z",
            None,
        )];
        let serialized = serialize_context(&ctx, &comments, false, None);
        // Parse back — compact comments should be skipped, stages preserved
        let parsed = parse_context(&serialized).unwrap();
        assert_eq!(parsed.stages.len(), ctx.stages.len());
        assert_eq!(parsed.stages[0].info.stage, ctx.stages[0].info.stage);
    }

    #[test]
    fn for_prompt_true_uses_compact_comment_format() {
        let ctx = TaskContext::default();
        let comments = vec![make_comment("a user comment", "2024-01-01T00:00:00Z", None)];
        let output = serialize_context(&ctx, &comments, true, None);
        // Prompt mode renders comments without timestamp or link
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
        // Should use plain [ctx_rec_N] format
        assert_eq!(rendered, "- ✅ Plan completed [ctx_rec_7]");
        // Must NOT contain <sub> or URL
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
        // Should render as simple list item with user prefix
        assert_eq!(rendered, "- user alice: please proceed");
        // Must NOT contain timestamp or URL
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
                run_id: 1,
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
        // Stage header should only show the stage name
        assert!(rendered.starts_with("- planning\n"));
        // Must NOT contain metadata
        assert!(!rendered.contains("`claude`"));
        assert!(!rendered.contains("claude-opus-4.6"));
        assert!(!rendered.contains("2024-01-01"));
        assert!(!rendered.contains("prompts/plan.md"));
        // Non-interactive record (Success without report_link) must NOT show ctx_rec ID
        assert!(
            !rendered.contains("ctx_rec_1"),
            "non-interactive success record should suppress ID in prompt mode"
        );
        // Record text should still appear
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
                        run_id: 1,
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
                // Empty stage (no records) — should be filtered in for_prompt mode
                StageContext {
                    info: StageInfo {
                        instance: "default".to_string(),
                        pipeline: Pipeline::from("main"),
                        run_id: 1,
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
                        run_id: 1,
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

        // for_prompt=true: empty stages should be filtered out
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

        // for_prompt=false: ALL stages should appear, including empty ones
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
        // Build a realistic context with multiple stages, one empty, and interleaved comments
        let ctx = TaskContext {
            stages: vec![
                StageContext {
                    info: StageInfo {
                        instance: "skynet".to_string(),
                        pipeline: Pipeline::from("main"),
                        run_id: 1,
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
                // Empty stage — should be filtered out in for_prompt mode
                StageContext {
                    info: StageInfo {
                        instance: "skynet".to_string(),
                        pipeline: Pipeline::from("main"),
                        run_id: 1,
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
                        run_id: 1,
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

        // Comments interleaved between stages
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

        // 1. No <!-- stage --> markers anywhere
        assert!(
            !output.contains("<!-- stage -->"),
            "prompt output must not contain stage markers"
        );

        // 2. Stage headers are just "- {stage_name}" (no metadata)
        assert!(
            output.contains("- planning\n"),
            "planning stage should appear as plain name"
        );
        assert!(
            output.contains("- reviewing\n"),
            "reviewing stage should appear as plain name"
        );

        // 3. Empty "working" stage is filtered out
        assert!(
            !output.contains("working"),
            "empty working stage should be filtered out"
        );

        // 4. No stage metadata leaks (no tool, model, timestamps, prompt/output links)
        assert!(!output.contains("`claude`"), "no tool metadata");
        assert!(!output.contains("claude-opus-4.6"), "no model metadata");
        assert!(
            !output.contains("2024-06-01 10:00:00"),
            "no timestamps in stage headers"
        );
        assert!(!output.contains("prompts/"), "no prompt links");
        assert!(!output.contains("outputs/"), "no output links");

        // 5. Interactive records use plain [ctx_rec_N] (no <sub>, no URLs)
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

        // 6. Comments are plain "- user {name}: {body}" (no timestamp, no URL, no bold)
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

        // 7. Records are properly indented under stages
        assert!(
            output.contains("  - 💬 Plan ready for review [ctx_rec_1]"),
            "non-checkbox record indented with 2 spaces"
        );
        assert!(
            output.contains("    - [x] Define API schema [ctx_rec_2]"),
            "checkbox record indented with 4 spaces"
        );

        // 8. Verify correct ordering (planning, comment, comment, reviewing)
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

        // for_prompt=true: full multi-line body should be preserved
        let prompt_output = serialize_context(&ctx, &comments, true, None);
        assert!(
            prompt_output.contains("proceed with plan"),
            "first line should appear in prompt mode"
        );
        assert!(
            prompt_output.contains("also fix the bug"),
            "second line should appear in prompt mode"
        );
        assert!(
            prompt_output.contains("and update docs"),
            "third line should appear in prompt mode"
        );
        assert!(
            prompt_output
                .starts_with("- user alice: proceed with plan\nalso fix the bug\nand update docs"),
            "full multi-line body should be preserved verbatim"
        );

        // for_prompt=false: lines should be joined with spaces in non-prompt mode
        let normal_output = serialize_context(&ctx, &comments, false, None);
        assert!(
            normal_output.contains("proceed with plan"),
            "first line should appear in normal mode"
        );
        assert!(
            normal_output.contains("also fix the bug"),
            "second line should appear in normal mode (joined with space)"
        );
        assert!(
            normal_output.contains("and update docs"),
            "third line should appear in normal mode (joined with space)"
        );
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
        assert!(
            !rendered.contains("ctx_rec_"),
            "non-interactive success should suppress ID"
        );
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
        assert!(
            !rendered.contains("ctx_rec_"),
            "non-interactive failure should suppress ID"
        );
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
        assert!(
            !rendered.contains("ctx_rec_"),
            "non-interactive comment should suppress ID"
        );
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
        assert!(
            !rendered.contains("ctx_rec_"),
            "non-interactive question should suppress ID"
        );
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
        assert!(
            !rendered.contains("reports/review.md"),
            "report URL should not leak"
        );
        assert!(!rendered.contains("<sub>"), "no <sub> tags in prompt mode");
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
        assert!(
            !rendered.contains("reports/build.md"),
            "report URL should not leak"
        );
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
        assert!(
            !rendered.contains("reports/plan.md"),
            "report URL should not leak"
        );
    }

    #[test]
    fn md_record_normal_mode_always_shows_id() {
        // Verify that normal mode (for_prompt=false) still shows IDs for all record types
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

    // -- End-to-end test with mixed interactive and non-interactive records in prompt mode --

    #[test]
    fn for_prompt_mixed_interactive_and_non_interactive_records() {
        let ctx = TaskContext {
            stages: vec![StageContext {
                info: StageInfo {
                    instance: "default".to_string(),
                    pipeline: Pipeline::from("main"),
                    run_id: 1,
                    stage: Stage::new("working"),
                    tool: Some("claude".to_string()),
                    model: Some("claude-opus-4.6".parse().unwrap()),
                    prompt_link: None,
                    output_link: None,
                    timestamp: utc("2024-01-01T00:00:00Z"),
                },
                records: vec![
                    // Interactive: checkbox unchecked
                    ContextRecord {
                        id: 1,
                        record_type: ContextRecordType::Checkbox(false),
                        brief: "Implement feature".to_string(),
                        report_link: None,
                    },
                    // Interactive: checkbox checked
                    ContextRecord {
                        id: 2,
                        record_type: ContextRecordType::Checkbox(true),
                        brief: "Write tests".to_string(),
                        report_link: None,
                    },
                    // Interactive: success with report_link
                    ContextRecord {
                        id: 3,
                        record_type: ContextRecordType::Success,
                        brief: "All tests passed".to_string(),
                        report_link: Some("reports/tests.md".to_string()),
                    },
                    // Non-interactive: success without report_link
                    ContextRecord {
                        id: 4,
                        record_type: ContextRecordType::Success,
                        brief: "Lint clean".to_string(),
                        report_link: None,
                    },
                    // Non-interactive: failure without report_link
                    ContextRecord {
                        id: 5,
                        record_type: ContextRecordType::Failure,
                        brief: "Flaky test".to_string(),
                        report_link: None,
                    },
                    // Interactive: failure with report_link
                    ContextRecord {
                        id: 6,
                        record_type: ContextRecordType::Failure,
                        brief: "Build error".to_string(),
                        report_link: Some("reports/build.md".to_string()),
                    },
                    // Non-interactive: comment without report_link
                    ContextRecord {
                        id: 7,
                        record_type: ContextRecordType::Comment,
                        brief: "Retrying now".to_string(),
                        report_link: None,
                    },
                    // Interactive: comment with report_link
                    ContextRecord {
                        id: 8,
                        record_type: ContextRecordType::Comment,
                        brief: "Plan ready".to_string(),
                        report_link: Some("reports/plan.md".to_string()),
                    },
                    // Non-interactive: question without report_link
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

        // Interactive records MUST show [ctx_rec_N]
        assert!(
            output.contains("[ctx_rec_1]"),
            "unchecked checkbox should show ID"
        );
        assert!(
            output.contains("[ctx_rec_2]"),
            "checked checkbox should show ID"
        );
        assert!(
            output.contains("[ctx_rec_3]"),
            "success with link should show ID"
        );
        assert!(
            output.contains("[ctx_rec_6]"),
            "failure with link should show ID"
        );
        assert!(
            output.contains("[ctx_rec_8]"),
            "comment with link should show ID"
        );

        // Non-interactive records MUST NOT show [ctx_rec_N]
        assert!(
            !output.contains("[ctx_rec_4]"),
            "success without link should suppress ID"
        );
        assert!(
            !output.contains("[ctx_rec_5]"),
            "failure without link should suppress ID"
        );
        assert!(
            !output.contains("[ctx_rec_7]"),
            "comment without link should suppress ID"
        );
        assert!(
            !output.contains("[ctx_rec_9]"),
            "question without link should suppress ID"
        );

        // All record briefs should still appear regardless of interactivity
        assert!(
            output.contains("Implement feature"),
            "checkbox brief should appear"
        );
        assert!(
            output.contains("Write tests"),
            "checkbox brief should appear"
        );
        assert!(
            output.contains("All tests passed"),
            "success brief should appear"
        );
        assert!(
            output.contains("Lint clean"),
            "non-interactive success brief should appear"
        );
        assert!(
            output.contains("Flaky test"),
            "non-interactive failure brief should appear"
        );
        assert!(
            output.contains("Build error"),
            "failure with link brief should appear"
        );
        assert!(
            output.contains("Retrying now"),
            "non-interactive comment brief should appear"
        );
        assert!(
            output.contains("Plan ready"),
            "comment with link brief should appear"
        );
        assert!(
            output.contains("Should we refactor?"),
            "non-interactive question brief should appear"
        );

        // No <sub> tags or report URLs in prompt mode
        assert!(!output.contains("<sub>"), "no <sub> tags in prompt output");
        assert!(
            !output.contains("reports/"),
            "no report URLs in prompt output"
        );

        // Verify normal mode still shows ALL IDs for comparison
        let normal_output = serialize_context(&ctx, &[], false, None);
        for id in 1..=9 {
            assert!(
                normal_output.contains(&format!("ctx_rec_{}", id)),
                "normal mode should show ctx_rec_{id}"
            );
        }
    }

    #[test]
    fn parse_errors_on_malformed_stage_after_marker() {
        // A valid stage followed by <!-- stage --> marker and a malformed stage title
        // (model token with a space) should produce an error, not silently skip.
        let text = "\
- default:main:1:**working** `claude` `claude-opus-4.6` `2024-01-01 00:00:00 +0000`
<!-- stage -->
- default:main:2:**working** `claude` `bad model` `2024-06-15 10:30:00 +0300`
";
        let result = parse_context(text);
        assert!(result.is_err(), "expected error for malformed stage title after <!-- stage --> marker");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Malformed stage title after <!-- stage --> marker"),
            "error should mention the marker context, got: {err_msg}"
        );
    }
}
