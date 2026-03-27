//! Context serialization/deserialization module.
//!
//! Provides a two-stage conversion between domain types and markdown format:
//! 1. Domain types (`TaskContext`, `Comment`) ↔ Markdown representation types (`MdContext`)
//! 2. Markdown representation types ↔ Markdown string (via `serde::Serialize`/`Deserialize`)
//!
//! The markdown representation types are local to this module and not exported.

mod stage_title;

use std::fmt;
use std::str::FromStr;

use anyhow::{bail, Context as _, Result};

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
    parent_record_id: Option<u64>,
}

impl fmt::Display for MdRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.record_type.prefix(), self.brief)?;
        let id_tag = format!("ctx_rec_{}", self.id);
        if let Some(url) = &self.report_link {
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
            parent_record_id: None,
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
            parent_record_id: r.parent_record_id,
        }
    }

    /// Convert to a domain `ContextRecord`.
    fn into_context_record(self) -> ContextRecord {
        ContextRecord {
            id: self.id,
            record_type: ContextRecordType::from(&self.record_type),
            brief: self.brief,
            report_link: self.report_link,
            parent_record_id: self.parent_record_id,
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────────
// Markdown user comment
// ────────────────────────────────────────────────────────────────────────────────

/// A user comment as it appears in markdown blockquote format.
///
/// Format:
/// ```text
/// > **[YYYY-MM-DD HH:MM:SS <sub>+HHMM</sub>]** first line
/// > continued line
/// ```
#[derive(Debug, Clone)]
struct MdUserComment {
    timestamp: chrono::DateTime<chrono::FixedOffset>,
    text: String,
}

impl fmt::Display for MdUserComment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "> **[{} <sub>{}</sub>]** ",
            self.timestamp.format("%Y-%m-%d %H:%M:%S"),
            self.timestamp.format("%z"),
        )?;
        for (i, line) in self.text.lines().enumerate() {
            if i > 0 {
                write!(f, "\n> ")?;
            }
            write!(f, "{}", line)?;
        }
        Ok(())
    }
}

impl FromStr for MdUserComment {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let mut lines = s.lines();
        let first = lines
            .next()
            .ok_or_else(|| anyhow::anyhow!("Empty comment"))?;
        let first = first.trim_start_matches('>').trim();

        // Parse: **[YYYY-MM-DD HH:MM:SS <sub>+HHMM</sub>]** text
        let inner = first
            .strip_prefix("**[")
            .ok_or_else(|| anyhow::anyhow!("Invalid comment header: {}", first))?;
        let bracket_end = inner
            .find("]**")
            .ok_or_else(|| anyhow::anyhow!("Missing ]** in comment header"))?;
        let ts_part = &inner[..bracket_end];
        let text_start = inner[bracket_end + 3..].trim();

        // Parse timestamp: "YYYY-MM-DD HH:MM:SS <sub>+HHMM</sub>"
        let sub_start = ts_part
            .find("<sub>")
            .ok_or_else(|| anyhow::anyhow!("Missing <sub> in comment timestamp"))?;
        let sub_end = ts_part
            .find("</sub>")
            .ok_or_else(|| anyhow::anyhow!("Missing </sub> in comment timestamp"))?;
        let datetime_part = ts_part[..sub_start].trim();
        let tz = &ts_part[sub_start + 5..sub_end];
        let full = format!("{} {}", datetime_part, tz);
        let timestamp = chrono::DateTime::parse_from_str(&full, "%Y-%m-%d %H:%M:%S %z")
            .with_context(|| format!("Invalid comment timestamp: {}", ts_part))?;

        // Collect text from all lines
        let mut text = text_start.to_string();
        for line in lines {
            let content = line.trim_start_matches('>');
            let content = if content.starts_with(' ') {
                &content[1..]
            } else {
                content
            };
            text.push('\n');
            text.push_str(content);
        }

        Ok(MdUserComment { timestamp, text })
    }
}

impl serde::Serialize for MdUserComment {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for MdUserComment {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl From<&Comment> for MdUserComment {
    fn from(c: &Comment) -> Self {
        MdUserComment {
            timestamp: c.timestamp,
            text: c.text.clone(),
        }
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
///   - [ ] record 1 <sub>ctx_rec_1</sub>
///   - ✅ record 2 <sub>ctx_rec_2</sub>
/// ```
#[derive(Debug, Clone)]
struct MdStage {
    title: MdStageTitle,
    records: Vec<MdRecord>,
}

impl fmt::Display for MdStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "- {}", self.title)?;

