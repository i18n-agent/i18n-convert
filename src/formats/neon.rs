use super::*;
use indexmap::IndexMap;

pub struct Parser;
pub struct Writer;

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

/// A raw entry extracted from the NEON file before IR conversion.
#[derive(Debug)]
struct RawEntry {
    key: String,
    value: String,
    comments: Vec<String>,
}

/// Unquote a NEON string value. Handles single-quoted, double-quoted, and unquoted strings.
fn unquote_neon(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.len() >= 2 {
        if (trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
        {
            return trimmed[1..trimmed.len() - 1].to_string();
        }
    }
    trimmed.to_string()
}

/// Parse NEON content into raw entries with dot-separated keys.
fn parse_neon_content(content: &str) -> Result<I18nResource, ParseError> {
    let mut raw_entries: Vec<RawEntry> = Vec::new();
    let mut pending_comments: Vec<String> = Vec::new();

    // Stack to track nesting: (indent_level, key_prefix)
    let mut key_stack: Vec<(usize, String)> = Vec::new();

    // Detect indent unit from first indented line
    let mut indent_unit: Option<usize> = None;
    for line in content.lines() {
        if line.is_empty() || line.trim().is_empty() || line.trim().starts_with('#') {
            continue;
        }
        let leading: usize = line.chars().take_while(|c| *c == ' ' || *c == '\t').count();
        if leading > 0 {
            // Check if it's tabs
            let first_ws = line.chars().next().unwrap_or(' ');
            if first_ws == '\t' {
                indent_unit = Some(1); // tab-based
            } else {
                indent_unit = Some(leading); // space-based, use first indent as unit
            }
            break;
        }
    }
    let indent_size = indent_unit.unwrap_or(1);

    for line in content.lines() {
        let trimmed = line.trim();

        // Blank lines reset pending comments
        if trimmed.is_empty() {
            pending_comments.clear();
            continue;
        }

        // Comment lines
        if trimmed.starts_with('#') {
            let text = &trimmed[1..];
            let text = if text.starts_with(' ') { &text[1..] } else { text };
            pending_comments.push(text.to_string());
            continue;
        }

        // Determine indentation level
        let leading_ws: usize = line.chars().take_while(|c| *c == ' ' || *c == '\t').count();
        let current_level = if leading_ws == 0 {
            0
        } else {
            let first_ws = line.chars().next().unwrap_or(' ');
            if first_ws == '\t' {
                leading_ws // each tab is one level
            } else {
                // space-based indentation
                if indent_size > 0 {
                    leading_ws / indent_size
                } else {
                    0
                }
            }
        };

        // Pop key_stack to match current level
        while let Some(&(level, _)) = key_stack.last() {
            if level >= current_level {
                key_stack.pop();
            } else {
                break;
            }
        }

        // Parse key: value or key: (section header for nesting)
        if let Some(colon_pos) = trimmed.find(':') {
            let key_part = trimmed[..colon_pos].trim();
            let value_part = trimmed[colon_pos + 1..].trim();

            // Build full key from stack
            let prefix = key_stack
                .iter()
                .map(|(_, k)| k.as_str())
                .collect::<Vec<_>>()
                .join(".");
            let full_key = if prefix.is_empty() {
                key_part.to_string()
            } else {
                format!("{}.{}", prefix, key_part)
            };

            if value_part.is_empty() {
                // This is a section/group key — push onto stack for nesting
                key_stack.push((current_level, key_part.to_string()));
                // Comments before a section header annotate the section itself,
                // not the first child — so consume them here.
                pending_comments.clear();
            } else {
                // This is a concrete key-value pair
                let value = unquote_neon(value_part);

                raw_entries.push(RawEntry {
                    key: full_key,
                    value,
                    comments: std::mem::take(&mut pending_comments),
                });
            }
        }
    }

    // Group entries into IR, handling plural suffixes
    let entries = group_entries(raw_entries)?;

    Ok(I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Neon,
            format_ext: Some(FormatExtension::Neon(NeonExt {})),
            ..Default::default()
        },
        entries,
    })
}

