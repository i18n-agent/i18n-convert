use super::*;
use indexmap::IndexMap;

pub struct Parser;
pub struct Writer;

// ---------------------------------------------------------------------------
// Parser helpers
// ---------------------------------------------------------------------------

/// Recursively walk a plist dictionary, flattening nested dicts to dot-separated keys.
/// Only extracts String values and Array-of-String values; skips integers, bools, etc.
fn walk_dict(
    dict: &plist::Dictionary,
    prefix: &str,
    entries: &mut IndexMap<String, I18nEntry>,
) {
    for (key, value) in dict.iter() {
        let full_key = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };

        match value {
            plist::Value::String(s) => {
                entries.insert(
                    full_key.clone(),
                    I18nEntry {
                        key: full_key,
                        value: EntryValue::Simple(s.clone()),
                        ..Default::default()
                    },
                );
            }
            plist::Value::Dictionary(nested) => {
                walk_dict(nested, &full_key, entries);
            }
            plist::Value::Array(arr) => {
                // Only include if all elements are strings
                let strings: Vec<String> = arr
                    .iter()
                    .filter_map(|v| {
                        if let plist::Value::String(s) = v {
                            Some(s.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                if strings.len() == arr.len() && !arr.is_empty() {
                    entries.insert(
                        full_key.clone(),
                        I18nEntry {
                            key: full_key,
                            value: EntryValue::Array(strings),
                            ..Default::default()
                        },
                    );
                }
            }
            // Skip non-string, non-dict, non-array values (integers, bools, etc.)
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Writer helpers
// ---------------------------------------------------------------------------

/// Reconstruct a nested plist Dictionary from dot-separated IR entries.
fn unflatten_to_plist(entries: &IndexMap<String, I18nEntry>) -> plist::Dictionary {
    let mut root = plist::Dictionary::new();

    for entry in entries.values() {
        let parts: Vec<&str> = entry.key.split('.').collect();
        let plist_value = entry_value_to_plist(&entry.value);
        insert_nested_plist(&mut root, &parts, plist_value);
    }

    root
}

fn entry_value_to_plist(value: &EntryValue) -> plist::Value {
    match value {
        EntryValue::Simple(s) => plist::Value::String(s.clone()),
        EntryValue::Array(arr) => {
            plist::Value::Array(arr.iter().map(|s| plist::Value::String(s.clone())).collect())
        }
        EntryValue::Plural(ps) => {
            // Plist doesn't natively support plurals; write the "other" form
            plist::Value::String(ps.other.clone())
        }
        EntryValue::Select(ss) => {
            let val = ss.cases.get("other").cloned().unwrap_or_default();
            plist::Value::String(val)
        }
        EntryValue::MultiVariablePlural(mvp) => {
            plist::Value::String(mvp.pattern.clone())
        }
    }
}

fn insert_nested_plist(
    dict: &mut plist::Dictionary,
    parts: &[&str],
    value: plist::Value,
) {
    if parts.is_empty() {
        return;
    }

    if parts.len() == 1 {
        dict.insert(parts[0].to_string(), value);
        return;
    }

    // Navigate/create nested dict
    let key = parts[0].to_string();
    if !dict.contains_key(&key) {
        dict.insert(key.clone(), plist::Value::Dictionary(plist::Dictionary::new()));
    }

    if let Some(plist::Value::Dictionary(ref mut child_dict)) = dict.get_mut(&key) {
        insert_nested_plist(child_dict, &parts[1..], value);
    }
}

// ---------------------------------------------------------------------------
// Trait implementations
// ---------------------------------------------------------------------------

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        let text = String::from_utf8_lossy(content);

        // Exclude stringsdict files (they use .plist but are a different format)
        if text.contains("NSStringLocalizedFormatKey") {
            return Confidence::None;
        }

        if extension == ".plist" {
            return Confidence::Definite;
        }

        // Also detect by content for .xml files that are actually plists
        if extension == ".xml" {
            if text.contains("<plist") || text.contains("<!DOCTYPE plist") {
                return Confidence::High;
            }
        }

        Confidence::None
    }

    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError> {
        let value: plist::Value = plist::from_bytes(content)
            .map_err(|e| ParseError::InvalidFormat(format!("Plist parse error: {e}")))?;

        let dict = match value {
            plist::Value::Dictionary(d) => d,
            _ => {
                return Err(ParseError::InvalidFormat(
                    "Root plist value must be a dictionary".to_string(),
                ));
            }
        };

        let mut entries = IndexMap::new();
        walk_dict(&dict, "", &mut entries);

        Ok(I18nResource {
            metadata: ResourceMetadata {
                source_format: FormatId::IosPlist,
                format_ext: Some(FormatExtension::IosPlist(IosPlistExt {
                    plist_format: Some("xml1".to_string()),
                })),
                ..Default::default()
            },
            entries,
        })
    }

    fn capabilities(&self) -> FormatCapabilities {
        FormatCapabilities {
            nested_keys: true,
            arrays: true,
            ..Default::default()
        }
    }
}

impl FormatWriter for Writer {
    fn write(&self, resource: &I18nResource) -> Result<Vec<u8>, WriteError> {
        let dict = unflatten_to_plist(&resource.entries);
        let plist_value = plist::Value::Dictionary(dict);

        let mut buf = Vec::new();
        plist::to_writer_xml(&mut buf, &plist_value)
            .map_err(|e| WriteError::Serialization(format!("Plist write error: {e}")))?;

        Ok(buf)
    }

    fn capabilities(&self) -> FormatCapabilities {
        Parser.capabilities()
    }
}
