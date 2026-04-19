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

use crate::task::TaskContext;

/// Opening marker of the JSON block. The trailing newline is part of the
/// marker so the JSON starts on its own line, which is both prettier and
/// easier to parse.
const OPEN_MARKER: &str = "<!-- zbobr-ctx-v1\n";

/// Closing marker of the JSON block.
const CLOSE_MARKER: &str = "\n-->";

/// Serialize a `TaskContext` as a pretty-printed JSON block wrapped in the
/// `zbobr-ctx-v1` HTML comment envelope.
///
/// The returned string starts with `OPEN_MARKER` and ends with `CLOSE_MARKER`.
/// Callers typically want to append a trailing newline before the rendered
/// markdown view.
pub(super) fn serialize_json_block(ctx: &TaskContext) -> String {
    // serde_json only fails on non-string map keys or non-finite floats;
    // TaskContext has neither, so `expect` is safe.
    let json =
        serde_json::to_string_pretty(ctx).expect("TaskContext JSON serialization is infallible");
    format!("{OPEN_MARKER}{json}{CLOSE_MARKER}")
}

/// Locate a `zbobr-ctx-v1` block anywhere in `text` and parse its JSON payload.
///
/// Returns:
/// - `None` if the marker is absent — caller should fall back to the legacy
///   markdown parser.
/// - `Some(Ok(ctx))` on a successful parse.
/// - `Some(Err(_))` if the marker is present but the payload is missing,
///   truncated, or not valid JSON. Never silently drop a block we recognise.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{
        ContextRecord, ContextRecordType, Pipeline, Stage, StageContext, StageInfo,
    };

    fn utc(s: &str) -> chrono::DateTime<chrono::FixedOffset> {
        s.parse().unwrap()
    }

    fn sample() -> TaskContext {
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

    #[test]
    fn roundtrip() {
        let ctx = sample();
        let block = serialize_json_block(&ctx);
        let parsed = parse_json_block(&block).expect("marker present").unwrap();
        assert_eq!(parsed, ctx);
    }

    #[test]
    fn block_uses_expected_envelope() {
        let ctx = sample();
        let block = serialize_json_block(&ctx);
        assert!(block.starts_with("<!-- zbobr-ctx-v1\n"));
        assert!(block.ends_with("\n-->"));
    }

    #[test]
    fn missing_marker_returns_none() {
        assert!(parse_json_block("just some markdown").is_none());
    }

    #[test]
    fn unclosed_block_is_error() {
        let bad = "<!-- zbobr-ctx-v1\n{\"stages\":[]}";
        let err = parse_json_block(bad).expect("marker present").unwrap_err();
        assert!(err.to_string().contains("not closed"));
    }

    #[test]
    fn malformed_json_is_error() {
        let bad = "<!-- zbobr-ctx-v1\n{not json}\n-->";
        assert!(parse_json_block(bad).expect("marker present").is_err());
    }

    #[test]
    fn block_can_be_embedded_in_surrounding_markdown() {
        let ctx = sample();
        let block = serialize_json_block(&ctx);
        let wrapped = format!("some prose above\n\n{block}\n\nmore prose below\n");
        let parsed = parse_json_block(&wrapped).unwrap().unwrap();
        assert_eq!(parsed, ctx);
    }

    #[test]
    fn unknown_fields_are_ignored() {
        // Forward-compat: a future writer could add fields; an older reader
        // must still accept the payload.
        let payload = r#"<!-- zbobr-ctx-v1
{"stages":[],"future_field":42}
-->"#;
        let parsed = parse_json_block(payload).unwrap().unwrap();
        assert!(parsed.stages.is_empty());
    }
}
