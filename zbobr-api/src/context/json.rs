//! JSON-backed context format.
//!
//! The context section of a task description embeds its source of truth as a
//! pretty-printed JSON payload wrapped in an HTML comment:
//!
//! ```text
//! <!-- zbobr-ctx-v1
//! { "stages": [ ... ] }
//! -->
//! ```
//!
//! GitHub (and most markdown renderers) hide HTML comments, so the block is
//! invisible in rendered output. The rendered markdown below the block is a
//! human-readable view regenerated on every write — the JSON wins on parse.
//!
//! If the marker is absent, callers fall back to the legacy markdown parser.
//! If the marker is present but the payload is malformed, parsing fails loudly
//! rather than silently discarding data.
//!
//! Design notes:
//! - Comments are not persisted in this payload: they live as separate
//!   GitHub issue comments and are re-fetched alongside the task.
//! - `report_link` / `prompt_link` / `output_link` are stored as raw values
//!   (exactly what's on `TaskContext`). URL rewriting is a display concern
//!   applied only when rendering the markdown view.
//! - Unknown fields in the JSON are ignored by serde (default behaviour), so
//!   newer writers can add fields without breaking older readers.
//! - The `v1` suffix lets us bump the format without ambiguity.
//!
//! # Security / trust
//!
//! The payload is parsed with `serde_json`, which rejects JSON bombs and
//! malformed input. Unknown fields are silently dropped.
//!
//! # Examples
//!
//! Emit a JSON block and parse it back:
//!
//! ```ignore
//! let block = serialize_json_block(&ctx);
//! let parsed = parse_json_block(&block).unwrap().unwrap();
//! assert_eq!(parsed, ctx);
//! ```
//!
//! When the marker is missing, the parser returns `None` so callers can fall
//! back to the legacy format:
//!
//! ```ignore
//! assert!(parse_json_block("legacy markdown").is_none());
//! ```

use anyhow::{Context, Result, anyhow};

use crate::task::{StageContext, TaskContext};

/// Closing marker shared by all JSON block variants.
const CLOSE_MARKER: &str = "\n-->";

// ── Legacy single-block format (zbobr-ctx-v1) ───────────────────────────────
// Kept for reading tasks written before the per-stage format was introduced.
// New serialization uses the per-stage format below.

const OPEN_MARKER: &str = "<!-- zbobr-ctx-v1\n";

/// Parse a legacy `zbobr-ctx-v1` block (whole `TaskContext` in one comment).
///
/// Returns `None` if the marker is absent, `Some(Err(_))` if malformed.
pub(super) fn parse_json_block(text: &str) -> Option<Result<TaskContext>> {
    let open_idx = text.find(OPEN_MARKER)?;
    let after_open = &text[open_idx + OPEN_MARKER.len()..];
    let close_rel = match after_open.find(CLOSE_MARKER) {
        Some(i) => i,
        None => {
            return Some(Err(anyhow!(
                "zbobr-ctx-v1 block is not closed — missing {CLOSE_MARKER:?}"
            )));
        }
    };
    let payload = &after_open[..close_rel];
    let parsed: Result<TaskContext> = serde_json::from_str(payload)
        .with_context(|| "failed to parse zbobr-ctx-v1 JSON payload");
    Some(parsed)
}

// ── Current per-stage format (zbobr-ctx-v1-stage) ───────────────────────────
// Each stage has its own JSON block placed immediately before its markdown
// rendering.  This allows the user to reposition the `---CONTEXT---` separator
// anywhere inside the context section to move older stages to DEAD_CONTEXT
// without losing their structured data.

const STAGE_OPEN_MARKER: &str = "<!-- zbobr-ctx-v1-stage\n";

/// Serialize a single `StageContext` as a pretty-printed JSON block wrapped in
/// the `zbobr-ctx-v1-stage` HTML comment envelope.
pub(super) fn serialize_stage_json_block(stage: &StageContext) -> String {
    let json = serde_json::to_string_pretty(stage)
        .expect("StageContext JSON serialization is infallible");
    format!("{STAGE_OPEN_MARKER}{json}{CLOSE_MARKER}")
}

