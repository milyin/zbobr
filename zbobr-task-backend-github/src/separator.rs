use std::collections::HashMap;

use zbobr_api::{
    checklist_format::{parse_grouped_checklist, serialize_grouped_checklist},
    task::ChecklistItem,
};

// -- Checklist parsing and serialization helpers --

pub(crate) const PARAMETERS_SEPARATOR: &str = "\n\n---PARAMETERS---\n";
pub(crate) const ERROR_SEPARATOR: &str = "\n\n---ERROR---\n";
pub(crate) const CHECKLIST_SEPARATOR: &str = "\n\n---CHECKLIST---\n";

/// Parse parameters from the PARAMETERS section.
/// Returns a map of parameter names to values.
pub(crate) fn parse_parameters(params_text: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();
    for line in params_text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(colon_pos) = line.find(':') {
            let key = line[..colon_pos].trim().to_string();
            let value = line[colon_pos + 1..].trim().to_string();
            params.insert(key, value);
        }
    }
    params
}

/// Serialize parameters into the PARAMETERS section format.
pub(crate) fn serialize_parameters(params: &HashMap<String, String>) -> String {
    let mut result = String::new();
    for (key, value) in params {
        result.push_str(&format!("{}: {}\n", key, value));
    }
    result
}

/// Parse a task description into (description, parameters, error, checklist).
/// Section order: description → PARAMETERS → ERROR → CHECKLIST.
pub(crate) fn parse_description_full(
    full_text: &str,
) -> (String, HashMap<String, String>, Option<String>, Vec<ChecklistItem>) {
    // Normalize line endings so separators match regardless of \r\n vs \n.
    let normalized = if full_text.contains("\r\n") {
        full_text.replace("\r\n", "\n")
    } else {
        full_text.to_string()
    };

    // First split by checklist
    let parts: Vec<&str> = normalized.split(CHECKLIST_SEPARATOR).collect();

    let (before_checklist, checklist_text) = match parts.len() {
        1 => (parts[0], ""),
        _ => (parts[0], parts[1]),
    };

    // Split by error separator
    let error_parts: Vec<&str> = before_checklist.split(ERROR_SEPARATOR).collect();
    let (before_error, error_text) = match error_parts.len() {
        1 => (error_parts[0], None),
        _ => {
            let text = error_parts[1].trim();
            (error_parts[0], if text.is_empty() { None } else { Some(text) })
        }
    };

    // Now split by parameters separator
    let param_parts: Vec<&str> = before_error.split(PARAMETERS_SEPARATOR).collect();
    let (description, params_text) = match param_parts.len() {
        1 => (param_parts[0].to_string(), ""),
        _ => (param_parts[0].to_string(), param_parts[1].trim()),
    };

    // Parse parameters
    let parameters = parse_parameters(params_text);

    let error = error_text.map(|s| s.to_string());

    // Parse checklist items using shared format
    let items = parse_grouped_checklist(checklist_text);

    (description, parameters, error, items)
}

/// Serialize description, parameters, error, and checklist items back into the full format.
/// Items are grouped by pipeline run with visual headers for clarity.
/// Legacy plan sections are not included; they should be managed via Plan comments.
/// Section order: description → PARAMETERS → ERROR → CHECKLIST.
pub(crate) fn serialize_description_full(
    original_description: &str,
    parameters: &HashMap<String, String>,
    error: &Option<String>,
    items: &[ChecklistItem],
) -> String {
    // Strip everything from the description first
    let (clean_description, _, _, _) = parse_description_full(original_description);

    let mut result = clean_description;

    // Add parameters if present
    if !parameters.is_empty() {
        result.push_str(PARAMETERS_SEPARATOR);
        result.push_str(&serialize_parameters(parameters));
    }

    // Add error if present
    if let Some(err) = error {
        result.push_str(ERROR_SEPARATOR);
        result.push_str(err);
        result.push('\n');
    }

    // Add checklist if present using shared format
    if !items.is_empty() {
        result.push_str(CHECKLIST_SEPARATOR);
        result.push_str(&serialize_grouped_checklist(items));
    }

    result
}

