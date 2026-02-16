use std::collections::HashMap;

use zbobr_dispatcher::ChecklistItem;

// -- Plan and Checklist parsing and serialization helpers --

const PARAMETERS_SEPARATOR: &str = "\n\n---PARAMETERS---\n";
const PLAN_SEPARATOR: &str = "\n\n---PLAN---\n";
const CHECKLIST_SEPARATOR: &str = "\n\n---CHECKLIST---\n";

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

/// Parse a task description into (description, parameters, plan, checklist).
/// Format: description | ---PARAMETERS--- | params | ---PLAN--- | plan text | ---CHECKLIST--- | checklist
pub(crate) fn parse_description_full(full_text: &str) -> (String, HashMap<String, String>, String, Vec<ChecklistItem>) {
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

    // Now split by plan separator
    let plan_parts: Vec<&str> = before_checklist.split(PLAN_SEPARATOR).collect();
    let (before_plan, plan) = match plan_parts.len() {
        1 => (plan_parts[0], ""),
        _ => (plan_parts[0], plan_parts[1].trim()),
    };

    // Now split by parameters separator
    let param_parts: Vec<&str> = before_plan.split(PARAMETERS_SEPARATOR).collect();
    let (description, params_text) = match param_parts.len() {
        1 => (param_parts[0].to_string(), ""),
        _ => (param_parts[0].to_string(), param_parts[1].trim()),
    };

    // Parse parameters
    let parameters = parse_parameters(params_text);

    // Parse checklist items
    let mut items = Vec::new();
    for line in checklist_text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Parse checkbox format: - [ ] id: text or - [x] id: text
        if let Some(rest) = line.strip_prefix("- [")
            && let Some(pos) = rest.find(']')
        {
            let checkbox = &rest[..pos];
            let checked = checkbox.trim() == "x" || checkbox.trim() == "X";

            let after_checkbox = rest[pos + 1..].trim();
            if let Some(colon_pos) = after_checkbox.find(':') {
                let id = after_checkbox[..colon_pos].trim().to_string();
                let text = after_checkbox[colon_pos + 1..].trim().to_string();

                items.push(ChecklistItem { id, checked, text });
            }
        }
    }

    (description, parameters, plan.to_string(), items)
}

/// Parse a task description into (description, plan, checklist) - backward compatibility.
/// Format: description | ---PLAN--- | plan text | ---CHECKLIST--- | checklist
pub(crate) fn parse_description_with_plan_and_checklist(full_text: &str) -> (String, String, Vec<ChecklistItem>) {
    let (description, _parameters, plan, items) = parse_description_full(full_text);
    (description, plan, items)
}

/// Parse a task description, separating the original description from the checklist.
/// Returns (original_description, checklist_items).
/// This function now also strips the plan section automatically.
pub(crate) fn parse_description_with_checklist(description: &str) -> (String, Vec<ChecklistItem>) {
    let (desc, _, items) = parse_description_with_plan_and_checklist(description);
    (desc, items)
}

/// Extract the original description, removing any existing plan and checklist sections.
/// This ensures no duplicate separators in the description.
pub(crate) fn strip_plan_and_checklist_from_description(description: &str) -> String {
    let (original, _, _) = parse_description_with_plan_and_checklist(description);
    original
}

/// Extract the original description, removing any existing checklist section.
/// This ensures no duplicate checklist separators in the description.
pub(crate) fn strip_checklist_from_description(description: &str) -> String {
    let (original, _) = parse_description_with_checklist(description);
    original
}

/// Extract the plan from a full description text.
/// Returns an empty string if no plan section exists.
pub(crate) fn extract_plan(full_text: &str) -> String {
    let (_, _, plan, _) = parse_description_full(full_text);
    plan
}

/// Extract parameters from a full description text.
pub(crate) fn extract_parameters(full_text: &str) -> HashMap<String, String> {
    let (_, params, _, _) = parse_description_full(full_text);
    params
}

