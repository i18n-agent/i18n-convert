use crate::ir::*;
use super::*;

pub struct Parser;
pub struct Writer;

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        if extension == ".json5" {
            if let Ok(s) = std::str::from_utf8(content) {
                // Try to parse as JSON5 — if it succeeds, definite match
                if json5::from_str::<serde_json::Value>(s).is_ok() {
                    return Confidence::Definite;
                }
            }
            return Confidence::Low;
        }
        Confidence::None
    }

    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError> {
        let s = std::str::from_utf8(content)
            .map_err(|e| ParseError::InvalidFormat(format!("Invalid UTF-8: {e}")))?;

        let root: serde_json::Value = json5::from_str(s)
            .map_err(|e| ParseError::Json(format!("JSON5 parse error: {e}")))?;

        let obj = root.as_object().ok_or_else(|| {
            ParseError::InvalidFormat("Root must be a JSON5 object".to_string())
        })?;

        let mut entries = IndexMap::new();
        flatten_json5_object(obj, &mut String::new(), &mut entries);

        Ok(I18nResource {
            metadata: ResourceMetadata {
                source_format: FormatId::Json5,
                format_ext: Some(FormatExtension::Json5(Json5Ext {
                    trailing_commas: None,
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

/// Recursively flatten a JSON object (parsed from JSON5) into dot-separated keys.
fn flatten_json5_object(
    obj: &serde_json::Map<String, serde_json::Value>,
    prefix: &mut String,
    entries: &mut IndexMap<String, I18nEntry>,
) {
    for (key, value) in obj {
        let full_key = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };

        match value {
            serde_json::Value::Object(nested) => {
                flatten_json5_object(nested, &mut full_key.clone(), entries);
            }
            serde_json::Value::String(s) => {
                entries.insert(
                    full_key.clone(),
                    I18nEntry {
                        key: full_key,
                        value: EntryValue::Simple(s.clone()),
                        ..Default::default()
                    },
                );
            }
            serde_json::Value::Number(n) => {
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
            serde_json::Value::Bool(b) => {
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
            serde_json::Value::Array(arr) => {
                // Convert arrays of scalar values to Array entries
                let string_items: Vec<String> = arr
                    .iter()
                    .filter_map(|v| match v {
                        serde_json::Value::String(s) => Some(s.clone()),
                        serde_json::Value::Number(n) => Some(n.to_string()),
                        serde_json::Value::Bool(b) => Some(b.to_string()),
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
            serde_json::Value::Null => {
                // Skip null values
            }
        }
    }
}

impl FormatWriter for Writer {
    fn write(&self, resource: &I18nResource) -> Result<Vec<u8>, WriteError> {
        // Since the json5 crate has no serializer, write as standard JSON
        // (JSON is a valid subset of JSON5)
        let mut root = serde_json::Map::new();

        for entry in resource.entries.values() {
            let json_value = match &entry.value {
                EntryValue::Simple(s) => serde_json::Value::String(s.clone()),
                EntryValue::Plural(ps) => serde_json::Value::String(ps.other.clone()),
                EntryValue::Array(items) => {
                    serde_json::Value::Array(
                        items.iter().map(|s| serde_json::Value::String(s.clone())).collect(),
                    )
                }
                EntryValue::Select(ss) => {
                    serde_json::Value::String(
                        ss.cases.get("other").cloned().unwrap_or_default(),
                    )
                }
                EntryValue::MultiVariablePlural(mvp) => {
                    serde_json::Value::String(mvp.pattern.clone())
                }
            };

            insert_nested_json5(&mut root, &entry.key, json_value);
        }

        let json = serde_json::to_string_pretty(&serde_json::Value::Object(root))
            .map_err(|e| WriteError::Serialization(format!("{e}")))?;

        let mut output = json.into_bytes();
        if !output.ends_with(b"\n") {
            output.push(b'\n');
        }
        Ok(output)
    }

    fn capabilities(&self) -> FormatCapabilities {
        Parser.capabilities()
    }
}

/// Insert a value into a nested JSON map using a dot-separated key path.
fn insert_nested_json5(
    root: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    value: serde_json::Value,
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
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()))
            .as_object_mut()
            .expect("Expected nested object in key path");
    }

    let leaf_key = parts[parts.len() - 1];
    current.insert(leaf_key.to_string(), value);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::{FormatParser, FormatWriter};

    #[test]
    fn test_detect_json5_extension() {
        let parser = Parser;
        let content = b"{ greeting: 'Hello' }";
        assert_eq!(parser.detect(".json5", content), Confidence::Definite);
    }

    #[test]
    fn test_detect_non_json5_extension() {
        let parser = Parser;
        let content = b"{ greeting: 'Hello' }";
        assert_eq!(parser.detect(".json", content), Confidence::None);
    }

    #[test]
    fn test_parse_simple() {
        let parser = Parser;
        let content = b"{ greeting: 'Hello', farewell: \"Goodbye\" }";
        let resource = parser.parse(content).expect("parse should succeed");

        assert_eq!(resource.metadata.source_format, FormatId::Json5);
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
        let content = b"{ nested: { welcome: 'Welcome' }, top: 'Top' }";
        let resource = parser.parse(content).expect("parse should succeed");

        assert_eq!(resource.entries.len(), 2);
        assert_eq!(
            resource.entries["nested.welcome"].value,
            EntryValue::Simple("Welcome".to_string())
        );
        assert_eq!(
            resource.entries["top"].value,
            EntryValue::Simple("Top".to_string())
        );
    }

    #[test]
    fn test_parse_with_comments() {
        let parser = Parser;
        let content = b"{\n  // A comment\n  greeting: 'Hello',\n  /* block */\n  farewell: 'Bye',\n}";
        let resource = parser.parse(content).expect("parse should succeed");

        // Comments are stripped by the json5 crate but parsing should still succeed
        assert_eq!(resource.entries.len(), 2);
        assert_eq!(
            resource.entries["greeting"].value,
            EntryValue::Simple("Hello".to_string())
        );
    }

    #[test]
    fn test_parse_trailing_commas() {
        let parser = Parser;
        let content = b"{ greeting: 'Hello', farewell: 'Bye', }";
        let resource = parser.parse(content).expect("parse should succeed");
        assert_eq!(resource.entries.len(), 2);
    }

    #[test]
    fn test_roundtrip() {
        let parser = Parser;
        let writer = Writer;
        // Writer outputs standard JSON, which is valid JSON5, so round-trip works
        let content = br#"{"greeting":"Hello","farewell":"Goodbye"}"#;
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
            "nested.key".to_string(),
            I18nEntry {
                key: "nested.key".to_string(),
                value: EntryValue::Simple("value".to_string()),
                ..Default::default()
            },
        );

        let resource = I18nResource {
            metadata: ResourceMetadata {
                source_format: FormatId::Json5,
                ..Default::default()
            },
            entries,
        };

        let output = writer.write(&resource).expect("write should succeed");
        let output_str = std::str::from_utf8(&output).expect("valid UTF-8");

        let parsed: serde_json::Value = serde_json::from_str(output_str).expect("valid JSON");
        assert_eq!(parsed["nested"]["key"], "value");
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
