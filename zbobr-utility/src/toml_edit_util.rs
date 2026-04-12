use toml_edit::{Array, DocumentMut, Item, Table, Value};

/// Convert every named child table in `table` into an inline table.
pub fn inline_named_children_as_inline_tables(table: &mut Table) {
    let keys: Vec<String> = table.iter().map(|(k, _)| k.to_string()).collect();
    for key in &keys {
        if let Some(item) = table.get_mut(key) {
            inline_item_as_inline_table(item);
        }
        if let Some(mut k) = table.key_mut(key) {
            k.fmt();
        }
    }
}

/// Convert a single TOML item to an inline table if it is a table.
pub fn inline_item_as_inline_table(item: &mut Item) {
    if let Some(table) = item.as_table_mut() {
        let inline = table.clone().into_inline_table();
        *item = Item::Value(Value::InlineTable(inline));
    }
}

/// Convert the named child table of `parent` to an inline table.
pub fn inline_child_table(parent: &mut Table, child_key: &str) {
    if let Some(item) = parent.get_mut(child_key) {
        inline_item_as_inline_table(item);
    }
    if let Some(mut key) = parent.key_mut(child_key) {
        key.fmt();
    }
}

/// Convert every named child array-of-tables in `table` to an inline array.
pub fn inline_named_children_as_inline_table_arrays(table: &mut Table) {
    let keys: Vec<String> = table.iter().map(|(k, _)| k.to_string()).collect();
    for key in &keys {
        if let Some(item) = table.get_mut(key) {
            inline_item_as_inline_table_array(item);
        }
        if let Some(mut k) = table.key_mut(key) {
            k.fmt();
        }
    }
}

/// Set a named child table to dotted mode for a parent table.
pub fn set_child_table_dotted(doc: &mut DocumentMut, parent_table_name: &str, child_key: &str) {
    if let Some(Item::Table(parent_table)) = doc.get_mut(parent_table_name)
        && let Some(Item::Table(child_table)) = parent_table.get_mut(child_key)
    {
        child_table.set_dotted(true);
    }
}

/// Convert a single TOML item to an inline array of tables if it is an array of tables.
pub fn inline_item_as_inline_table_array(item: &mut Item) {
    if let Item::ArrayOfTables(aot) = item {
        let mut array = Array::new();
        for table in aot.iter() {
            array.push(Value::InlineTable(table.clone().into_inline_table()));
        }
        *item = Item::Value(Value::Array(array));
    }
}

#[cfg(test)]
mod tests {
    use toml_edit::DocumentMut;

    use super::*;

    #[test]
    fn inline_named_children_as_inline_tables_converts_child_tables() {
        let mut doc: DocumentMut = r#"
[parent]
[ parent.child ]
a = 1
[ parent.other ]
b = 2
"#
        .parse()
        .unwrap();
        let parent = doc["parent"].as_table_mut().unwrap();
        inline_named_children_as_inline_tables(parent);
        let output = doc.to_string();
        assert!(output.contains("child = { a = 1 }"));
        assert!(output.contains("other = { b = 2 }"));
    }

    #[test]
    fn inline_item_as_inline_table_array_converts_array_of_tables() {
        let mut doc: DocumentMut = r#"
[[tools.developer]]
provider = "claude"
model = "claude-sonnet-4-6"

[[tools.developer]]
provider = "copilot"
model = "gpt-5.4"
"#
        .parse()
        .unwrap();
        let tools = doc["tools"].as_table_mut().unwrap();
        inline_named_children_as_inline_table_arrays(tools);
        let output = doc.to_string();
        assert!(output.contains("developer = ["));
        assert!(output.contains("{ provider = \"claude\""));
        assert!((output.contains("{ provider = \"copilot\"")));
    }

    #[test]
    fn inline_child_table_formats_key_and_value_correctly() {
        let mut doc: DocumentMut = r#"
[role]
[role.prompts]
main = "planner.md"
"#
        .parse()
        .unwrap();
        let role = doc["role"].as_table_mut().unwrap();
        inline_child_table(role, "prompts");
        let output = doc.to_string();
        assert!(output.contains("prompts = { main = \"planner.md\" }"));
        assert!(!output.contains("prompts= {"));
    }

    #[test]
    fn set_child_table_dotted_converts_nested_tables_to_dotted_keys() {
        let mut doc: DocumentMut = r#"
[repo]
[repo.github_token]
value = ""
[tasks]
[tasks.github_token]
value = ""
"#
        .parse()
        .unwrap();
        set_child_table_dotted(&mut doc, "repo", "github_token");
        set_child_table_dotted(&mut doc, "tasks", "github_token");
        let output = doc.to_string();
        assert!(output.contains("[repo]"));
        assert!(output.contains("github_token.value = \"\""));
        assert!(output.contains("tasks.github_token.value = \"\""));
        assert!(!output.contains("[repo.github_token]"));
        assert!(!output.contains("[tasks.github_token]"));
    }
}
