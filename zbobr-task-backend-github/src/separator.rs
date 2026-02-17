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
pub(crate) fn parse_description_full(
    full_text: &str,
) -> (String, HashMap<String, String>, String, Vec<ChecklistItem>) {
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
    let merged_params = if we_changed_params {
        new_params
    } else {
        curr_params
    };
    let merged_plan = if we_changed_plan { new_plan } else { curr_plan };
    let merged_checklist = if we_changed_checklist {
        new_checklist
    } else {
        curr_checklist
    };

    // Serialize back with the merged content
    serialize_description_full(
        &merged_desc,
        &merged_params,
        &merged_plan,
        &merged_checklist,
    )
}
