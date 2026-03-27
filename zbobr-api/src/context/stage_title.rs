//! Stage title serialization/deserialization.
//!
//! The stage title is a markdown line with the format:
//! ```text
//! pipeline:run_id:**stage** `tool` `model` <sub>[YYYY-MM-DD HH:MM:SS +HHMM](prompt_url)</sub>
//! ```
//!
//! When there is no prompt link the trailing `<sub>` contains just the timestamp:
//! ```text
//! pipeline:run_id:**stage** `tool` `model` <sub>YYYY-MM-DD HH:MM:SS +HHMM</sub>
//! ```
//!
//! This module provides [`MdStageTitle`] which can be converted to/from a string
//! via `Display`/`FromStr`, and to/from [`StageInfo`] for use in the rest of the codebase.
//!
//! `Serialize`/`Deserialize` delegate to `Display`/`FromStr` for serde compatibility.

use std::fmt;
use std::str::FromStr;

use anyhow::{Context as _, Result};

use crate::task::{Model, Pipeline, Stage, StageInfo, Tool};

/// A parsed stage title line, mapping 1:1 to the markdown format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MdStageTitle {
    pub timestamp: chrono::DateTime<chrono::FixedOffset>,
    pub pipeline: Pipeline,
    pub run_id: u64,
    pub stage: Stage,
    pub tool: Option<Tool>,
    pub model: Option<Model>,
    pub prompt_link: Option<String>,
}

impl From<&StageInfo> for MdStageTitle {
    fn from(info: &StageInfo) -> Self {
        MdStageTitle {
            timestamp: info.timestamp,
            pipeline: info.pipeline.clone(),
            run_id: info.run_id,
            stage: info.stage.clone(),
            tool: info.tool,
            model: info.model.clone(),
            prompt_link: info.prompt_link.clone(),
        }
    }
}

impl From<MdStageTitle> for StageInfo {
    fn from(t: MdStageTitle) -> Self {
        StageInfo {
            timestamp: t.timestamp,
            pipeline: t.pipeline,
            run_id: t.run_id,
            stage: t.stage,
            tool: t.tool,
            model: t.model,
            prompt_link: t.prompt_link,
        }
    }
}

// -- Display (serialization) -------------------------------------------------

/// Wrapper: `pipeline:run_id:**stage**`
struct PipelineStage<'a> {
    pipeline: &'a Pipeline,
    run_id: u64,
    stage: &'a Stage,
}

impl fmt::Display for PipelineStage<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:**{}**", self.pipeline, self.run_id, self.stage)
    }
}

/// Wrapper: `` `value` ``
struct Backtick<T: fmt::Display>(T);

impl<T: fmt::Display> fmt::Display for Backtick<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "`{}`", self.0)
    }
}

/// Format a timestamp as `YYYY-MM-DD HH:MM:SS +HHMM`.
fn format_timestamp(ts: &chrono::DateTime<chrono::FixedOffset>) -> String {
    format!("{} {}", ts.format("%Y-%m-%d %H:%M:%S"), ts.format("%z"))
}

impl fmt::Display for MdStageTitle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            PipelineStage {
                pipeline: &self.pipeline,
                run_id: self.run_id,
                stage: &self.stage,
            },
        )?;
        if let Some(tool) = &self.tool {
            write!(f, " {}", Backtick(tool))?;
        }
        if let Some(model) = &self.model {
            write!(f, " {}", Backtick(model))?;
        }
        let ts = format_timestamp(&self.timestamp);
        if let Some(link) = &self.prompt_link {
            write!(f, " <sub>[{}]({})</sub>", ts, link)?;
        } else {
            write!(f, " <sub>{}</sub>", ts)?;
        }
        Ok(())
    }
}

// -- FromStr (deserialization) ------------------------------------------------

impl FromStr for MdStageTitle {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let s = s.strip_prefix("- ").unwrap_or(s).trim();
        let mut rest = s;

