use crate::ir::*;
use super::*;

pub struct Parser;
pub struct Writer;

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        if extension == ".toml" {
            if let Ok(s) = std::str::from_utf8(content) {
                // Try to parse as TOML — if it succeeds, high confidence
                if toml::from_str::<toml::Value>(s).is_ok() {
                    return Confidence::High;
                }
            }
            return Confidence::Low;
        }
        Confidence::None
    }

    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError> {
        let s = std::str::from_utf8(content)
            .map_err(|e| ParseError::InvalidFormat(format!("Invalid UTF-8: {e}")))?;

        let root: toml::Value = toml::from_str(s)
            .map_err(|e| ParseError::InvalidFormat(format!("TOML parse error: {e}")))?;

        let table = root.as_table().ok_or_else(|| {
            ParseError::InvalidFormat("Root must be a TOML table".to_string())
        })?;

        let mut entries = IndexMap::new();
        flatten_toml_table(table, &mut String::new(), &mut entries);

        Ok(I18nResource {
            metadata: ResourceMetadata {
                source_format: FormatId::Toml,
                format_ext: Some(FormatExtension::Toml(TomlExt {
                    table_path: None,
                })),
                ..Default::default()
            },
            entries,
        })
    }

    fn capabilities(&self) -> FormatCapabilities {
        FormatCapabilities {
            plurals: false,
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

/// Recursively flatten a TOML table into dot-separated keys.
fn flatten_toml_table(
    table: &toml::map::Map<String, toml::Value>,
    prefix: &mut String,
    entries: &mut IndexMap<String, I18nEntry>,
) {
    for (key, value) in table {
        let full_key = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };

        match value {
            toml::Value::Table(nested) => {
                flatten_toml_table(nested, &mut full_key.clone(), entries);
            }
            toml::Value::String(s) => {
                entries.insert(
                    full_key.clone(),
                    I18nEntry {
                        key: full_key,
                        value: EntryValue::Simple(s.clone()),
                        ..Default::default()
                    },
                );
            }
            toml::Value::Integer(n) => {
                let s = n.to_string();
                entries.insert(
                    full_key.clone(),
                    I18nEntry {
                        key: full_key,
                        value: EntryValue::Simple(s),
                        ..Default::default()
                    },
                );
            }
            toml::Value::Float(f) => {
                let s = f.to_string();
                entries.insert(
                    full_key.clone(),
                    I18nEntry {
                        key: full_key,
                        value: EntryValue::Simple(s),
                        ..Default::default()
                    },
                );
            }
            toml::Value::Boolean(b) => {
                let s = b.to_string();
                entries.insert(
                    full_key.clone(),
                    I18nEntry {
                        key: full_key,
                        value: EntryValue::Simple(s),
                        ..Default::default()
                    },
                );
            }
            toml::Value::Array(arr) => {
                // Check if all elements are strings
                let string_items: Vec<String> = arr
                    .iter()
                    .filter_map(|v| match v {
                        toml::Value::String(s) => Some(s.clone()),
                        toml::Value::Integer(n) => Some(n.to_string()),
                        toml::Value::Float(f) => Some(f.to_string()),
                        toml::Value::Boolean(b) => Some(b.to_string()),
                        _ => None,
                    })
                    .collect();

                if string_items.len() == arr.len() {
                    entries.insert(
                        full_key.clone(),
                        I18nEntry {
                            key: full_key,
                            value: EntryValue::Array(string_items),
                            ..Default::default()
                        },
                    );
                }
                // Skip arrays with non-scalar elements
            }
            toml::Value::Datetime(dt) => {
                let s = dt.to_string();
                entries.insert(
                    full_key.clone(),
                    I18nEntry {
                        key: full_key,
                        value: EntryValue::Simple(s),
                        ..Default::default()
                    },
                );
            }
        }
    }
}

impl FormatWriter for Writer {
    fn write(&self, resource: &I18nResource) -> Result<Vec<u8>, WriteError> {
        let mut root = toml::map::Map::new();

        for entry in resource.entries.values() {
            let toml_value = match &entry.value {
                EntryValue::Simple(s) => toml::Value::String(s.clone()),
                EntryValue::Plural(ps) => toml::Value::String(ps.other.clone()),
                EntryValue::Array(items) => {
                    toml::Value::Array(items.iter().map(|s| toml::Value::String(s.clone())).collect())
                }
                EntryValue::Select(ss) => {
                    toml::Value::String(ss.cases.get("other").cloned().unwrap_or_default())
                }
                EntryValue::MultiVariablePlural(mvp) => {
                    toml::Value::String(mvp.pattern.clone())
                }
            };

            insert_nested_toml(&mut root, &entry.key, toml_value);
        }

        let toml_str = toml::to_string_pretty(&toml::Value::Table(root))
            .map_err(|e| WriteError::Serialization(format!("{e}")))?;

        let mut output = toml_str.into_bytes();
        if !output.ends_with(b"\n") {
            output.push(b'\n');
        }
        Ok(output)
    }

    fn capabilities(&self) -> FormatCapabilities {
        Parser.capabilities()
    }
}

/// Insert a value into a nested TOML table using a dot-separated key path.
fn insert_nested_toml(
    root: &mut toml::map::Map<String, toml::Value>,
    key: &str,
    value: toml::Value,
) {
    let parts: Vec<&str> = key.split('.').collect();
    if parts.len() == 1 {
        root.insert(key.to_string(), value);
        return;
    }

    let mut current = root;
    for part in &parts[..parts.len() - 1] {
        current = current
            .entry(part.to_string())
            .or_insert_with(|| toml::Value::Table(toml::map::Map::new()))
            .as_table_mut()
            .expect("Expected nested table in key path");
    }

    let leaf_key = parts[parts.len() - 1];
    current.insert(leaf_key.to_string(), value);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::{FormatParser, FormatWriter};

    #[test]
    fn test_detect_toml_extension() {
        let parser = Parser;
        let content = b"[messages]\ngreeting = \"Hello\"\n";
        assert_eq!(parser.detect(".toml", content), Confidence::High);
    }

    #[test]
    fn test_detect_non_toml_extension() {
        let parser = Parser;
        let content = b"[messages]\ngreeting = \"Hello\"\n";
        assert_eq!(parser.detect(".json", content), Confidence::None);
    }

    #[test]
    fn test_parse_flat() {
        let parser = Parser;
        let content = b"greeting = \"Hello\"\nfarewell = \"Goodbye\"\n";
        let resource = parser.parse(content).expect("parse should succeed");

        assert_eq!(resource.metadata.source_format, FormatId::Toml);
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
    fn test_parse_nested_tables() {
        let parser = Parser;
        let content = b"[messages]\ngreeting = \"Hello\"\nfarewell = \"Goodbye\"\n\n[errors]\nnot_found = \"Not found\"\n";
        let resource = parser.parse(content).expect("parse should succeed");

        assert_eq!(resource.entries.len(), 3);
        assert_eq!(
            resource.entries["messages.greeting"].value,
            EntryValue::Simple("Hello".to_string())
        );
        assert_eq!(
            resource.entries["errors.not_found"].value,
            EntryValue::Simple("Not found".to_string())
        );
    }

    #[test]
    fn test_parse_array() {
        let parser = Parser;
        let content = b"colors = [\"red\", \"green\", \"blue\"]\n";
        let resource = parser.parse(content).expect("parse should succeed");

        assert_eq!(resource.entries.len(), 1);
        assert_eq!(
            resource.entries["colors"].value,
            EntryValue::Array(vec![
                "red".to_string(),
                "green".to_string(),
                "blue".to_string(),
            ])
        );
    }

    #[test]
    fn test_roundtrip_simple() {
        let parser = Parser;
        let writer = Writer;
        let content = b"greeting = \"Hello\"\nfarewell = \"Goodbye\"\n";
        let resource = parser.parse(content).expect("parse should succeed");
        let output = writer.write(&resource).expect("write should succeed");
        let reparsed = parser.parse(&output).expect("reparse should succeed");

        assert_eq!(resource.entries.len(), reparsed.entries.len());
        for (key, entry) in &resource.entries {
            assert_eq!(entry.value, reparsed.entries[key].value, "Mismatch for key: {key}");
        }
    }

    #[test]
    fn test_write_nested() {
        let writer = Writer;
        let mut entries = IndexMap::new();
        entries.insert(
            "messages.greeting".to_string(),
            I18nEntry {
                key: "messages.greeting".to_string(),
                value: EntryValue::Simple("Hello".to_string()),
                ..Default::default()
            },
        );

        let resource = I18nResource {
            metadata: ResourceMetadata {
                source_format: FormatId::Toml,
                ..Default::default()
            },
            entries,
        };

        let output = writer.write(&resource).expect("write should succeed");
        let output_str = std::str::from_utf8(&output).expect("valid UTF-8");

        let parsed: toml::Value = toml::from_str(output_str).expect("valid TOML");
        assert_eq!(
            parsed["messages"]["greeting"].as_str(),
            Some("Hello")
        );
    }

    #[test]
    fn test_capabilities() {
        let caps = Parser.capabilities();
        assert!(caps.nested_keys);
        assert!(caps.arrays);
        assert!(!caps.plurals);
        assert!(!caps.comments);
    }
}