/// Locate all `zbobr-ctx-v1-stage` blocks in `text` and parse them in order.
///
/// Returns:
/// - `None` if no stage marker is present.
/// - `Some(Ok(stages))` when all blocks parsed successfully.
/// - `Some(Err(_))` if any block is unclosed or contains invalid JSON.
pub(super) fn parse_stage_json_blocks(text: &str) -> Option<Result<Vec<StageContext>>> {
    if !text.contains(STAGE_OPEN_MARKER) {
        return None;
    }
    let mut stages = Vec::new();
    let mut search_from = 0usize;
    loop {
        let Some(rel) = text[search_from..].find(STAGE_OPEN_MARKER) else {
            break;
        };
        let open_idx = search_from + rel;
        let after_open = &text[open_idx + STAGE_OPEN_MARKER.len()..];
        let close_rel = match after_open.find(CLOSE_MARKER) {
            Some(i) => i,
            None => {
                return Some(Err(anyhow!(
                    "zbobr-ctx-v1-stage block is not closed — missing {CLOSE_MARKER:?}"
                )));
            }
        };
        let payload = &after_open[..close_rel];
        let parsed: Result<StageContext> = serde_json::from_str(payload)
            .with_context(|| "failed to parse zbobr-ctx-v1-stage JSON payload");
        match parsed {
            Ok(stage) => stages.push(stage),
            Err(e) => return Some(Err(e)),
        }
        search_from =
            open_idx + STAGE_OPEN_MARKER.len() + close_rel + CLOSE_MARKER.len();
    }
    Some(Ok(stages))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{
        ContextRecord, ContextRecordType, Pipeline, Stage, StageContext, StageInfo,
    };

    fn utc(s: &str) -> chrono::DateTime<chrono::FixedOffset> {
        s.parse().unwrap()
    }

    fn sample_task_context() -> TaskContext {
        TaskContext {
            stages: vec![StageContext {
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
                records: vec![ContextRecord {
                    id: 1,
                    record_type: ContextRecordType::Checkbox(false),
                    brief: "item".to_string(),
                    report_link: Some("reports/r.md".to_string()),
                }],
            }],
        }
    }

    fn sample_stage() -> StageContext {
        sample_task_context().stages.into_iter().next().unwrap()
    }

    // ── Legacy single-block format tests ─────────────────────────────────────

    #[test]
    fn legacy_roundtrip() {
        let ctx = sample_task_context();
        let block = parse_json_block(&format!(
            "<!-- zbobr-ctx-v1\n{}\n-->",
            serde_json::to_string_pretty(&ctx).unwrap()
        ))
        .expect("marker present")
        .unwrap();
        assert_eq!(block, ctx);
    }

    #[test]
    fn legacy_block_uses_expected_envelope() {
        // Manually build the old format for the envelope check.
        let payload = r#"<!-- zbobr-ctx-v1
{"stages":[]}
-->"#;
        assert!(payload.starts_with("<!-- zbobr-ctx-v1\n"));
        assert!(payload.ends_with("\n-->"));
        let parsed = parse_json_block(payload).unwrap().unwrap();
        assert!(parsed.stages.is_empty());
    }

    #[test]
    fn legacy_missing_marker_returns_none() {
        assert!(parse_json_block("just some markdown").is_none());
    }

    #[test]
    fn legacy_unclosed_block_is_error() {
        let bad = "<!-- zbobr-ctx-v1\n{\"stages\":[]}";
        let err = parse_json_block(bad).expect("marker present").unwrap_err();
        assert!(err.to_string().contains("not closed"));
    }

    #[test]
    fn legacy_malformed_json_is_error() {
        let bad = "<!-- zbobr-ctx-v1\n{not json}\n-->";
        assert!(parse_json_block(bad).expect("marker present").is_err());
    }

    #[test]
    fn legacy_block_can_be_embedded_in_surrounding_markdown() {
        let ctx = sample_task_context();
        let payload = serde_json::to_string_pretty(&ctx).unwrap();
        let block = format!("<!-- zbobr-ctx-v1\n{payload}\n-->");
        let wrapped = format!("some prose above\n\n{block}\n\nmore prose below\n");
        let parsed = parse_json_block(&wrapped).unwrap().unwrap();
        assert_eq!(parsed, ctx);
    }

    #[test]
    fn legacy_unknown_fields_are_ignored() {
        let payload = r#"<!-- zbobr-ctx-v1
{"stages":[],"future_field":42}
-->"#;
        let parsed = parse_json_block(payload).unwrap().unwrap();
        assert!(parsed.stages.is_empty());
    }

    // ── Per-stage format tests ────────────────────────────────────────────────

    #[test]
    fn stage_roundtrip() {
        let stage = sample_stage();
        let block = serialize_stage_json_block(&stage);
        let stages = parse_stage_json_blocks(&block)
            .expect("marker present")
            .unwrap();
        assert_eq!(stages.len(), 1);
        assert_eq!(stages[0], stage);
    }

    #[test]
    fn stage_block_uses_expected_envelope() {
        let stage = sample_stage();
        let block = serialize_stage_json_block(&stage);
        assert!(block.starts_with("<!-- zbobr-ctx-v1-stage\n"));
        assert!(block.ends_with("\n-->"));
    }

    #[test]
    fn stage_missing_marker_returns_none() {
        assert!(parse_stage_json_blocks("just some markdown").is_none());
    }

    #[test]
    fn stage_unclosed_block_is_error() {
        let bad = "<!-- zbobr-ctx-v1-stage\n{\"info\":{}}";
        let err = parse_stage_json_blocks(bad)
            .expect("marker present")
            .unwrap_err();
        assert!(err.to_string().contains("not closed"));
    }

    #[test]
    fn stage_malformed_json_is_error() {
        let bad = "<!-- zbobr-ctx-v1-stage\n{not json}\n-->";
        assert!(parse_stage_json_blocks(bad)
            .expect("marker present")
            .is_err());
    }

    #[test]
    fn multiple_stage_blocks_parsed_in_order() {
        let stage1 = sample_stage();
        let mut stage2 = sample_stage();
        stage2.info.stage = Stage::new("working");

        let block1 = serialize_stage_json_block(&stage1);
        let block2 = serialize_stage_json_block(&stage2);
        let text = format!("{block1}\n- some markdown\n\n{block2}\n- more markdown\n");

        let stages = parse_stage_json_blocks(&text)
            .expect("markers present")
            .unwrap();
        assert_eq!(stages.len(), 2);
        assert_eq!(stages[0], stage1);
        assert_eq!(stages[1], stage2);
    }

    #[test]
    fn stage_block_embedded_in_surrounding_text() {
        let stage = sample_stage();
        let block = serialize_stage_json_block(&stage);
        let wrapped = format!("prose before\n\n{block}\n- markdown after\n");
        let stages = parse_stage_json_blocks(&wrapped).unwrap().unwrap();
        assert_eq!(stages.len(), 1);
        assert_eq!(stages[0], stage);
    }

    #[test]
    fn stage_unknown_fields_are_ignored() {
        let stage = sample_stage();
        let mut json_val: serde_json::Value = serde_json::to_value(&stage).unwrap();
        json_val["future_field"] = serde_json::json!(42);
        let payload = serde_json::to_string_pretty(&json_val).unwrap();
        let text = format!("<!-- zbobr-ctx-v1-stage\n{payload}\n-->");
        let stages = parse_stage_json_blocks(&text).unwrap().unwrap();
        assert_eq!(stages.len(), 1);
        assert_eq!(stages[0].info.stage, stage.info.stage);
    }

    #[test]
    fn stage_marker_does_not_match_legacy_marker() {
        // A text with the old "zbobr-ctx-v1\n" marker (no "-stage") must NOT
        // be picked up by parse_stage_json_blocks.
        let ctx = sample_task_context();
        let payload = serde_json::to_string_pretty(&ctx).unwrap();
        let old_block = format!("<!-- zbobr-ctx-v1\n{payload}\n-->");
        assert!(
            parse_stage_json_blocks(&old_block).is_none(),
            "old-format block must not match per-stage parser"
        );
    }
}
