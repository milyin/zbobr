//! Stage title rendering for the human-readable markdown view.
//!
//! The stage title is a single markdown list item:
//! ```text
//! instance:pipeline:**stage** `tool` `model` `YYYY-MM-DD HH:MM:SS +HHMM` <sub>[prompt](url)</sub> <sub>[output](url)</sub>
//! ```
//! This module provides [`MdStageTitle`] with `Display` and `From<&StageInfo>`
//! conversions used when generating the human-readable context view.

use std::fmt;

use crate::task::{Model, Pipeline, Stage, StageInfo};

/// Label used for the prompt sub-link in the stage title markdown.
const PROMPT_LABEL: &str = "prompt";
/// Label used for the output sub-link in the stage title markdown.
const OUTPUT_LABEL: &str = "output";

/// A stage title line in its markdown form.
#[derive(Debug, Clone)]
pub struct MdStageTitle {
    pub instance: String,
    pub timestamp: chrono::DateTime<chrono::FixedOffset>,
    pub pipeline: Pipeline,
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
            stage: t.stage,
            tool: t.tool,
            model: t.model,
            prompt_link: t.prompt_link,
            output_link: t.output_link,
        }
    }
}

// -- Display ------------------------------------------------------------------

/// Wrapper: `instance:pipeline:**stage**`
struct PipelineStage<'a> {
    instance: &'a str,
    pipeline: &'a Pipeline,
    stage: &'a Stage,
}

impl fmt::Display for PipelineStage<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:**{}**", self.instance, self.pipeline, self.stage)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_title() -> MdStageTitle {
        MdStageTitle {
            instance: "myinstance".to_string(),
            timestamp: chrono::DateTime::parse_from_rfc3339("2024-06-15T10:30:00+03:00").unwrap(),
            pipeline: Pipeline::from("main"),
            stage: Stage::new("working"),
            tool: Some("claude".to_string()),
            model: Some("claude-opus-4.6".parse().unwrap()),
            prompt_link: Some("prompts/work.md".to_string()),
            output_link: Some("output/work.md".to_string()),
        }
    }

    #[test]
    fn display_format() {
        let title = make_title();
        let s = title.to_string();
        assert_eq!(
            s,
            "myinstance:main:**working** `claude` `claude-opus-4.6` `2024-06-15 10:30:00 +0300` <sub>[prompt](prompts/work.md)</sub> <sub>[output](output/work.md)</sub>"
        );
    }

    #[test]
    fn display_without_optionals() {
        let title = MdStageTitle {
            instance: "default".to_string(),
            timestamp: chrono::DateTime::parse_from_rfc3339("2024-01-01T00:00:00+00:00").unwrap(),
            pipeline: Pipeline::from("merge"),
            stage: Stage::new("review"),
            tool: None,
            model: None,
            prompt_link: None,
            output_link: None,
        };
        let s = title.to_string();
        assert_eq!(s, "default:merge:**review** `2024-01-01 00:00:00 +0000`");
    }

    #[test]
    fn display_with_prompt_only() {
        let title = MdStageTitle {
            instance: "default".to_string(),
            timestamp: chrono::DateTime::parse_from_rfc3339("2024-01-01T00:00:00+00:00").unwrap(),
            pipeline: Pipeline::from("merge"),
            stage: Stage::new("review"),
            tool: None,
            model: None,
            prompt_link: Some("prompts/review.md".to_string()),
            output_link: None,
        };
        let s = title.to_string();
        assert_eq!(
            s,
            "default:merge:**review** `2024-01-01 00:00:00 +0000` <sub>[prompt](prompts/review.md)</sub>"
        );
    }
}