        // Group records by parent_record_id for hierarchical display
        for record in &self.records {
            if record.parent_record_id.is_none() {
                // Top-level record
                writeln!(f, "  {}", record)?;

                // Display all child records (those with this record's id as parent)
                for child in &self.records {
                    if child.parent_record_id == Some(record.id) {
                        writeln!(f, "    {}", child)?;
                    }
                }
            }
        }
        Ok(())
    }
}

impl FromStr for MdStage {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let mut lines = s.lines();
        let first = lines
            .next()
            .ok_or_else(|| anyhow::anyhow!("Empty stage"))?;
        let title: MdStageTitle = first.parse()?;
        let mut records = Vec::new();
        let mut last_top_level_id: Option<u64> = None;

        for line in lines {
            if line.trim().is_empty() {
                continue;
            }

            // Count leading spaces to determine indentation level
            let leading_spaces = line.len() - line.trim_start().len();
            let trimmed = line.trim();

            if let Some(record) = MdRecord::try_parse(trimmed)? {
                let mut record = record;
                // If indented by 4 spaces (child level), set parent to last top-level record
                if leading_spaces >= 4 && last_top_level_id.is_some() {
                    record.parent_record_id = last_top_level_id;
                } else {
                    // Top-level record (less indentation)
                    last_top_level_id = Some(record.id);
                }
                records.push(record);
            }
        }
        Ok(MdStage { title, records })
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

        // Transform prompt link URL if needed
        if let Some(link) = &mut title.prompt_link {
            if !link.starts_with("http://") && !link.starts_with("https://") {
                if let Some(f) = report_url {
                    *link = f(link);
                }
            }
        }

        // Omit prompt link for agent prompts
        if for_prompt {
            title.prompt_link = None;
        }

        let records = stage
            .records
            .iter()
            .map(|r| MdRecord::from_context_record(r, report_url))
            .collect();

