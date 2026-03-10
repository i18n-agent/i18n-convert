use super::*;
use regex::Regex;
use std::sync::LazyLock;

pub struct Parser;
pub struct Writer;

/// CLDR plural categories recognized by Rails i18n
const PLURAL_KEYS: &[&str] = &["zero", "one", "two", "few", "many", "other"];

/// Check if all keys in a YAML mapping are plural categories
fn is_plural_mapping(mapping: &serde_yaml::Mapping) -> bool {
    if mapping.is_empty() {
        return false;
    }
    // Must have "other" key and all keys must be plural categories
    let has_other = mapping.iter().any(|(k, _)| {
        k.as_str().map_or(false, |s| s == "other")
    });
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

/// Check if a YAML value is a leaf (string, number, bool, null) — not a mapping or sequence
fn is_leaf_value(value: &serde_yaml::Value) -> bool {
    matches!(
        value,
        serde_yaml::Value::String(_)
            | serde_yaml::Value::Number(_)
            | serde_yaml::Value::Bool(_)
            | serde_yaml::Value::Null
    )
}

/// Convert a serde_yaml::Value to a string representation
fn value_to_string(value: &serde_yaml::Value) -> String {
    match value {
        serde_yaml::Value::String(s) => s.clone(),
        serde_yaml::Value::Number(n) => n.to_string(),
        serde_yaml::Value::Bool(b) => b.to_string(),
        serde_yaml::Value::Null => String::new(),
        _ => String::new(),
    }
}

static RE_YAML_PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"%\{([^}]+)\}").expect("valid regex pattern")
});

/// Extract %{name} style placeholders from a string
fn extract_placeholders(text: &str) -> Vec<Placeholder> {
    RE_YAML_PLACEHOLDER.captures_iter(text)
        .enumerate()
        .map(|(i, cap)| {
            let name = cap.get(1).expect("regex group 1 always captures").as_str().to_string();
            let original = cap.get(0).expect("regex group 0 always captures").as_str().to_string();
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

/// Recursively flatten a YAML mapping into IR entries.
/// `prefix` is the dot-separated key prefix (excluding the locale root).
fn flatten_yaml(
    mapping: &serde_yaml::Mapping,
    prefix: &str,
    entries: &mut IndexMap<String, I18nEntry>,
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
                    // This is a plural group — create a PluralSet entry
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

                    // Deduplicate placeholders by name
                    let mut seen = std::collections::HashSet::new();
                    all_placeholders.retain(|p| seen.insert(p.name.clone()));

                    entries.insert(full_key.clone(), I18nEntry {
                        key: full_key,
                        value: EntryValue::Plural(plural_set),
                        placeholders: all_placeholders,
                        ..Default::default()
                    });
                } else {
                    // Recurse into nested mapping
                    flatten_yaml(child_map, &full_key, entries);
                }
            }
            serde_yaml::Value::Sequence(seq) => {
                // Convert YAML arrays to EntryValue::Array
                let items: Vec<String> = seq.iter().map(value_to_string).collect();
                entries.insert(full_key.clone(), I18nEntry {
                    key: full_key,
                    value: EntryValue::Array(items),
                    ..Default::default()
                });
            }
            _ => {
                // Leaf value → Simple entry
                let text = value_to_string(value);
                let placeholders = extract_placeholders(&text);
                entries.insert(full_key.clone(), I18nEntry {
                    key: full_key,
                    value: EntryValue::Simple(text),
                    placeholders,
                    ..Default::default()
                });
            }
        }
    }
}

