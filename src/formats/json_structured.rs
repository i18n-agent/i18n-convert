use super::*;

pub struct Parser;
pub struct Writer;

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        if extension != ".json" {
            return Confidence::None;
        }
        let s = match std::str::from_utf8(content) {
            Ok(s) => s,
            Err(_) => return Confidence::None,
        };
        // Exclude ARB (has @@locale) and xcstrings (has sourceLanguage+strings)
        if s.contains("\"@@locale\"") {
            return Confidence::None;
        }
        if s.contains("\"sourceLanguage\"") && s.contains("\"strings\"") {
            return Confidence::None;
        }
        // Try to parse as JSON for structural analysis
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(s) {
            if let Some(obj) = val.as_object() {
                // Exclude i18next: check for actual plural key pairs in the object tree
                if has_plural_key_pairs(obj) {
                    return Confidence::None;
                }
                // Valid JSON object with no competing format signals
                return Confidence::High;
            }
        }
        // Fallback: looks like JSON but didn't fully parse
        if s.trim_start().starts_with('{') {
            return Confidence::Low;
        }
        Confidence::None
    }

    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError> {
        let s = std::str::from_utf8(content)
            .map_err(|e| ParseError::InvalidFormat(format!("Invalid UTF-8: {e}")))?;

        let root: serde_json::Value =
            serde_json::from_str(s).map_err(|e| ParseError::Json(format!("{e}")))?;

        let obj = root
            .as_object()
            .ok_or_else(|| ParseError::InvalidFormat("Root must be a JSON object".to_string()))?;

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

/// Check if a JSON object has i18next-style plural key pairs at any nesting level.
/// Requires both `base_other` and at least one sibling (`base_one`, `base_zero`, etc.)
/// to avoid false positives from keys that just happen to end with `_other`.
fn has_plural_key_pairs(obj: &serde_json::Map<String, serde_json::Value>) -> bool {
    let keys: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
    for key in &keys {
        if let Some(base) = key.strip_suffix("_other") {
            for suffix in &["_one", "_zero", "_many", "_few", "_two"] {
                let sibling = format!("{base}{suffix}");
                if keys.contains(&sibling.as_str()) {
                    return true;
                }
            }
        }
    }
    for value in obj.values() {
        if let serde_json::Value::Object(nested) = value {
            if has_plural_key_pairs(nested) {
                return true;
            }
        }
    }
    false
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
                EntryValue::Select(ss) => ss.cases.get("other").cloned().unwrap_or_default(),
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
        if let Some(existing) = root.get(key) {
            if existing.is_object() {
                let obj = root.get_mut(key).unwrap().as_object_mut().unwrap();
                obj.insert("_content".to_string(), value);
                return;
            }
        }
        root.insert(key.to_string(), value);
        return;
    }

    let mut current = root;
    for part in &parts[..parts.len() - 1] {
        let entry = current
            .entry(part.to_string())
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));

        if !entry.is_object() {
            let old_value = entry.clone();
            *entry = serde_json::Value::Object(serde_json::Map::new());
            entry
                .as_object_mut()
                .unwrap()
                .insert("_content".to_string(), old_value);
        }

        current = entry.as_object_mut().unwrap();
    }

    let leaf_key = parts[parts.len() - 1];
    current.insert(leaf_key.to_string(), value);
}