/// Group raw entries into IR entries, handling plural suffixes.
fn group_entries(raw_entries: Vec<RawEntry>) -> Result<IndexMap<String, I18nEntry>, ParseError> {
    // First pass: detect plural groups
    let mut plural_bases: IndexMap<String, IndexMap<String, usize>> = IndexMap::new();
    let mut non_plural_indices: Vec<usize> = Vec::new();

    for (i, raw) in raw_entries.iter().enumerate() {
        if let Some((base, category)) = strip_plural_suffix(&raw.key) {
            plural_bases
                .entry(base.to_string())
                .or_default()
                .insert(category.to_string(), i);
        } else {
            non_plural_indices.push(i);
        }
    }

    let mut entries = IndexMap::new();
    let mut consumed: std::collections::HashSet<usize> = std::collections::HashSet::new();

    // Process plural groups (only if they have _other)
    for (base_key, category_map) in &plural_bases {
        if !category_map.contains_key("other") {
            // Not a valid plural group; treat each as a regular entry
            for &idx in category_map.values() {
                non_plural_indices.push(idx);
            }
            continue;
        }

        let mut ps = PluralSet::default();
        let mut comments = Vec::new();

        for (category, &idx) in category_map {
            consumed.insert(idx);
            let raw = &raw_entries[idx];

            match category.as_str() {
                "zero" => ps.zero = Some(raw.value.clone()),
                "one" => ps.one = Some(raw.value.clone()),
                "two" => ps.two = Some(raw.value.clone()),
                "few" => ps.few = Some(raw.value.clone()),
                "many" => ps.many = Some(raw.value.clone()),
                "other" => ps.other = raw.value.clone(),
                _ => {}
            }

            // Collect comments from the first entry
            if comments.is_empty() {
                for text in &raw.comments {
                    if !text.is_empty() {
                        comments.push(Comment {
                            text: text.clone(),
                            role: CommentRole::General,
                            priority: None,
                            annotates: None,
                        });
                    }
                }
            }
        }

        let entry = I18nEntry {
            key: base_key.clone(),
            value: EntryValue::Plural(ps),
            comments,
            ..Default::default()
        };
        entries.insert(base_key.clone(), entry);
    }

    // Process non-plural entries
    non_plural_indices.sort();
    for idx in non_plural_indices {
        if consumed.contains(&idx) {
            continue;
        }
        let raw = &raw_entries[idx];

        let mut entry_comments = Vec::new();
        for text in &raw.comments {
            if !text.is_empty() {
                entry_comments.push(Comment {
                    text: text.clone(),
                    role: CommentRole::General,
                    priority: None,
                    annotates: None,
                });
            }
        }

        let entry = I18nEntry {
            key: raw.key.clone(),
            value: EntryValue::Simple(raw.value.clone()),
            comments: entry_comments,
            ..Default::default()
        };
        entries.insert(raw.key.clone(), entry);
    }

    Ok(entries)
}

// ---------------------------------------------------------------------------
// Writer
// ---------------------------------------------------------------------------

/// Represents a node in the nested key tree for writing.
enum NeonNode {
    Leaf(String, Vec<Comment>), // (value, comments)
    Branch(IndexMap<String, NeonNode>),
}