        // 1. pipeline:run_id:**stage**
        let (pipeline, run_id, stage) = parse_next_pipeline_stage(&mut rest)?;

        // 2. Optional backtick tokens (tool, model)
        let mut tool = None;
        let mut model = None;

        while !rest.is_empty() {
            if let Some(value) = try_parse_next_backtick(&mut rest) {
                if tool.is_none() {
                    tool = Some(value.parse().context("Invalid tool value")?);
                } else if model.is_none() {
                    model = Some(value.parse().context("Invalid model value")?);
                }
                continue;
            }
            break;
        }

        // 3. Trailing <sub>...</sub> with timestamp and optional prompt link
        let (timestamp, prompt_link) = parse_trailing_timestamp_sub(&mut rest)?;

        Ok(MdStageTitle {
            timestamp,
            pipeline,
            run_id,
            stage,
            tool,
            model,
            prompt_link,
        })
    }
}

// -- Serde (delegates to Display/FromStr) -------------------------------------

impl serde::Serialize for MdStageTitle {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for MdStageTitle {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

// -- Parsing helpers ----------------------------------------------------------

fn parse_next_pipeline_stage(rest: &mut &str) -> Result<(Pipeline, u64, Stage)> {
    let stage_marker = ":**";
    let marker_pos = rest
        .find(stage_marker)
        .ok_or_else(|| anyhow::anyhow!("Missing ':**' stage marker"))?;
    let pipeline_and_run = &rest[..marker_pos];
    let after_marker = &rest[marker_pos + stage_marker.len()..];

    let sep_pos = pipeline_and_run
        .rfind(':')
        .ok_or_else(|| anyhow::anyhow!("Missing ':' between pipeline and run_id"))?;
    let pipeline_str = pipeline_and_run[..sep_pos].trim();
    let run_id: u64 = pipeline_and_run[sep_pos + 1..]
        .trim()
        .parse()
        .context("Invalid run_id")?;

    let stage_end = after_marker
        .find("**")
        .ok_or_else(|| anyhow::anyhow!("Missing closing '**' for stage"))?;
    let stage_str = after_marker[..stage_end].trim();
    *rest = after_marker[stage_end + 2..].trim();

    Ok((pipeline_str.parse().unwrap(), run_id, Stage::new(stage_str)))
}

/// Try to parse a backtick-wrapped value. Returns the inner string if found.
fn try_parse_next_backtick<'a>(rest: &mut &'a str) -> Option<&'a str> {
    let after_tick = rest.strip_prefix('`')?;
    let tick_end = after_tick.find('`')?;
    let value = &after_tick[..tick_end];
    *rest = after_tick[tick_end + 1..].trim();
    Some(value)
}

/// Try to parse a `<sub>...</sub>` element. Returns the inner content if found.
fn try_parse_next_sub(rest: &mut &str) -> Option<String> {
    let after_open = rest.strip_prefix("<sub>")?;
    let close_pos = after_open.find("</sub>")?;
    let inner = after_open[..close_pos].to_string();
    *rest = after_open[close_pos + "</sub>".len()..].trim();
    Some(inner)
}

/// Parse a markdown link `[label](url)` and return `(label, url)`.
fn parse_markdown_link(s: &str) -> Option<(&str, &str)> {
    let (before, after) = s.split_once("](")?;
    let label = before.strip_prefix('[')?;
    let url = after.strip_suffix(')')?;
    Some((label, url))
}