/// Serialize description, parameters, plan and checklist items back into the full format.
/// Format: description | ---PARAMETERS--- | params | ---PLAN--- | plan | ---CHECKLIST--- | checklist
pub(crate) fn serialize_description_full(
    original_description: &str,
    parameters: &HashMap<String, String>,
    plan: &str,
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

    // Add plan if present
    if !plan.is_empty() {
        result.push_str(PLAN_SEPARATOR);
        result.push_str(plan);
    }

    // Add checklist if present
    if !items.is_empty() {
        result.push_str(CHECKLIST_SEPARATOR);
        for item in items {
            let checkbox = if item.checked { "x" } else { " " };
            result.push_str(&format!("- [{}] {}: {}\n", checkbox, item.id, item.text));
        }
    }

    result
}

/// Serialize description, plan and checklist items back into the full format.
/// Format: description | ---PARAMETERS--- | params | ---PLAN--- | plan | ---CHECKLIST--- | checklist
/// Preserves any existing parameters.
pub(crate) fn serialize_description_with_plan_and_checklist(
    original_description: &str,
    plan: &str,
    items: &[ChecklistItem],
) -> String {
    // Preserve existing parameters
    let parameters = extract_parameters(original_description);
    serialize_description_full(original_description, &parameters, plan, items)
}

