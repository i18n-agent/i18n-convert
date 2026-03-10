use super::*;
use indexmap::IndexMap;
use regex::Regex;
use std::sync::LazyLock;

pub struct Parser;
pub struct Writer;

/// CLDR plural categories recognized in nested plural mappings (Rails-style)
const PLURAL_KEYS: &[&str] = &["zero", "one", "two", "few", "many", "other"];

/// Locale codes that would indicate a Rails-style YAML file.
/// We check the first non-comment, non-blank YAML key against these patterns.
static RE_LOCALE_ROOT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[a-zA-Z]{2,3}(-[a-zA-Z]{2,4})?:\s*$").expect("valid regex")
});

static RE_PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\{([^}]+)\}").expect("valid regex")
});

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Check if all keys in a YAML mapping are CLDR plural categories
fn is_plural_mapping(mapping: &serde_yaml::Mapping) -> bool {
    if mapping.is_empty() {
        return false;
    }
    let has_other = mapping
        .iter()
        .any(|(k, _)| k.as_str().map_or(false, |s| s == "other"));
    if !has_other {
        return false;
    }
    mapping.iter().all(|(k, v)| {
        if let Some(key_str) = k.as_str() {
            PLURAL_KEYS.contains(&key_str) && is_leaf_value(v)
        } else {
            false
        }
    })
}

/// Check if a YAML value is a leaf (string, number, bool, null)
fn is_leaf_value(value: &serde_yaml::Value) -> bool {
    matches!(
        value,
        serde_yaml::Value::String(_)
            | serde_yaml::Value::Number(_)
            | serde_yaml::Value::Bool(_)
            | serde_yaml::Value::Null
    )
}

/// Convert a serde_yaml::Value to a string
fn value_to_string(value: &serde_yaml::Value) -> String {
    match value {
        serde_yaml::Value::String(s) => s.clone(),
        serde_yaml::Value::Number(n) => n.to_string(),
        serde_yaml::Value::Bool(b) => b.to_string(),
        serde_yaml::Value::Null => String::new(),
        _ => String::new(),
    }
}