        MdStage { title, records }
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
    Comment(MdUserComment),
}

/// The complete context document in markdown format.
///
/// `Serialize`/`Deserialize` convert to/from the full markdown text.
#[derive(Debug, Clone)]
struct MdContext {
    entries: Vec<MdEntry>,
}

impl fmt::Display for MdContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for entry in &self.entries {
            match entry {
                MdEntry::Stage(stage) => {
                    // Stage display already ends with \n via writeln!
                    write!(f, "{}", stage)?;
                }
                MdEntry::Comment(comment) => {
                    write!(f, "{}\n\n", comment)?;
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
        let mut comment_lines: Vec<&str> = Vec::new();

        for line in text.lines() {
            let trimmed = line.trim();

            if trimmed.is_empty() {
                // Flush accumulated comment lines (lenient: skip on parse failure)
                if !comment_lines.is_empty() {
                    let joined = comment_lines.join("\n");
                    if let Ok(comment) = joined.parse::<MdUserComment>() {
                        entries.push(MdEntry::Comment(comment));
                    }
                    comment_lines.clear();
                }
                continue;
            }

            // Accumulate blockquote lines
            if trimmed.starts_with('>') {
                comment_lines.push(trimmed);
                continue;
            }

            // Flush pending comment lines
            if !comment_lines.is_empty() {
                let joined = comment_lines.join("\n");
                if let Ok(comment) = joined.parse::<MdUserComment>() {
                    entries.push(MdEntry::Comment(comment));
                }
                comment_lines.clear();
            }

            // Try parsing as record (add to current stage)
            if let Some(record) = MdRecord::try_parse(trimmed)? {
                let stage = current_stage.as_mut().ok_or_else(|| {
                    anyhow::anyhow!("Context record found before any stage header: {}", trimmed)
                })?;
                stage.records.push(record);
                continue;
            }

            // Parse as stage title
            if trimmed.starts_with("- ") {
                // Flush previous stage
                if let Some(stage) = current_stage.take() {
                    entries.push(MdEntry::Stage(stage));
                }
                let title: MdStageTitle = trimmed
                    .parse()
                    .with_context(|| format!("Failed to parse stage title: {}", trimmed))?;
                current_stage = Some(MdStage {
                    title,
                    records: Vec::new(),
                });
                continue;
            }

            bail!("Unrecognized line in context: {}", trimmed);
        }

        // Flush remaining
        if !comment_lines.is_empty() {
            let joined = comment_lines.join("\n");
            if let Ok(comment) = joined.parse::<MdUserComment>() {
                entries.push(MdEntry::Comment(comment));
            }
        }
        if let Some(stage) = current_stage {
            entries.push(MdEntry::Stage(stage));
        }

        Ok(MdContext { entries })
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
        let mut events: Vec<(chrono::DateTime<chrono::FixedOffset>, MdEntry)> = Vec::new();

        for stage in &ctx.stages {
            events.push((
                stage.info.timestamp,
                MdEntry::Stage(MdStage::from_stage_context(stage, for_prompt, report_url)),
            ));
        }

        for comment in comments {
            events.push((
                comment.timestamp,
                MdEntry::Comment(MdUserComment::from(comment)),
            ));
        }

        // Sort by timestamp (stable sort preserves insertion order for equal timestamps)
        events.sort_by(|a, b| a.0.cmp(&b.0));

        MdContext {
            entries: events.into_iter().map(|(_, e)| e).collect(),
        }
    }

    /// Convert to domain `TaskContext` (comments are discarded).
    fn into_task_context(self) -> TaskContext {
        let stages = self
            .entries
            .into_iter()
            .filter_map(|e| match e {
                MdEntry::Stage(s) => Some(s.into_stage_context()),
                MdEntry::Comment(_) => None,
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
                        pipeline: Pipeline::from("main"),
                        run_id: 1,
                        stage: Stage::new("planning"),
                        tool: Some(crate::task::Tool::Claude),
                        model: Some(Model::ClaudeOpus4_6),
                        prompt_link: Some("prompts/plan.md".to_string()),
                        timestamp: utc("2024-01-01T00:00:00Z"),
                    },
                    records: vec![
                        ContextRecord {
                            id: 1,
                            record_type: ContextRecordType::Checkbox(false),
                            brief: "Define API schema".to_string(),
                            report_link: None,
                            parent_record_id: None,
                        },
                        ContextRecord {
                            id: 2,
                            record_type: ContextRecordType::Checkbox(true),
                            brief: "Review requirements".to_string(),
                            report_link: None,
                            parent_record_id: None,
                        },
                        ContextRecord {
                            id: 3,
                            record_type: ContextRecordType::Success,
                            brief: "Plan completed".to_string(),
                            report_link: Some("reports/plan_success.md".to_string()),
                            parent_record_id: None,
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
                        timestamp: utc("2024-01-01T01:00:00Z"),
                    },
                    records: vec![
                        ContextRecord {
                            id: 4,
                            record_type: ContextRecordType::Failure,
                            brief: "Build failed".to_string(),
                            report_link: Some("reports/build_fail.md".to_string()),
                            parent_record_id: None,
                        },
                        ContextRecord {
                            id: 5,
                            record_type: ContextRecordType::Comment,
                            brief: "Retrying with fix".to_string(),
                            report_link: None,
                            parent_record_id: None,
                        },
                        ContextRecord {
                            id: 6,
                            record_type: ContextRecordType::Question,
                            brief: "Should we use async?".to_string(),
                            report_link: None,
                            parent_record_id: None,
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

        assert!(output.contains("main:1:**planning** `claude` `claude-opus-4.6`"));
        assert!(output.contains("<sub>[2024-01-01 00:00:00 +0000](prompts/plan.md)</sub>"));
        assert!(output.contains("  - [ ] Define API schema"));
        assert!(output.contains("  - [x] Review requirements"));
        assert!(output.contains(
            "  - ✅ Plan completed <sub>[ctx_rec_3](reports/plan_success.md)</sub>"
        ));
        assert!(output
            .contains("  - ❌ Build failed <sub>[ctx_rec_4](reports/build_fail.md)</sub>"));
        assert!(output.contains("  - 💬 Retrying with fix"));
        assert!(output.contains("  - ❓ Should we use async?"));
    }

    #[test]
    fn serialize_for_prompt_omits_prompt_link() {
        let ctx = sample_context();
        let output = serialize_context(&ctx, &[], true, None);

        assert!(!output.contains("](prompts/plan.md)"));
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
        assert_eq!(
            s0.records[1].record_type,
            ContextRecordType::Checkbox(true)
        );

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
- main:1:**working** <sub>2024-01-01 00:00:00 +0000</sub>
  - [ ] Do work <sub>ctx_rec_1</sub>

> **[2024-01-01 00:00:30 <sub>+0000</sub>]** User says hello
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
- main:1:**working** <sub>2024-01-01 00:00:00 +0000</sub>
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
                    timestamp: utc("2024-01-01T00:00:00Z"),
                },
                records: vec![ContextRecord {
                    id: 1,
                    record_type: ContextRecordType::Checkbox(false),
                    brief: "Task A".to_string(),
                    report_link: None,
                    parent_record_id: None,
                }],
            }],
        };

        let comments = vec![Comment {
            timestamp: utc("2024-01-01T00:30:00Z"),
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
                    pipeline: Pipeline::from("main"),
                    run_id: 1,
                    stage: Stage::new("working"),
                    tool: None,
                    model: None,
                    prompt_link: Some(
                        "https://github.com/org/repo/blob/reports/reports/task_1/prompt.md"
                            .to_string(),
                    ),
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
                    parent_record_id: None,
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
        assert!(!output
            .contains("https://github.com/org/repo/blob/reports/reports/task_1/https://"));
        assert!(output.contains(
            "](https://github.com/org/repo/blob/reports/reports/task_1/prompt.md)"
        ));
    }

    // -- Serde roundtrip tests for wrapper types --

    #[test]
    fn md_record_display_roundtrip() {
        let record = MdRecord {
            record_type: MdRecordType::Success,
            brief: "All tests passed".to_string(),
            id: 42,
            report_link: Some("reports/test.md".to_string()),
            parent_record_id: None,
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
            parent_record_id: None,
        };
        let s = record.to_string();
        let parsed: MdRecord = s.parse().unwrap();
        assert_eq!(parsed.id, 1);
        assert_eq!(parsed.record_type, MdRecordType::CheckboxUnchecked);
        assert_eq!(parsed.brief, "Todo item");
        assert!(parsed.report_link.is_none());
    }

    #[test]
    fn md_user_comment_display_roundtrip() {
        let comment = MdUserComment {
            timestamp: utc("2024-06-15T10:30:00+03:00"),
            text: "Hello world".to_string(),
        };
        let s = comment.to_string();
        let parsed: MdUserComment = s.parse().unwrap();
        assert_eq!(parsed.timestamp, comment.timestamp);
        assert_eq!(parsed.text, "Hello world");
    }

    #[test]
    fn md_user_comment_multiline_roundtrip() {
        let comment = MdUserComment {
            timestamp: utc("2024-01-01T00:00:00Z"),
            text: "First line\nSecond line\nThird line".to_string(),
        };
        let s = comment.to_string();
        let parsed: MdUserComment = s.parse().unwrap();
        assert_eq!(parsed.timestamp, comment.timestamp);
        assert_eq!(parsed.text, "First line\nSecond line\nThird line");
    }

    #[test]
    fn md_stage_display_roundtrip() {
        let stage = MdStage {
            title: MdStageTitle {
                timestamp: utc("2024-01-01T00:00:00Z"),
                pipeline: Pipeline::from("main"),
                run_id: 1,
                stage: Stage::new("working"),
                tool: None,
                model: None,
                prompt_link: None,
            },
            records: vec![
                MdRecord {
                    record_type: MdRecordType::CheckboxUnchecked,
                    brief: "Item 1".to_string(),
                    id: 1,
                    report_link: None,
                    parent_record_id: None,
                },
                MdRecord {
                    record_type: MdRecordType::Success,
                    brief: "Done".to_string(),
                    id: 2,
                    report_link: Some("r.md".to_string()),
                    parent_record_id: None,
                },
            ],
        };
        let s = stage.to_string();
        let parsed: MdStage = s.parse().unwrap();
        assert_eq!(parsed.title, stage.title);
        assert_eq!(parsed.records.len(), 2);
        assert_eq!(parsed.records[0].id, 1);
        assert_eq!(parsed.records[1].id, 2);
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
}
