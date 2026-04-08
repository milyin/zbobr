//! Stage title serialization/deserialization.
//!
//! The stage title is a markdown line with the format:
//! ```text
//! pipeline:run_id:**stage** `tool` `model` `YYYY-MM-DD HH:MM:SS +HHMM` <sub>[prompt](prompt_url)</sub> <sub>[output](output_url)</sub>
//! ```
//!
//! The prompt and output sub-links are optional. When there are no links:
//! ```text
//! pipeline:run_id:**stage** `tool` `model` `YYYY-MM-DD HH:MM:SS +HHMM`
//! ```
//!
//! This module provides [`MdStageTitle`] which can be converted to/from a string
//! via `Display`/`FromStr`, and to/from [`StageInfo`] for use in the rest of the codebase.
//!
//! `Serialize`/`Deserialize` delegate to `Display`/`FromStr` for serde compatibility.

use std::{fmt, str::FromStr};

use anyhow::{Context as _, Result};

/// Label used for the prompt sub-link in the stage title markdown.
const PROMPT_LABEL: &str = "prompt";
/// Label used for the output sub-link in the stage title markdown.
const OUTPUT_LABEL: &str = "output";

use crate::task::{Model, Pipeline, Stage, StageInfo};

/// A parsed stage title line, mapping 1:1 to the markdown format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MdStageTitle {
    pub instance: String,
    pub timestamp: chrono::DateTime<chrono::FixedOffset>,
    pub pipeline: Pipeline,
    pub run_id: u64,
    pub stage: Stage,
    pub tool: Option<String>,
    pub model: Option<Model>,
    pub prompt_link: Option<String>,
    pub output_link: Option<String>,
}

impl From<&StageInfo> for MdStageTitle {
    fn from(info: &StageInfo) -> Self {
        MdStageTitle {
            instance: info.instance.clone(),
            timestamp: info.timestamp,
            pipeline: info.pipeline.clone(),
            run_id: info.run_id,
            stage: info.stage.clone(),
            tool: info.tool.clone(),
            model: info.model.clone(),
            prompt_link: info.prompt_link.clone(),
            output_link: info.output_link.clone(),
        }
    }
}

impl From<MdStageTitle> for StageInfo {
    fn from(t: MdStageTitle) -> Self {
        StageInfo {
            instance: t.instance,
            timestamp: t.timestamp,
            pipeline: t.pipeline,
            run_id: t.run_id,
            stage: t.stage,
            tool: t.tool,
            model: t.model,
            prompt_link: t.prompt_link,
            output_link: t.output_link,
        }
    }
}

// -- Display (serialization) -------------------------------------------------

/// Wrapper: `instance:pipeline:run_id:**stage**`
struct PipelineStage<'a> {
    instance: &'a str,
    pipeline: &'a Pipeline,
    run_id: u64,
    stage: &'a Stage,
}

impl fmt::Display for PipelineStage<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}:**{}**",
            self.instance, self.pipeline, self.run_id, self.stage
        )
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
pub fn format_timestamp(ts: &chrono::DateTime<chrono::FixedOffset>) -> String {
    format!("{} {}", ts.format("%Y-%m-%d %H:%M:%S"), ts.format("%z"))
}

impl fmt::Display for MdStageTitle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            PipelineStage {
                instance: &self.instance,
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
        write!(f, " {}", Backtick(format_timestamp(&self.timestamp)))?;
        if let Some(link) = &self.prompt_link {
            write!(f, " <sub>[{PROMPT_LABEL}]({})</sub>", link)?;
        }
        if let Some(link) = &self.output_link {
            write!(f, " <sub>[{OUTPUT_LABEL}]({})</sub>", link)?;
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

        // 1. instance:pipeline:run_id:**stage**
        let (instance, pipeline, run_id, stage) = parse_next_pipeline_stage(&mut rest)?;

        // 2. Optional backtick tokens (tool, model, timestamp)
        let mut tool = None;
        let mut model = None;
        let mut timestamp_from_backtick = None;

        while !rest.is_empty() {
            if let Some(value) = try_parse_next_backtick(&mut rest) {
                // A backtick containing a space is a timestamp (dates look like
                // "2024-01-01 00:00:00 +0000"); tool names and models never have
                // spaces — this invariant is enforced at the type level by `Model`.
                if let Ok(ts) = chrono::DateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S %z") {
                    timestamp_from_backtick = Some(ts);
                    break;
                }
                if tool.is_none() {
                    tool = Some(value.to_string());
                } else if model.is_none() {
                    model = Some(value.parse::<Model>().map_err(|e| {
                        anyhow::anyhow!("Invalid model token {:?} in stage title: {e}", value)
                    })?);
                }
                continue;
            }
            break;
        }

        // 3. Parse timestamp and optional links
        let timestamp = timestamp_from_backtick
            .ok_or_else(|| anyhow::anyhow!("Missing backtick timestamp in stage title"))?;
        let mut prompt_link = None;
        let mut output_link = None;
        while !rest.is_empty() {
            if let Some(inner) = try_parse_next_sub(&mut rest) {
                if let Some((label, url)) = parse_markdown_link(&inner) {
                    match label {
                        PROMPT_LABEL => prompt_link = Some(url.to_string()),
                        OUTPUT_LABEL => output_link = Some(url.to_string()),
                        _ => {}
                    }
                }
                continue;
            }
            break;
        }