/// Reconstruct nested YAML structure from flat dot-separated IR entries.
/// Returns a serde_yaml::Mapping.
fn unflatten_to_yaml(entries: &IndexMap<String, I18nEntry>) -> serde_yaml::Mapping {
    let mut root = serde_yaml::Mapping::new();

    for entry in entries.values() {
        let parts: Vec<&str> = entry.key.split('.').collect();
        insert_nested(&mut root, &parts, &entry.value);
    }

    root
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
        // Leaf — insert the value
        let yaml_value = entry_value_to_yaml(value);
        mapping.insert(key, yaml_value);
    } else {
        // Intermediate — ensure a mapping exists and recurse
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
            // These types don't have a natural YAML Rails representation;
            // fall back to a placeholder string
            serde_yaml::Value::String("[unsupported]".to_string())
        }
    }
}

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        if extension == ".yml" || extension == ".yaml" {
            if let Ok(s) = std::str::from_utf8(content) {
                // Rails convention: top-level key is a locale code like "en:", "ja:", "de:", etc.
                let trimmed = s.trim_start();
                if trimmed.starts_with("en:")
                    || trimmed.starts_with("ja:")
                    || trimmed.starts_with("de:")
                    || trimmed.starts_with("fr:")
                    || trimmed.starts_with("es:")
                    || trimmed.starts_with("zh:")
                    || trimmed.starts_with("ko:")
                    || trimmed.starts_with("pt:")
                    || trimmed.starts_with("it:")
                    || trimmed.starts_with("ru:")
                {
                    return Confidence::High;
                }
            }
            return Confidence::Low;
        }
        Confidence::None
    }

    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError> {
        let text = std::str::from_utf8(content)
            .map_err(|e| ParseError::InvalidFormat(format!("Invalid UTF-8: {}", e)))?;

        let yaml_value: serde_yaml::Value = serde_yaml::from_str(text)
            .map_err(|e| ParseError::Yaml(format!("{}", e)))?;

        let root_mapping = yaml_value
            .as_mapping()
            .ok_or_else(|| ParseError::Yaml("Expected a YAML mapping at root".to_string()))?;

        // Rails convention: exactly one top-level key which is the locale code
        if root_mapping.len() != 1 {
            return Err(ParseError::Yaml(format!(
                "Expected exactly one top-level locale key, found {}",
                root_mapping.len()
            )));
        }

        let (locale_key, locale_value) = root_mapping.iter().next().expect("len == 1 checked above");
        let locale = locale_key
            .as_str()
            .ok_or_else(|| ParseError::Yaml("Locale key must be a string".to_string()))?
            .to_string();

        let translations_mapping = locale_value
            .as_mapping()
            .ok_or_else(|| ParseError::Yaml("Locale value must be a mapping".to_string()))?;

        let mut entries = IndexMap::new();
        flatten_yaml(translations_mapping, "", &mut entries);

        Ok(I18nResource {
            metadata: ResourceMetadata {
                source_format: FormatId::YamlRails,
                locale: Some(locale),
                format_ext: Some(FormatExtension::YamlRails(YamlRailsExt {})),
                ..Default::default()
            },
            entries,
        })
    }

    fn capabilities(&self) -> FormatCapabilities {
        FormatCapabilities {
            plurals: true,
            arrays: true,
            comments: false,
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
        let locale = resource
            .metadata
            .locale
            .as_deref()
            .unwrap_or("en");

        let inner_mapping = unflatten_to_yaml(&resource.entries);

        // Wrap in locale root key
        let mut root = serde_yaml::Mapping::new();
        root.insert(
            serde_yaml::Value::String(locale.to_string()),
            serde_yaml::Value::Mapping(inner_mapping),
        );

        let yaml_str = serde_yaml::to_string(&serde_yaml::Value::Mapping(root))
            .map_err(|e| WriteError::Serialization(format!("{}", e)))?;

        Ok(yaml_str.into_bytes())
    }

    fn capabilities(&self) -> FormatCapabilities {
        Parser.capabilities()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::{FormatParser, FormatWriter};

    #[test]
    fn test_detect_yml_with_locale() {
        let parser = Parser;
        let content = b"en:\n  greeting: Hello\n";
        assert_eq!(parser.detect(".yml", content), Confidence::High);
    }

    #[test]
    fn test_detect_yaml_with_locale() {
        let parser = Parser;
        let content = b"ja:\n  greeting: Hello\n";
        assert_eq!(parser.detect(".yaml", content), Confidence::High);
    }

    #[test]
    fn test_detect_yml_without_locale() {
        let parser = Parser;
        let content = b"some_key:\n  value: test\n";
        assert_eq!(parser.detect(".yml", content), Confidence::Low);
    }

    #[test]
    fn test_detect_non_yaml() {
        let parser = Parser;
        let content = b"en:\n  greeting: Hello\n";
        assert_eq!(parser.detect(".json", content), Confidence::None);
    }

    #[test]
    fn test_parse_simple() {
        let parser = Parser;
        let content = b"en:\n  greeting: Hello\n  farewell: Goodbye\n";
        let resource = parser.parse(content).unwrap();

        assert_eq!(resource.metadata.locale, Some("en".to_string()));
        assert_eq!(resource.metadata.source_format, FormatId::YamlRails);
        assert_eq!(resource.entries.len(), 2);

        let greeting = &resource.entries["greeting"];
        assert_eq!(greeting.value, EntryValue::Simple("Hello".to_string()));

        let farewell = &resource.entries["farewell"];
        assert_eq!(farewell.value, EntryValue::Simple("Goodbye".to_string()));
    }

    #[test]
    fn test_parse_nested() {
        let parser = Parser;
        let content = b"en:\n  common:\n    greeting: Hello\n    farewell: Goodbye\n";
        let resource = parser.parse(content).unwrap();

        assert_eq!(resource.entries.len(), 2);
        assert_eq!(
            resource.entries["common.greeting"].value,
            EntryValue::Simple("Hello".to_string())
        );
        assert_eq!(
            resource.entries["common.farewell"].value,
            EntryValue::Simple("Goodbye".to_string())
        );
    }

    #[test]
    fn test_parse_plurals() {
        let parser = Parser;
        let content = b"en:\n  items:\n    one: one item\n    other: \"%{count} items\"\n";
        let resource = parser.parse(content).unwrap();

        assert_eq!(resource.entries.len(), 1);
        match &resource.entries["items"].value {
            EntryValue::Plural(ps) => {
                assert_eq!(ps.one, Some("one item".to_string()));
                assert_eq!(ps.other, "%{count} items".to_string());
            }
            other => panic!("Expected Plural, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_interpolation() {
        let parser = Parser;
        let content = b"en:\n  greeting: \"Hello, %{name}!\"\n";
        let resource = parser.parse(content).unwrap();

        let entry = &resource.entries["greeting"];
        assert_eq!(entry.placeholders.len(), 1);
        assert_eq!(entry.placeholders[0].name, "name");
        assert_eq!(entry.placeholders[0].original_syntax, "%{name}");
    }

    #[test]
    fn test_write_simple() {
        let writer = Writer;
        let mut entries = IndexMap::new();
        entries.insert("greeting".to_string(), I18nEntry {
            key: "greeting".to_string(),
            value: EntryValue::Simple("Hello".to_string()),
            ..Default::default()
        });

        let resource = I18nResource {
            metadata: ResourceMetadata {
                source_format: FormatId::YamlRails,
                locale: Some("en".to_string()),
                ..Default::default()
            },
            entries,
        };

        let output = writer.write(&resource).unwrap();
        let text = std::str::from_utf8(&output).unwrap();
        assert!(text.contains("en:"));
        assert!(text.contains("greeting: Hello"));
    }

    #[test]
    fn test_roundtrip_simple() {
        let parser = Parser;
        let writer = Writer;
        let input = b"en:\n  greeting: Hello\n  farewell: Goodbye\n";
        let resource = parser.parse(input).unwrap();
        let output = writer.write(&resource).unwrap();
        let resource2 = parser.parse(&output).unwrap();

        assert_eq!(resource.metadata.locale, resource2.metadata.locale);
        assert_eq!(resource.entries.len(), resource2.entries.len());
        for (key, entry) in &resource.entries {
            assert_eq!(entry.value, resource2.entries[key].value);
        }
    }

    #[test]
    fn test_is_plural_mapping() {
        // Valid plural mapping
        let mut m = serde_yaml::Mapping::new();
        m.insert(
            serde_yaml::Value::String("one".to_string()),
            serde_yaml::Value::String("one item".to_string()),
        );
        m.insert(
            serde_yaml::Value::String("other".to_string()),
            serde_yaml::Value::String("%{count} items".to_string()),
        );
        assert!(is_plural_mapping(&m));

        // Missing "other" — not plural
        let mut m2 = serde_yaml::Mapping::new();
        m2.insert(
            serde_yaml::Value::String("one".to_string()),
            serde_yaml::Value::String("one item".to_string()),
        );
        assert!(!is_plural_mapping(&m2));

        // Non-plural keys — not plural
        let mut m3 = serde_yaml::Mapping::new();
        m3.insert(
            serde_yaml::Value::String("title".to_string()),
            serde_yaml::Value::String("Title".to_string()),
        );
        m3.insert(
            serde_yaml::Value::String("other".to_string()),
            serde_yaml::Value::String("other".to_string()),
        );
        assert!(!is_plural_mapping(&m3));
    }

    #[test]
    fn test_plural_with_nested_mapping_not_detected() {
        // If "other" value is a mapping, it should not be detected as plural
        let mut m = serde_yaml::Mapping::new();
        m.insert(
            serde_yaml::Value::String("one".to_string()),
            serde_yaml::Value::String("one item".to_string()),
        );
        let mut nested = serde_yaml::Mapping::new();
        nested.insert(
            serde_yaml::Value::String("key".to_string()),
            serde_yaml::Value::String("val".to_string()),
        );
        m.insert(
            serde_yaml::Value::String("other".to_string()),
            serde_yaml::Value::Mapping(nested),
        );
        assert!(!is_plural_mapping(&m));
    }

    #[test]
    fn test_extract_placeholders() {
        let phs = extract_placeholders("Hello, %{name}! You have %{count} items.");
        assert_eq!(phs.len(), 2);
        assert_eq!(phs[0].name, "name");
        assert_eq!(phs[1].name, "count");
    }

    #[test]
    fn test_extract_no_placeholders() {
        let phs = extract_placeholders("Hello, world!");
        assert!(phs.is_empty());
    }
}