/// Build a tree from flat dot-separated keys.
fn build_tree(entries: &IndexMap<String, I18nEntry>) -> IndexMap<String, NeonNode> {
    let mut root: IndexMap<String, NeonNode> = IndexMap::new();

    // First, expand plurals back into suffix keys
    let mut flat_entries: Vec<(String, String, Vec<Comment>)> = Vec::new();

    for (_key, entry) in entries {
        match &entry.value {
            EntryValue::Simple(val) => {
                flat_entries.push((entry.key.clone(), val.clone(), entry.comments.clone()));
            }
            EntryValue::Plural(ps) => {
                // Expand plural into individual suffix keys
                let comments = entry.comments.clone();
                if let Some(ref zero) = ps.zero {
                    flat_entries.push((
                        format!("{}_zero", entry.key),
                        zero.clone(),
                        comments.clone(),
                    ));
                }
                if let Some(ref one) = ps.one {
                    flat_entries.push((format!("{}_one", entry.key), one.clone(), Vec::new()));
                }
                if let Some(ref two) = ps.two {
                    flat_entries.push((format!("{}_two", entry.key), two.clone(), Vec::new()));
                }
                if let Some(ref few) = ps.few {
                    flat_entries.push((format!("{}_few", entry.key), few.clone(), Vec::new()));
                }
                if let Some(ref many) = ps.many {
                    flat_entries.push((format!("{}_many", entry.key), many.clone(), Vec::new()));
                }
                flat_entries.push((
                    format!("{}_other", entry.key),
                    ps.other.clone(),
                    Vec::new(),
                ));
            }
            EntryValue::Array(arr) => {
                flat_entries.push((
                    entry.key.clone(),
                    arr.join(", "),
                    entry.comments.clone(),
                ));
            }
            EntryValue::Select(ss) => {
                let val = ss.cases.values().next().cloned().unwrap_or_default();
                flat_entries.push((entry.key.clone(), val, entry.comments.clone()));
            }
            EntryValue::MultiVariablePlural(mvp) => {
                flat_entries.push((
                    entry.key.clone(),
                    mvp.pattern.clone(),
                    entry.comments.clone(),
                ));
            }
        }
    }

    for (full_key, value, comments) in flat_entries {
        let parts: Vec<&str> = full_key.split('.').collect();
        insert_into_tree(&mut root, &parts, value, comments);
    }

    root
}

fn insert_into_tree(
    tree: &mut IndexMap<String, NeonNode>,
    parts: &[&str],
    value: String,
    comments: Vec<Comment>,
) {
    if parts.is_empty() {
        return;
    }

    if parts.len() == 1 {
        tree.insert(
            parts[0].to_string(),
            NeonNode::Leaf(value, comments),
        );
        return;
    }

    let key = parts[0].to_string();
    let sub_tree = tree
        .entry(key)
        .or_insert_with(|| NeonNode::Branch(IndexMap::new()));

    if let NeonNode::Branch(ref mut branch) = sub_tree {
        insert_into_tree(branch, &parts[1..], value, comments);
    }
}

/// Write NEON output from the nested tree.
fn write_neon(entries: &IndexMap<String, I18nEntry>) -> String {
    let tree = build_tree(entries);
    let mut out = String::new();
    write_tree(&tree, 0, &mut out);
    out
}

fn write_tree(tree: &IndexMap<String, NeonNode>, depth: usize, out: &mut String) {
    let indent = "\t".repeat(depth);

    for (key, node) in tree {
        match node {
            NeonNode::Leaf(value, comments) => {
                // Write comments
                for comment in comments {
                    out.push_str(&format!("{}# {}\n", indent, comment.text));
                }
                // Determine if quoting is needed
                let formatted_value = format_neon_value(value);
                out.push_str(&format!("{}{}: {}\n", indent, key, formatted_value));
            }
            NeonNode::Branch(sub_tree) => {
                out.push_str(&format!("{}{}:\n", indent, key));
                write_tree(sub_tree, depth + 1, out);
            }
        }
    }
}

/// Format a NEON value, adding quotes if necessary.
fn format_neon_value(value: &str) -> String {
    // Quote if the value contains special characters that could be ambiguous
    let needs_quoting = value.is_empty()
        || value.contains('#')
        || value.contains(':')
        || value.starts_with('"')
        || value.starts_with('\'')
        || value.starts_with(' ')
        || value.ends_with(' ')
        || value == "true"
        || value == "false"
        || value == "yes"
        || value == "no"
        || value == "null"
        || value.contains('%');

    if needs_quoting {
        // Use double quotes, escaping inner double quotes
        let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{}\"", escaped)
    } else {
        value.to_string()
    }
}