/// Serialize checklist items back into the full description format.
/// If the description contains an existing checklist, it will be replaced with the new one.
pub(crate) fn serialize_description_with_checklist(original_description: &str, items: &[ChecklistItem]) -> String {
    // Keep any existing plan, only replace checklist
    let current_plan = extract_plan(original_description);
    serialize_description_with_plan_and_checklist(original_description, &current_plan, items)
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
    let (orig_desc, orig_params, orig_plan, orig_checklist) = parse_description_full(original);
    let (curr_desc, curr_params, curr_plan, curr_checklist) = parse_description_full(current);
    let (new_desc, new_params, new_plan, new_checklist) = parse_description_full(our_new);

    // Determine what we changed
    let we_changed_desc = new_desc != orig_desc;
    let we_changed_params = new_params != orig_params;
    let we_changed_plan = new_plan != orig_plan;
    let we_changed_checklist = serde_json::to_string(&new_checklist).unwrap_or_default()
        != serde_json::to_string(&orig_checklist).unwrap_or_default();

    // Merge: prefer our changes if we made them, otherwise prefer their changes
    let merged_desc = if we_changed_desc { new_desc } else { curr_desc };
    let merged_params = if we_changed_params { new_params } else { curr_params };
    let merged_plan = if we_changed_plan { new_plan } else { curr_plan };
    let merged_checklist = if we_changed_checklist { new_checklist } else { curr_checklist };

    // Serialize back with the merged content
    serialize_description_full(
        &merged_desc,
        &merged_params,
        &merged_plan,
        &merged_checklist,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checklist_parsing_and_serialization() {
        // Test with no checklist
        let desc = "This is a task description";
        let (original, items) = parse_description_with_checklist(desc);
        assert_eq!(original, desc);
        assert!(items.is_empty());

        // Test with checklist (separators use \n\n before marker)
        let desc_with_checklist = "Task description\n\n---CHECKLIST---\n- [ ] item1: First item\n- [x] item2: Second item checked\n- [ ] item3: Third item\n";
        let (original, items) = parse_description_with_checklist(desc_with_checklist);
        assert_eq!(original, "Task description");
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].id, "item1");
        assert_eq!(items[0].text, "First item");
        assert!(!items[0].checked);
        assert_eq!(items[1].id, "item2");
        assert_eq!(items[1].text, "Second item checked");
        assert!(items[1].checked);
        assert_eq!(items[2].id, "item3");
        assert_eq!(items[2].text, "Third item");
        assert!(!items[2].checked);

        // Test serialization
        let serialized = serialize_description_with_checklist(&original, &items);
        assert!(serialized.contains("Task description"));
        assert!(serialized.contains("---CHECKLIST---"));
        assert!(serialized.contains("- [ ] item1: First item"));
        assert!(serialized.contains("- [x] item2: Second item checked"));
        assert!(serialized.contains("- [ ] item3: Third item"));

        // Test round-trip
        let (original2, items2) = parse_description_with_checklist(&serialized);
        assert_eq!(original, original2);
        assert_eq!(items.len(), items2.len());
        for (item1, item2) in items.iter().zip(items2.iter()) {
            assert_eq!(item1.id, item2.id);
            assert_eq!(item1.text, item2.text);
            assert_eq!(item1.checked, item2.checked);
        }
    }

    #[test]
    fn test_description_checklist_validation() {
        // Test stripping existing checklist from description
        let desc_with_old_checklist = "Task description\n\n---CHECKLIST---\n- [ ] old1: Old item\n";
        let stripped = strip_checklist_from_description(desc_with_old_checklist);
        assert_eq!(stripped, "Task description");

        // Test that serialize_description_with_checklist replaces old checklist with new one
        let new_checklist = vec![
            ChecklistItem { id: "new1".to_string(), checked: false, text: "New item".to_string() },
        ];
        let serialized = serialize_description_with_checklist(desc_with_old_checklist, &new_checklist);

        // Should contain the new checklist, not the old one
        assert!(serialized.contains("- [ ] new1: New item"));
        assert!(!serialized.contains("old1"));

        // Should parse correctly
        let (original, items) = parse_description_with_checklist(&serialized);
        assert_eq!(original, "Task description");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "new1");
        assert_eq!(items[0].text, "New item");
    }

    #[test]
    fn test_plan_parsing_and_serialization() {
        // Test with no plan or checklist
        let desc = "This is a task description";
        let (original, plan, items) = parse_description_with_plan_and_checklist(desc);
        assert_eq!(original, desc);
        assert_eq!(plan, "");
        assert!(items.is_empty());

        // Test with plan and checklist (separators use \n\n before marker)
        let full_text = "Task description\n\n---PLAN---\nImplementation plan here\n\n---CHECKLIST---\n- [ ] item1: First item\n- [x] item2: Done item\n";
        let (original, plan, items) = parse_description_with_plan_and_checklist(full_text);
        assert_eq!(original, "Task description");
        assert_eq!(plan, "Implementation plan here");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].id, "item1");
        assert!(!items[0].checked);
        assert_eq!(items[1].id, "item2");
        assert!(items[1].checked);

        // Test parsing with CRLF line endings
        let full_text_crlf = "Task description\r\n\r\n---PLAN---\r\nImplementation plan here\r\n\r\n---CHECKLIST---\r\n- [ ] item1: First item\r\n- [x] item2: Done item\r\n";
        let (original_crlf, plan_crlf, items_crlf) = parse_description_with_plan_and_checklist(full_text_crlf);
        assert_eq!(original_crlf, "Task description");
        assert_eq!(plan_crlf, "Implementation plan here");
        assert_eq!(items_crlf.len(), 2);

        // Test extract_plan function
        let extracted_plan = extract_plan(full_text);
        assert_eq!(extracted_plan, "Implementation plan here");

        // Test serialization
        let serialized = serialize_description_with_plan_and_checklist(&original, &plan, &items);
        assert!(serialized.contains("Task description"));
        assert!(serialized.contains("---PLAN---"));
        assert!(serialized.contains("Implementation plan here"));
        assert!(serialized.contains("---CHECKLIST---"));
        assert!(serialized.contains("- [ ] item1: First item"));
        assert!(serialized.contains("- [x] item2: Done item"));

        // Test round-trip
        let (original2, plan2, items2) = parse_description_with_plan_and_checklist(&serialized);
        assert_eq!(original, original2);
        assert_eq!(plan, plan2);
        assert_eq!(items.len(), items2.len());
        for (item1, item2) in items.iter().zip(items2.iter()) {
            assert_eq!(item1.id, item2.id);
            assert_eq!(item1.text, item2.text);
            assert_eq!(item1.checked, item2.checked);
        }
    }

    #[test]
    fn test_plan_replacement() {
        // Test replacing an existing plan
        let old_full = "Task description\n\n---PLAN---\nOld plan\n\n---CHECKLIST---\n- [ ] item1: Item\n";
        let new_plan = "New implementation plan";
        let (desc, _, items) = parse_description_with_plan_and_checklist(old_full);

        let serialized = serialize_description_with_plan_and_checklist(&desc, new_plan, &items);
        assert!(serialized.contains("New implementation plan"));
        assert!(!serialized.contains("Old plan"));
        assert!(serialized.contains("- [ ] item1: Item"));

        // Verify it parses correctly
        let (parsed_desc, parsed_plan, parsed_items) = parse_description_with_plan_and_checklist(&serialized);
        assert_eq!(parsed_desc, "Task description");
        assert_eq!(parsed_plan, "New implementation plan");
        assert_eq!(parsed_items.len(), 1);
    }
}