/// Parse the trailing `<sub>` element that contains the timestamp
/// and an optional prompt link.
///
/// Formats:
/// - `<sub>[YYYY-MM-DD HH:MM:SS +HHMM](url)</sub>` — timestamp as link text
/// - `<sub>YYYY-MM-DD HH:MM:SS +HHMM</sub>` — plain timestamp
fn parse_trailing_timestamp_sub(
    rest: &mut &str,
) -> Result<(chrono::DateTime<chrono::FixedOffset>, Option<String>)> {
    let inner = try_parse_next_sub(rest)
        .ok_or_else(|| anyhow::anyhow!("Missing trailing <sub> timestamp element"))?;

    if let Some((ts_str, url)) = parse_markdown_link(&inner) {
        let timestamp = chrono::DateTime::parse_from_str(ts_str, "%Y-%m-%d %H:%M:%S %z")
            .with_context(|| format!("Invalid timestamp in link: {}", ts_str))?;
        Ok((timestamp, Some(url.to_string())))
    } else {
        let timestamp =
            chrono::DateTime::parse_from_str(inner.trim(), "%Y-%m-%d %H:%M:%S %z")
                .with_context(|| format!("Invalid timestamp: {}", inner))?;
        Ok((timestamp, None))
    }
}

// -- Display variant without prompt link (for prompts) ------------------------

/// A wrapper that serializes a `MdStageTitle` without the prompt link.
#[allow(dead_code)]
pub struct MdMdStageTitleForPrompt<'a>(pub &'a MdStageTitle);

impl fmt::Display for MdMdStageTitleForPrompt<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let t = self.0;
        write!(
            f,
            "{}",
            PipelineStage {
                pipeline: &t.pipeline,
                run_id: t.run_id,
                stage: &t.stage,
            },
        )?;
        if let Some(tool) = &t.tool {
            write!(f, " {}", Backtick(tool))?;
        }
        if let Some(model) = &t.model {
            write!(f, " {}", Backtick(model))?;
        }
        write!(f, " <sub>{}</sub>", format_timestamp(&t.timestamp))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_title() -> MdStageTitle {
        MdStageTitle {
            timestamp: chrono::DateTime::parse_from_rfc3339("2024-06-15T10:30:00+03:00").unwrap(),
            pipeline: Pipeline::from("main"),
            run_id: 2,
            stage: Stage::new("working"),
            tool: Some(Tool::Claude),
            model: Some(Model::ClaudeOpus4_6),
            prompt_link: Some("prompts/work.md".to_string()),
        }
    }

    #[test]
    fn display_roundtrip() {
        let title = make_title();
        let s = title.to_string();
        let parsed: MdStageTitle = s.parse().unwrap();
        assert_eq!(parsed, title);
    }

    #[test]
    fn display_format() {
        let title = make_title();
        let s = title.to_string();
        assert_eq!(
            s,
            "main:2:**working** `claude` `claude-opus-4.6` <sub>[2024-06-15 10:30:00 +0300](prompts/work.md)</sub>"
        );
    }

    #[test]
    fn parse_with_list_prefix() {
        let title = make_title();
        let s = format!("- {}", title);
        let parsed: MdStageTitle = s.parse().unwrap();
        assert_eq!(parsed, title);
    }

    #[test]
    fn display_without_optionals() {
        let title = MdStageTitle {
            timestamp: chrono::DateTime::parse_from_rfc3339("2024-01-01T00:00:00+00:00").unwrap(),
            pipeline: Pipeline::from("merge"),
            run_id: 1,
            stage: Stage::new("review"),
            tool: None,
            model: None,
            prompt_link: None,
        };
        let s = title.to_string();
        assert_eq!(s, "merge:1:**review** <sub>2024-01-01 00:00:00 +0000</sub>");
        let parsed: MdStageTitle = s.parse().unwrap();
        assert_eq!(parsed, title);
    }

    #[test]
    fn for_prompt_omits_link() {
        let title = make_title();
        let full = title.to_string();
        let prompt = MdMdStageTitleForPrompt(&title).to_string();
        // Full version has the prompt URL in the link
        assert!(full.contains("](prompts/work.md)"));
        // Prompt version has timestamp but no link
        assert!(!prompt.contains("](prompts/work.md)"));
        assert!(prompt.contains("<sub>2024-06-15 10:30:00 +0300</sub>"));
    }
}