/// Merge concurrent updates to a task description.
///
/// This function handles the case where two concurrent updates have been made to different
/// sections of the task description (description, parameters, plan, checklist).
///
/// Given:
/// - `original`: The description as it was when we first read it
/// - `current`: The description as it exists now (after someone else modified it)
/// - `our_new`: The description we want to write
///
/// This function extracts what parts we modified vs what parts someone else modified,
/// and merges them intelligently:
/// - If we both modified the same section, our change wins (last write wins, simplified)
/// - If only one of us modified a section, that modification is preserved
///
/// The strategy is to parse all three versions, detect what changed in each,
/// and prefer newer values while preserving non-conflicting changes.
pub(crate) fn merge_concurrent_description_updates(
    original: &str,
    current: &str,
    our_new: &str,
) -> String {
    // Parse all three versions
    let (orig_desc, orig_params, orig_error, orig_checklist) = parse_description_full(original);
    let (curr_desc, curr_params, curr_error, curr_checklist) = parse_description_full(current);
    let (new_desc, new_params, new_error, new_checklist) = parse_description_full(our_new);

    // Determine what we changed
    let we_changed_desc = new_desc != orig_desc;
    let we_changed_params = new_params != orig_params;
    let we_changed_error = new_error != orig_error;
    let we_changed_checklist = serde_json::to_string(&new_checklist).unwrap_or_default()
        != serde_json::to_string(&orig_checklist).unwrap_or_default();

    // Merge: prefer our changes if we made them, otherwise prefer their changes
    let merged_desc = if we_changed_desc { new_desc } else { curr_desc };
    let merged_params = if we_changed_params {
        new_params
    } else {
        curr_params
    };
    let merged_error = if we_changed_error {
        new_error
    } else {
        curr_error
    };
    let merged_checklist = if we_changed_checklist {
        new_checklist
    } else {
        curr_checklist
    };

    // Serialize back with the merged content
    serialize_description_full(&merged_desc, &merged_params, &merged_error, &merged_checklist)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_preserves_grouped_items() {
        let orig_items = vec![
            ChecklistItem {
                id: "main__1__task1".to_string(),
                checked: false,
                text: "work".to_string(),
            },
            ChecklistItem {
                id: "main__1__task2".to_string(),
                checked: true,
                text: "done".to_string(),
            },
            ChecklistItem {
                id: "merge__3__other".to_string(),
                checked: false,
                text: "other pipeline".to_string(),
            },
        ];

        let serialized = serialize_description_full("my task", &HashMap::new(), &None, &orig_items);
        let (desc, _, _, parsed_items) = parse_description_full(&serialized);

        assert_eq!(desc, "my task");
        assert_eq!(parsed_items.len(), 3);

        // Verify IDs match after roundtrip
        assert_eq!(parsed_items[0].id, orig_items[0].id);
        assert_eq!(parsed_items[1].id, orig_items[1].id);
        assert_eq!(parsed_items[2].id, orig_items[2].id);

        // Verify states match
        assert_eq!(parsed_items[0].checked, false);
        assert_eq!(parsed_items[1].checked, true);
        assert_eq!(parsed_items[2].checked, false);
    }

    #[test]
    fn unscoped_checklist_items_are_not_parsed() {
        let legacy = "description\n\n---CHECKLIST---\n\
            - [ ] task1: legacy item\n\
            - [x] task2: old format\n";

        let (desc, _, _, items) = parse_description_full(legacy);

        assert_eq!(desc, "description");
        assert_eq!(items.len(), 0);
    }

    #[test]
    fn roundtrip_preserves_error_section() {
        let mut params = HashMap::new();
        params.insert("key".to_string(), "value".to_string());
        let error = Some("Something went wrong\ndetails here".to_string());
        let items = vec![ChecklistItem {
            id: "main__1__task1".to_string(),
            checked: false,
            text: "work".to_string(),
        }];

        let serialized = serialize_description_full("my task", &params, &error, &items);
        let (desc, parsed_params, parsed_error, parsed_items) =
            parse_description_full(&serialized);

        assert_eq!(desc, "my task");
        assert_eq!(parsed_params.get("key").unwrap(), "value");
        assert_eq!(parsed_error, error);
        assert_eq!(parsed_items.len(), 1);
        assert_eq!(parsed_items[0].id, "main__1__task1");

        // Verify section order in serialized output
        let params_pos = serialized.find("---PARAMETERS---").unwrap();
        let error_pos = serialized.find("---ERROR---").unwrap();
        let checklist_pos = serialized.find("---CHECKLIST---").unwrap();
        assert!(params_pos < error_pos);
        assert!(error_pos < checklist_pos);
    }

    #[test]
    fn roundtrip_no_error_section() {
        let serialized = serialize_description_full("desc", &HashMap::new(), &None, &[]);
        let (desc, _, error, items) = parse_description_full(&serialized);

        assert_eq!(desc, "desc");
        assert_eq!(error, None);
        assert!(items.is_empty());
        assert!(!serialized.contains("---ERROR---"));
    }
}