/// Extract {name} style placeholders from a string
fn extract_placeholders(text: &str) -> Vec<Placeholder> {
    RE_PLACEHOLDER
        .captures_iter(text)
        .enumerate()
        .map(|(i, cap)| {
            let name = cap
                .get(1)
                .expect("regex group 1 always captures")
                .as_str()
                .to_string();
            let original = cap
                .get(0)
                .expect("regex group 0 always captures")
                .as_str()
                .to_string();
            Placeholder {
                name,
                original_syntax: original,
                placeholder_type: None,
                position: Some(i),
                example: None,
                description: None,
                format: None,
                optional_parameters: None,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Comment extraction
// ---------------------------------------------------------------------------

/// Extract comments from the raw YAML text and associate them with keys.
/// Returns a map from line number of a key to the comment lines preceding it.
fn extract_comments_from_text(text: &str) -> IndexMap<usize, Vec<String>> {
    let mut result = IndexMap::new();
    let mut pending_comments = Vec::new();

    for (line_num, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            let comment_text = trimmed[1..].trim_start().to_string();
            pending_comments.push(comment_text);
        } else if trimmed.is_empty() {
            // Blank lines reset pending comments
            pending_comments.clear();
        } else if !pending_comments.is_empty() {
            // Non-comment, non-blank line: associate pending comments
            result.insert(line_num, std::mem::take(&mut pending_comments));
        }
    }

    result
}

/// Build a map from dot-separated key to comment lines by matching key positions
/// in the YAML text.
fn map_comments_to_keys(text: &str) -> IndexMap<String, Vec<String>> {
    let line_comments = extract_comments_from_text(text);
    let mut key_comments: IndexMap<String, Vec<String>> = IndexMap::new();

    // Build a mapping from line number to the full dot-path key at that line
    let mut current_path: Vec<(usize, String)> = Vec::new(); // (indent_level, key_part)

    for (line_num, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Calculate indentation
        let indent = line.len() - line.trim_start().len();

        // Extract the key from this line
        if let Some(colon_pos) = trimmed.find(':') {
            let key_part = trimmed[..colon_pos].trim().to_string();
            if key_part.is_empty() {
                continue;
            }

            // Pop path components that are at the same or deeper indentation
            while let Some(&(prev_indent, _)) = current_path.last() {
                if prev_indent >= indent {
                    current_path.pop();
                } else {
                    break;
                }
            }

            current_path.push((indent, key_part));

            // Build the full key path
            let full_key: String = current_path
                .iter()
                .map(|(_, k)| k.as_str())
                .collect::<Vec<_>>()
                .join(".");

            // Check if this line has associated comments
            if let Some(comments) = line_comments.get(&line_num) {
                key_comments.insert(full_key, comments.clone());
            }
        }
    }

    key_comments
}

// ---------------------------------------------------------------------------
// Flatten YAML to IR entries
// ---------------------------------------------------------------------------

/// Recursively flatten a YAML mapping into IR entries.
fn flatten_yaml(
    mapping: &serde_yaml::Mapping,
    prefix: &str,
    entries: &mut IndexMap<String, I18nEntry>,
    key_comments: &IndexMap<String, Vec<String>>,
) {
    for (key, value) in mapping {
        let key_str = match key.as_str() {
            Some(s) => s,
            None => continue,
        };

        let full_key = if prefix.is_empty() {
            key_str.to_string()
        } else {
            format!("{}.{}", prefix, key_str)
        };

        match value {
            serde_yaml::Value::Mapping(child_map) => {
                if is_plural_mapping(child_map) {
                    // Plural group (nested style)
                    let mut plural_set = PluralSet::default();
                    let mut all_placeholders = Vec::new();

                    for (pk, pv) in child_map {
                        if let Some(pk_str) = pk.as_str() {
                            let val = value_to_string(pv);
                            let mut phs = extract_placeholders(&val);
                            all_placeholders.append(&mut phs);
                            match pk_str {
                                "zero" => plural_set.zero = Some(val),
                                "one" => plural_set.one = Some(val),
                                "two" => plural_set.two = Some(val),
                                "few" => plural_set.few = Some(val),
                                "many" => plural_set.many = Some(val),
                                "other" => plural_set.other = val,
                                _ => {}
                            }
                        }
                    }

                    // Deduplicate placeholders
                    let mut seen = std::collections::HashSet::new();
                    all_placeholders.retain(|p| seen.insert(p.name.clone()));

                    let comments = build_comments_for_key(&full_key, key_comments);

                    entries.insert(
                        full_key.clone(),
                        I18nEntry {
                            key: full_key,
                            value: EntryValue::Plural(plural_set),
                            placeholders: all_placeholders,
                            comments,
                            ..Default::default()
                        },
                    );
                } else {
                    // Recurse into nested mapping
                    flatten_yaml(child_map, &full_key, entries, key_comments);
                }
            }
            serde_yaml::Value::Sequence(seq) => {
                let items: Vec<String> = seq.iter().map(value_to_string).collect();
                let comments = build_comments_for_key(&full_key, key_comments);
                entries.insert(
                    full_key.clone(),
                    I18nEntry {
                        key: full_key,
                        value: EntryValue::Array(items),
                        comments,
                        ..Default::default()
                    },
                );
            }
            _ => {
                let text = value_to_string(value);
                let placeholders = extract_placeholders(&text);
                let comments = build_comments_for_key(&full_key, key_comments);
                entries.insert(
                    full_key.clone(),
                    I18nEntry {
                        key: full_key,
                        value: EntryValue::Simple(text),
                        placeholders,
                        comments,
                        ..Default::default()
                    },
                );
            }
        }
    }
}

fn build_comments_for_key(
    key: &str,
    key_comments: &IndexMap<String, Vec<String>>,
) -> Vec<Comment> {
    key_comments
        .get(key)
        .map(|texts| {
            texts
                .iter()
                .filter(|t| !t.is_empty())
                .map(|t| Comment {
                    text: t.clone(),
                    role: CommentRole::General,
                    priority: None,
                    annotates: None,
                })
                .collect()
        })
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Group flat plural-suffix keys into PluralSet entries
// ---------------------------------------------------------------------------

/// After flattening, group keys like `items_one`, `items_other` into a single
/// PluralSet entry keyed as `items`.
fn group_plural_suffix_keys(
    entries: IndexMap<String, I18nEntry>,
) -> IndexMap<String, I18nEntry> {
    // First pass: identify plural groups
    let mut plural_bases: IndexMap<String, IndexMap<String, usize>> = IndexMap::new();
    let keys: Vec<String> = entries.keys().cloned().collect();

    for (i, key) in keys.iter().enumerate() {
        if let Some((base, category)) = strip_plural_suffix(key) {
            plural_bases
                .entry(base.to_string())
                .or_default()
                .insert(category.to_string(), i);
        }
    }

    // Second pass: build the result, skipping consumed plural keys
    let mut consumed = std::collections::HashSet::new();
    let mut result = IndexMap::new();

    for (base_key, category_map) in &plural_bases {
        if !category_map.contains_key("other") {
            // Not a valid plural group; leave individual keys as-is
            continue;
        }

        let mut ps = PluralSet::default();
        let mut all_placeholders = Vec::new();
        let mut first_comments = Vec::new();
        let mut first = true;

        for (category, &idx) in category_map {
            let key = &keys[idx];
            consumed.insert(key.clone());

            if let Some(entry) = entries.get(key) {
                let val = match &entry.value {
                    EntryValue::Simple(s) => s.clone(),
                    _ => continue,
                };

                let mut phs = extract_placeholders(&val);
                all_placeholders.append(&mut phs);

                if first && !entry.comments.is_empty() {
                    first_comments = entry.comments.clone();
                    first = false;
                }

                match category.as_str() {
                    "zero" => ps.zero = Some(val),
                    "one" => ps.one = Some(val),
                    "two" => ps.two = Some(val),
                    "few" => ps.few = Some(val),
                    "many" => ps.many = Some(val),
                    "other" => ps.other = val,
                    _ => {}
                }
            }
        }

        // Deduplicate placeholders
        let mut seen = std::collections::HashSet::new();
        all_placeholders.retain(|p| seen.insert(p.name.clone()));

        result.insert(
            base_key.clone(),
            I18nEntry {
                key: base_key.clone(),
                value: EntryValue::Plural(ps),
                placeholders: all_placeholders,
                comments: first_comments,
                ..Default::default()
            },
        );
    }

    // Add non-plural entries in original order
    for (key, entry) in &entries {
        if !consumed.contains(key) {
            result.insert(key.clone(), entry.clone());
        }
    }

    result
}

// ---------------------------------------------------------------------------
// Unflatten IR entries back to nested YAML
// ---------------------------------------------------------------------------

/// Reconstruct nested YAML from flat dot-separated IR entries.
fn unflatten_to_yaml(entries: &IndexMap<String, I18nEntry>) -> serde_yaml::Mapping {
    let mut root = serde_yaml::Mapping::new();

    for entry in entries.values() {
        match &entry.value {
            EntryValue::Plural(ps) => {
                // For plural entries, we write using suffix keys (items_one, items_other)
                let parts: Vec<&str> = entry.key.split('.').collect();
                if let Some((&last, parent_parts)) = parts.split_last() {
                    let write_suffix =
                        |mapping: &mut serde_yaml::Mapping, suffix: &str, val: &str| {
                            let suffixed_key = format!("{}_{}", last, suffix);
                            let key = serde_yaml::Value::String(suffixed_key);
                            mapping.insert(key, serde_yaml::Value::String(val.to_string()));
                        };

                    // Navigate to the parent mapping
                    let parent = get_or_create_parent(&mut root, parent_parts);

                    if let Some(ref zero) = ps.zero {
                        write_suffix(parent, "zero", zero);
                    }
                    if let Some(ref one) = ps.one {
                        write_suffix(parent, "one", one);
                    }
                    if let Some(ref two) = ps.two {
                        write_suffix(parent, "two", two);
                    }
                    if let Some(ref few) = ps.few {
                        write_suffix(parent, "few", few);
                    }
                    if let Some(ref many) = ps.many {
                        write_suffix(parent, "many", many);
                    }
                    write_suffix(parent, "other", &ps.other);
                }
            }
            _ => {
                let parts: Vec<&str> = entry.key.split('.').collect();
                insert_nested(&mut root, &parts, &entry.value);
            }
        }
    }

    root
}

/// Navigate to (and create if necessary) the parent mapping for a given path.
fn get_or_create_parent<'a>(
    root: &'a mut serde_yaml::Mapping,
    parts: &[&str],
) -> &'a mut serde_yaml::Mapping {
    let mut current = root;
    for &part in parts {
        let key = serde_yaml::Value::String(part.to_string());
        let child = current
            .entry(key)
            .or_insert_with(|| serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
        current = match child {
            serde_yaml::Value::Mapping(ref mut m) => m,
            _ => panic!("Expected mapping at key {}", part),
        };
    }
    current
}

/// Insert a value into a nested YAML mapping at the given key path.
fn insert_nested(
    mapping: &mut serde_yaml::Mapping,
    parts: &[&str],
    value: &EntryValue,
) {
    if parts.is_empty() {
        return;
    }

    let key = serde_yaml::Value::String(parts[0].to_string());

    if parts.len() == 1 {
        let yaml_value = entry_value_to_yaml(value);
        mapping.insert(key, yaml_value);
    } else {
        let child = mapping
            .entry(key)
            .or_insert_with(|| serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));

        if let serde_yaml::Value::Mapping(ref mut child_map) = child {
            insert_nested(child_map, &parts[1..], value);
        }
    }
}

/// Convert an IR EntryValue to a serde_yaml::Value
fn entry_value_to_yaml(value: &EntryValue) -> serde_yaml::Value {
    match value {
        EntryValue::Simple(s) => serde_yaml::Value::String(s.clone()),
        EntryValue::Plural(plural_set) => {
            // For plain YAML, plurals are written as suffix keys
            // This path should not normally be reached (handled in unflatten_to_yaml),
            // but provide a fallback using nested mapping style
            let mut m = serde_yaml::Mapping::new();
            if let Some(ref zero) = plural_set.zero {
                m.insert(
                    serde_yaml::Value::String("zero".to_string()),
                    serde_yaml::Value::String(zero.clone()),
                );
            }
            if let Some(ref one) = plural_set.one {
                m.insert(
                    serde_yaml::Value::String("one".to_string()),
                    serde_yaml::Value::String(one.clone()),
                );
            }
            if let Some(ref two) = plural_set.two {
                m.insert(
                    serde_yaml::Value::String("two".to_string()),
                    serde_yaml::Value::String(two.clone()),
                );
            }
            if let Some(ref few) = plural_set.few {
                m.insert(
                    serde_yaml::Value::String("few".to_string()),
                    serde_yaml::Value::String(few.clone()),
                );
            }
            if let Some(ref many) = plural_set.many {
                m.insert(
                    serde_yaml::Value::String("many".to_string()),
                    serde_yaml::Value::String(many.clone()),
                );
            }
            m.insert(
                serde_yaml::Value::String("other".to_string()),
                serde_yaml::Value::String(plural_set.other.clone()),
            );
            serde_yaml::Value::Mapping(m)
        }
        EntryValue::Array(items) => {
            let seq: Vec<serde_yaml::Value> = items
                .iter()
                .map(|s| serde_yaml::Value::String(s.clone()))
                .collect();
            serde_yaml::Value::Sequence(seq)
        }
        EntryValue::Select(_) | EntryValue::MultiVariablePlural(_) => {
            serde_yaml::Value::String("[unsupported]".to_string())
        }
    }
}

// ---------------------------------------------------------------------------
// Trait implementations
// ---------------------------------------------------------------------------

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        if extension != ".yml" && extension != ".yaml" {
            return Confidence::None;
        }

        // Distinguish from Rails YAML by checking if the first non-comment,
        // non-blank line looks like a locale root key (e.g., "en:", "ja:", "zh-Hans:")
        if let Ok(text) = std::str::from_utf8(content) {
            let first_line = text
                .lines()
                .find(|l| {
                    let t = l.trim();
                    !t.is_empty() && !t.starts_with('#')
                });

            if let Some(line) = first_line {
                let trimmed = line.trim();
                if RE_LOCALE_ROOT.is_match(trimmed) {
                    // Looks like a Rails-style locale root key
                    return Confidence::Low;
                }
            }
        }

        Confidence::High
    }

    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError> {
        let text = std::str::from_utf8(content)
            .map_err(|e| ParseError::InvalidFormat(format!("Invalid UTF-8: {}", e)))?;

        // Handle empty content
        if text.trim().is_empty() {
            return Ok(I18nResource {
                metadata: ResourceMetadata {
                    source_format: FormatId::YamlPlain,
                    format_ext: Some(FormatExtension::YamlPlain(YamlPlainExt {})),
                    ..Default::default()
                },
                entries: IndexMap::new(),
            });
        }

        let yaml_value: serde_yaml::Value = serde_yaml::from_str(text)
            .map_err(|e| ParseError::Yaml(format!("{}", e)))?;

        let root_mapping = yaml_value
            .as_mapping()
            .ok_or_else(|| ParseError::Yaml("Expected a YAML mapping at root".to_string()))?;

        // Extract comments from raw text
        let key_comments = map_comments_to_keys(text);

        let mut entries = IndexMap::new();
        flatten_yaml(root_mapping, "", &mut entries, &key_comments);

        // Group plural suffix keys (items_one, items_other) into PluralSet entries
        let entries = group_plural_suffix_keys(entries);

        Ok(I18nResource {
            metadata: ResourceMetadata {
                source_format: FormatId::YamlPlain,
                format_ext: Some(FormatExtension::YamlPlain(YamlPlainExt {})),
                ..Default::default()
            },
            entries,
        })
    }

    fn capabilities(&self) -> FormatCapabilities {
        FormatCapabilities {
            plurals: true,
            arrays: true,
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
        let inner_mapping = unflatten_to_yaml(&resource.entries);

        let yaml_str = serde_yaml::to_string(&serde_yaml::Value::Mapping(inner_mapping))
            .map_err(|e| WriteError::Serialization(format!("{}", e)))?;

        Ok(yaml_str.into_bytes())
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
    use crate::formats::{FormatParser, FormatWriter};

    #[test]
    fn test_detect_yml_plain() {
        let parser = Parser;
        let content = b"greeting: Hello\nfarewell: Goodbye\n";
        assert_eq!(parser.detect(".yml", content), Confidence::High);
    }

    #[test]
    fn test_detect_yaml_plain() {
        let parser = Parser;
        let content = b"greeting: Hello\n";
        assert_eq!(parser.detect(".yaml", content), Confidence::High);
    }

    #[test]
    fn test_detect_rails_like_returns_low() {
        let parser = Parser;
        let content = b"en:\n  greeting: Hello\n";
        assert_eq!(parser.detect(".yml", content), Confidence::Low);
    }

    #[test]
    fn test_detect_non_yaml() {
        let parser = Parser;
        let content = b"greeting: Hello\n";
        assert_eq!(parser.detect(".json", content), Confidence::None);
    }

    #[test]
    fn test_parse_simple() {
        let parser = Parser;
        let content = b"greeting: Hello\nfarewell: Goodbye\n";
        let resource = parser.parse(content).unwrap();

        assert_eq!(resource.metadata.source_format, FormatId::YamlPlain);
        assert_eq!(resource.entries.len(), 2);
        assert_eq!(
            resource.entries["greeting"].value,
            EntryValue::Simple("Hello".to_string())
        );
        assert_eq!(
            resource.entries["farewell"].value,
            EntryValue::Simple("Goodbye".to_string())
        );
    }

    #[test]
    fn test_parse_nested() {
        let parser = Parser;
        let content = b"messages:\n  welcome: Welcome\n  error:\n    not_found: Not found\n";
        let resource = parser.parse(content).unwrap();

        assert_eq!(resource.entries.len(), 2);
        assert_eq!(
            resource.entries["messages.welcome"].value,
            EntryValue::Simple("Welcome".to_string())
        );
        assert_eq!(
            resource.entries["messages.error.not_found"].value,
            EntryValue::Simple("Not found".to_string())
        );
    }

    #[test]
    fn test_parse_plural_suffix_keys() {
        let parser = Parser;
        let content = b"items_one: \"{count} item\"\nitems_other: \"{count} items\"\n";
        let resource = parser.parse(content).unwrap();

        assert_eq!(resource.entries.len(), 1);
        match &resource.entries["items"].value {
            EntryValue::Plural(ps) => {
                assert_eq!(ps.one, Some("{count} item".to_string()));
                assert_eq!(ps.other, "{count} items");
            }
            other => panic!("Expected Plural, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_array() {
        let parser = Parser;
        let content = b"colors:\n  - red\n  - green\n  - blue\n";
        let resource = parser.parse(content).unwrap();

        assert_eq!(resource.entries.len(), 1);
        assert_eq!(
            resource.entries["colors"].value,
            EntryValue::Array(vec![
                "red".to_string(),
                "green".to_string(),
                "blue".to_string()
            ])
        );
    }

    #[test]
    fn test_roundtrip_simple() {
        let parser = Parser;
        let writer = Writer;
        let content = b"greeting: Hello\nfarewell: Goodbye\n";
        let resource = parser.parse(content).unwrap();
        let output = writer.write(&resource).unwrap();
        let resource2 = parser.parse(&output).unwrap();

        assert_eq!(resource.entries.len(), resource2.entries.len());
        for (key, entry) in &resource.entries {
            assert_eq!(entry.value, resource2.entries[key].value);
        }
    }

    #[test]
    fn test_strip_plural_suffix() {
        assert_eq!(strip_plural_suffix("items_one"), Some(("items", "one")));
        assert_eq!(strip_plural_suffix("items_other"), Some(("items", "other")));
        assert_eq!(strip_plural_suffix("items"), None);
        assert_eq!(strip_plural_suffix("_other"), None);
    }

    #[test]
    fn test_extract_placeholders() {
        let phs = extract_placeholders("Hello, {name}! You have {count} items.");
        assert_eq!(phs.len(), 2);
        assert_eq!(phs[0].name, "name");
        assert_eq!(phs[1].name, "count");
    }
}