// ---------------------------------------------------------------------------
// Trait implementations
// ---------------------------------------------------------------------------

impl FormatParser for Parser {
    fn detect(&self, extension: &str, _content: &[u8]) -> Confidence {
        if extension == ".neon" {
            Confidence::Definite
        } else {
            Confidence::None
        }
    }

    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError> {
        let text = std::str::from_utf8(content)
            .map_err(|e| ParseError::InvalidFormat(format!("Invalid UTF-8: {}", e)))?;
        parse_neon_content(text)
    }

    fn capabilities(&self) -> FormatCapabilities {
        FormatCapabilities {
            plurals: true,
            arrays: false,
            comments: true,
            context: false,
            source_string: false,
            translatable_flag: false,
            translation_state: false,
            max_width: false,
            device_variants: false,
            select_gender: false,
            nested_keys: true,
            inline_markup: false,
            alternatives: false,
            source_references: false,
            custom_properties: false,
        }
    }
}

impl FormatWriter for Writer {
    fn write(&self, resource: &I18nResource) -> Result<Vec<u8>, WriteError> {
        let output = write_neon(&resource.entries);
        Ok(output.into_bytes())
    }

    fn capabilities(&self) -> FormatCapabilities {
        Parser.capabilities()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_plural_suffix() {
        assert_eq!(strip_plural_suffix("items_one"), Some(("items", "one")));
        assert_eq!(strip_plural_suffix("items_other"), Some(("items", "other")));
        assert_eq!(strip_plural_suffix("items_zero"), Some(("items", "zero")));
        assert_eq!(strip_plural_suffix("items"), None);
        assert_eq!(strip_plural_suffix("_other"), None);
    }

    #[test]
    fn test_unquote_neon() {
        assert_eq!(unquote_neon("hello"), "hello");
        assert_eq!(unquote_neon("\"hello\""), "hello");
        assert_eq!(unquote_neon("'hello'"), "hello");
        assert_eq!(unquote_neon("  hello  "), "hello");
    }

    #[test]
    fn test_parse_basic() {
        let input = "greeting: Hello\nfarewell: Goodbye";
        let resource = parse_neon_content(input).expect("should parse");
        assert_eq!(resource.entries.len(), 2);
        assert!(resource.entries.contains_key("greeting"));
        assert!(resource.entries.contains_key("farewell"));
    }

    #[test]
    fn test_parse_nested() {
        let input = "messages:\n\twelcome: Hello\n\terror:\n\t\tnot_found: 404";
        let resource = parse_neon_content(input).expect("should parse");
        assert_eq!(resource.entries.len(), 2);
        assert!(resource.entries.contains_key("messages.welcome"));
        assert!(resource.entries.contains_key("messages.error.not_found"));
    }

    #[test]
    fn test_parse_comments() {
        let input = "# A greeting\ngreeting: Hello";
        let resource = parse_neon_content(input).expect("should parse");
        let entry = resource.entries.get("greeting").expect("should exist");
        assert_eq!(entry.comments.len(), 1);
        assert_eq!(entry.comments[0].text, "A greeting");
    }

    #[test]
    fn test_parse_plurals() {
        let input = "items_one: 1 item\nitems_other: many items";
        let resource = parse_neon_content(input).expect("should parse");
        assert_eq!(resource.entries.len(), 1);
        let entry = resource.entries.get("items").expect("should exist");
        match &entry.value {
            EntryValue::Plural(ps) => {
                assert_eq!(ps.one, Some("1 item".to_string()));
                assert_eq!(ps.other, "many items");
            }
            other => panic!("Expected plural, got {:?}", other),
        }
    }

    #[test]
    fn test_format_neon_value() {
        assert_eq!(format_neon_value("Hello"), "Hello");
        assert_eq!(format_neon_value("%count% items"), "\"%count% items\"");
        assert_eq!(format_neon_value("true"), "\"true\"");
        assert_eq!(format_neon_value(""), "\"\"");
    }
}
