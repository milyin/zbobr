use crate::task::ChecklistItem;

/// Serialize a list of checklist items into grouped format with run headers.
/// Items are grouped by pipeline and run ID (extracted from scoped IDs).
/// Scoped ID format: pipeline__run_id__item_id
/// 
/// Display format:
/// ```text
/// <!-- Run: pipeline #run_id -->
/// - [ ] item_id: text
/// ```
pub fn serialize_grouped_checklist(items: &[ChecklistItem]) -> String {
    if items.is_empty() {
        return String::new();
    }

    let mut result = String::new();

    // Group items by pipeline and run ID (extracted from scoped IDs)
    let mut run_groups: Vec<(String, u64, Vec<&ChecklistItem>)> = Vec::new();

    for item in items {
        let Some((pipeline, run_id, _)) = extract_run_scope(&item.id) else {
            continue;
        };

        if let Some(group) = run_groups.iter_mut().find(|(p, r, _)| p == &pipeline && *r == run_id) {
            group.2.push(item);
        } else {
            run_groups.push((pipeline, run_id, vec![item]));
        }
    }

    // Serialize groups with run headers
    for (pipeline, run_id, group_items) in run_groups {
        result.push_str(&format!("<!-- Run: {} #{} -->\n", pipeline, run_id));
        for item in group_items {
            let checkbox = if item.checked { "x" } else { " " };
            // Strip the scope prefix for display (pipeline__run_id__item_id -> item_id)
            let display_id = strip_run_scope(&item.id);
            result.push_str(&format!("- [{}] {}: {}\n", checkbox, display_id, item.text));
        }
        result.push('\n');
    }

    result
}

/// Parse grouped checklist text back into ChecklistItem list with scoped IDs.
/// 
/// Input format (with run headers):
/// ```text
/// <!-- Run: pipeline #run_id -->
/// - [ ] item_id: text
/// ```
/// 
/// Output: ChecklistItem with id = "pipeline__run_id__item_id"
pub fn parse_grouped_checklist(text: &str) -> Vec<ChecklistItem> {
    let mut items = Vec::new();
    let mut current_run_prefix: Option<String> = None;

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Check for run header: <!-- Run: pipeline #run_id -->
        if let Some(run_markers) = line.strip_prefix("<!-- Run: ")
            && let Some(end) = run_markers.find(" -->")
        {
            let run_info = &run_markers[..end];
            // Format: "pipeline #run_id"
            if let Some(hash_pos) = run_info.find('#') {
                let pipeline = run_info[..hash_pos].trim();
                if let Ok(run_id) = run_info[hash_pos + 1..].trim().parse::<u64>() {
                    if !pipeline.is_empty() && run_id > 0 {
                        current_run_prefix = Some(format!("{}__{}__", pipeline, run_id));
                        continue;
                    }
                }
            }
            current_run_prefix = None;
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
                let id_suffix = after_checkbox[..colon_pos].trim();
                let text = after_checkbox[colon_pos + 1..].trim().to_string();

                // Only accept checklist items inside an explicit run section.
                let Some(ref prefix) = current_run_prefix else {
                    continue;
                };
                if id_suffix.is_empty() {
                    continue;
                }

                let id = format!("{}{}", prefix, id_suffix);

                items.push(ChecklistItem { id, checked, text });
            }
        }
    }

    items
}

/// Extract pipeline, run ID, and item suffix from a scoped checklist item ID.
/// Format: pipeline__run_id__item_id
fn extract_run_scope(scoped_id: &str) -> Option<(String, u64, String)> {
    let mut parts = scoped_id.splitn(3, "__");
    let pipeline = parts.next()?.trim();
    let run_id_str = parts.next()?.trim();
    let item_suffix = parts.next()?.trim();
    if pipeline.is_empty() || item_suffix.is_empty() {
        return None;
    }
    let run_id = run_id_str.parse::<u64>().ok()?;
    if run_id == 0 {
        return None;
    }
    Some((
        pipeline.to_string(),
        run_id,
        item_suffix.to_string(),
    ))
}

