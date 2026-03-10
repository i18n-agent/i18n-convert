use crate::ir::*;
use super::*;

pub struct Parser;
pub struct Writer;

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        if extension == ".json" {
            if let Ok(s) = std::str::from_utf8(content) {
                // Exclude ARB (has @@locale) and xcstrings (has sourceLanguage+strings)
                if s.contains("\"@@locale\"") {
                    return Confidence::None;
                }
                if s.contains("\"sourceLanguage\"") && s.contains("\"strings\"") {
                    return Confidence::None;
                }
                // Check for i18next plural suffixes
                if s.contains("_one\"") || s.contains("_other\"") {
                    return Confidence::None;
                }
                // Generic JSON with string values
                if s.trim_start().starts_with('{') {
                    return Confidence::Low;
                }
            }
        }
        Confidence::None
    }

    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError> {
        let s = std::str::from_utf8(content)
            .map_err(|e| ParseError::InvalidFormat(format!("Invalid UTF-8: {e}")))?;

        let root: serde_json::Value = serde_json::from_str(s)
            .map_err(|e| ParseError::Json(format!("{e}")))?;

        let obj = root.as_object().ok_or_else(|| {
            ParseError::InvalidFormat("Root must be a JSON object".to_string())
        })?;

        let mut entries = IndexMap::new();
        flatten_object(obj, &mut String::new(), &mut entries);

        Ok(I18nResource {
            metadata: ResourceMetadata {
                source_format: FormatId::JsonStructured,
                ..Default::default()
            },
            entries,
        })
    }

    fn capabilities(&self) -> FormatCapabilities {
        FormatCapabilities {
            plurals: false,
            arrays: false,
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

/// Recursively flatten a JSON object into dot-separated keys with simple string values.
fn flatten_object(
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
                flatten_object(nested, &mut full_key.clone(), entries);
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
            // Numbers, booleans, null — coerce to string
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
            // Arrays and null are skipped (structured JSON i18n files use strings)
            _ => {}
        }
    }
}

impl FormatWriter for Writer {
    fn write(&self, resource: &I18nResource) -> Result<Vec<u8>, WriteError> {
        let mut root = serde_json::Map::new();

        for (_, entry) in &resource.entries {
            let value_str = match &entry.value {
                EntryValue::Simple(s) => s.clone(),
                // For non-simple values, use a reasonable string representation
                EntryValue::Plural(ps) => ps.other.clone(),
                EntryValue::Array(arr) => arr.join(", "),
                EntryValue::Select(ss) => {
                    ss.cases.get("other").cloned().unwrap_or_default()
                }
                EntryValue::MultiVariablePlural(mvp) => mvp.pattern.clone(),
            };

            insert_nested(&mut root, &entry.key, serde_json::Value::String(value_str));
        }

        let json = serde_json::to_string_pretty(&serde_json::Value::Object(root))
            .map_err(|e| WriteError::Serialization(format!("{e}")))?;

        // Ensure trailing newline
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
/// e.g., "common.greeting" → {"common": {"greeting": value}}
fn insert_nested(
    root: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    value: serde_json::Value,
) {
    let parts: Vec<&str> = key.split('.').collect();
    if parts.len() == 1 {
        root.insert(key.to_string(), value);
        return;
    }

    // Navigate/create nested objects for all parts except the last
    let mut current = root;
    for part in &parts[..parts.len() - 1] {
        current = current
            .entry(part.to_string())
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()))
            .as_object_mut()
            .expect("Expected nested object in key path");
    }

    // Insert the leaf value
    let leaf_key = parts[parts.len() - 1];
    current.insert(leaf_key.to_string(), value);
}