        Ok(MdStageTitle {
            instance,
            timestamp,
            pipeline,
            run_id,
            stage,
            tool,
            model,
            prompt_link,
            output_link,
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

fn parse_next_pipeline_stage(rest: &mut &str) -> Result<(String, Pipeline, u64, Stage)> {
    let stage_marker = ":**";
    let marker_pos = rest
        .find(stage_marker)
        .ok_or_else(|| anyhow::anyhow!("Missing ':**' stage marker"))?;
    let instance_pipeline_and_run = &rest[..marker_pos];
    let after_marker = &rest[marker_pos + stage_marker.len()..];

    // Format: instance:pipeline:run_id
    // Split on the last ':' to get run_id, then split the remainder on the first ':' to get instance and pipeline.
    let run_sep = instance_pipeline_and_run
        .rfind(':')
        .ok_or_else(|| anyhow::anyhow!("Missing ':' before run_id"))?;
    let run_id: u64 = instance_pipeline_and_run[run_sep + 1..]
        .trim()
        .parse()
        .context("Invalid run_id")?;
    let instance_and_pipeline = instance_pipeline_and_run[..run_sep].trim();

    let pipeline_sep = instance_and_pipeline
        .find(':')
        .ok_or_else(|| anyhow::anyhow!("Missing ':' between instance and pipeline"))?;
    let instance_str = instance_and_pipeline[..pipeline_sep].trim();
    let pipeline_str = instance_and_pipeline[pipeline_sep + 1..].trim();

    let stage_end = after_marker
        .find("**")
        .ok_or_else(|| anyhow::anyhow!("Missing closing '**' for stage"))?;
    let stage_str = after_marker[..stage_end].trim();
    *rest = after_marker[stage_end + 2..].trim();

    Ok((
        instance_str.to_string(),
        pipeline_str.parse().unwrap(),
        run_id,
        Stage::from(stage_str),
    ))
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

// -- Display variant without prompt link (for prompts) ------------------------

/// A wrapper that serializes a `MdStageTitle` without the prompt or output links.
#[allow(dead_code)]
pub struct MdMdStageTitleForPrompt<'a>(pub &'a MdStageTitle);

impl fmt::Display for MdMdStageTitleForPrompt<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let t = self.0;
        write!(
            f,
            "{}",
            PipelineStage {
                instance: &t.instance,
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
        write!(f, " {}", Backtick(format_timestamp(&t.timestamp)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_title() -> MdStageTitle {
        MdStageTitle {
            instance: "myinstance".to_string(),
            timestamp: chrono::DateTime::parse_from_rfc3339("2024-06-15T10:30:00+03:00").unwrap(),
            pipeline: Pipeline::from("main"),
            run_id: 2,
            stage: Stage::new("working"),
            tool: Some("claude".to_string()),
            model: Some("claude-opus-4.6".parse().unwrap()),
            prompt_link: Some("prompts/work.md".to_string()),
            output_link: Some("output/work.md".to_string()),
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
            "myinstance:main:2:**working** `claude` `claude-opus-4.6` `2024-06-15 10:30:00 +0300` <sub>[prompt](prompts/work.md)</sub> <sub>[output](output/work.md)</sub>"
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
            instance: "default".to_string(),
            timestamp: chrono::DateTime::parse_from_rfc3339("2024-01-01T00:00:00+00:00").unwrap(),
            pipeline: Pipeline::from("merge"),
            run_id: 1,
            stage: Stage::new("review"),
            tool: None,
            model: None,
            prompt_link: None,
            output_link: None,
        };
        let s = title.to_string();
        assert_eq!(s, "default:merge:1:**review** `2024-01-01 00:00:00 +0000`");
        let parsed: MdStageTitle = s.parse().unwrap();
        assert_eq!(parsed, title);
    }

    #[test]
    fn display_with_prompt_only() {
        let title = MdStageTitle {
            instance: "default".to_string(),
            timestamp: chrono::DateTime::parse_from_rfc3339("2024-01-01T00:00:00+00:00").unwrap(),
            pipeline: Pipeline::from("merge"),
            run_id: 1,
            stage: Stage::new("review"),
            tool: None,
            model: None,
            prompt_link: Some("prompts/review.md".to_string()),
            output_link: None,
        };
        let s = title.to_string();
        assert_eq!(
            s,
            "default:merge:1:**review** `2024-01-01 00:00:00 +0000` <sub>[prompt](prompts/review.md)</sub>"
        );
        let parsed: MdStageTitle = s.parse().unwrap();
        assert_eq!(parsed, title);
    }

    #[test]
    fn for_prompt_omits_links() {
        let title = make_title();
        let full = title.to_string();
        let prompt = MdMdStageTitleForPrompt(&title).to_string();
        // Full version has the links
        assert!(full.contains("<sub>[prompt](prompts/work.md)</sub>"));
        assert!(full.contains("<sub>[output](output/work.md)</sub>"));
        // Prompt version has timestamp in backtick but no links
        assert!(!prompt.contains("<sub>"));
        assert!(prompt.contains("`2024-06-15 10:30:00 +0300`"));
    }

    #[test]
    fn parse_rejects_malformed_model_token() {
        // Valid tool backtick but model backtick contains a space → should fail
        let s = "myinstance:main:2:**working** `claude` `bad model` `2024-06-15 10:30:00 +0300`";
        let result = s.parse::<MdStageTitle>();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Invalid model token"),
            "Expected 'Invalid model token' in error, got: {err_msg}"
        );
    }

    #[test]
    fn parse_accepts_valid_model_token() {
        let s =
            "myinstance:main:2:**working** `claude` `claude-opus-4.6` `2024-06-15 10:30:00 +0300`";
        let parsed: MdStageTitle = s.parse().unwrap();
        assert_eq!(parsed.tool.as_deref(), Some("claude"));
        assert_eq!(parsed.model.as_ref().unwrap().as_str(), "claude-opus-4.6");
    }
}