/// Strip the scope prefix from a scoped checklist item ID.
/// Format: pipeline__run_id__item_id -> item_id
fn strip_run_scope(scoped_id: &str) -> String {
    extract_run_scope(scoped_id)
        .map(|(_, _, suffix)| suffix)
        .unwrap_or_else(|| scoped_id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_groups_by_run() {
        let items = vec![
            ChecklistItem {
                id: "main__1__task1".to_string(),
                checked: false,
                text: "first run".to_string(),
            },
            ChecklistItem {
                id: "main__1__task2".to_string(),
                checked: true,
                text: "done".to_string(),
            },
            ChecklistItem {
                id: "main__2__task1".to_string(),
                checked: false,
                text: "second run".to_string(),
            },
        ];

        let serialized = serialize_grouped_checklist(&items);

        assert!(serialized.contains("<!-- Run: main #1 -->"));
        assert!(serialized.contains("<!-- Run: main #2 -->"));
        assert!(serialized.contains("- [ ] task1: first run"));
        assert!(serialized.contains("- [x] task2: done"));
        assert!(serialized.contains("- [ ] task1: second run"));

        // Display IDs should not contain scope
        assert!(!serialized.contains("main__1__"));
        assert!(!serialized.contains("main__2__"));
    }

    #[test]
    fn parse_restores_scoped_ids() {
        let text = "<!-- Run: main #1 -->\n\
            - [ ] task1: first\n\
            - [x] task2: done\n\
            \n\
            <!-- Run: main #2 -->\n\
            - [ ] task1: second\n";

        let items = parse_grouped_checklist(text);

        assert_eq!(items.len(), 3);
        assert_eq!(items[0].id, "main__1__task1");
        assert_eq!(items[0].text, "first");
        assert!(!items[0].checked);

        assert_eq!(items[1].id, "main__1__task2");
        assert!(items[1].checked);

        assert_eq!(items[2].id, "main__2__task1");
        assert_eq!(items[2].text, "second");
    }

    #[test]
    fn roundtrip_preserves_data() {
        let orig = vec![
            ChecklistItem {
                id: "main__1__a".to_string(),
                checked: false,
                text: "task a".to_string(),
            },
            ChecklistItem {
                id: "main__1__b".to_string(),
                checked: true,
                text: "task b".to_string(),
            },
            ChecklistItem {
                id: "init__2__c".to_string(),
                checked: false,
                text: "task c".to_string(),
            },
        ];

        let serialized = serialize_grouped_checklist(&orig);
        let parsed = parse_grouped_checklist(&serialized);

        assert_eq!(parsed.len(), orig.len());
        for (orig_item, parsed_item) in orig.iter().zip(parsed.iter()) {
            assert_eq!(parsed_item.id, orig_item.id);
            assert_eq!(parsed_item.checked, orig_item.checked);
            assert_eq!(parsed_item.text, orig_item.text);
        }
    }

    #[test]
    fn unscoped_items_are_not_serialized() {
        let items = vec![
            ChecklistItem {
                id: "simple".to_string(),
                checked: false,
                text: "legacy".to_string(),
            },
        ];

        let serialized = serialize_grouped_checklist(&items);
        assert_eq!(serialized, "");
    }

    #[test]
    fn unscoped_items_are_not_parsed() {
        let text = "- [ ] task1: legacy\n\
            - [x] task2: old\n";

        let items = parse_grouped_checklist(text);

        assert_eq!(items.len(), 0);
    }

    #[test]
    fn empty_checklist() {
        let items: Vec<ChecklistItem> = vec![];
        let serialized = serialize_grouped_checklist(&items);
        assert_eq!(serialized, "");

        let parsed = parse_grouped_checklist("");
        assert_eq!(parsed.len(), 0);
    }
}
