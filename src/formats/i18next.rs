use super::*;
use regex::Regex;
use std::sync::LazyLock;

pub struct Parser;
pub struct Writer;

/// Plural suffixes recognized by i18next (cardinal).
const PLURAL_SUFFIXES: &[(&str, PluralCategory)] = &[
    ("_zero", PluralCategory::Zero),
    ("_one", PluralCategory::One),
    ("_two", PluralCategory::Two),
    ("_few", PluralCategory::Few),
    ("_many", PluralCategory::Many),
    ("_other", PluralCategory::Other),
];

/// Ordinal plural suffixes recognized by i18next.
const ORDINAL_SUFFIXES: &[(&str, PluralCategory)] = &[
    ("_ordinal_zero", PluralCategory::Zero),
    ("_ordinal_one", PluralCategory::One),
    ("_ordinal_two", PluralCategory::Two),
    ("_ordinal_few", PluralCategory::Few),
    ("_ordinal_many", PluralCategory::Many),
    ("_ordinal_other", PluralCategory::Other),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PluralCategory {
    Zero,
    One,
    Two,
    Few,
    Many,
    Other,
}

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        if extension != ".json" {
            return Confidence::None;
        }
        let s = match std::str::from_utf8(content) {
            Ok(s) => s,
            Err(_) => return Confidence::None,
        };
        // Parse JSON and structurally verify plural key pairs in actual keys
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(s) {
            if let Some(obj) = val.as_object() {
                let pairs = count_plural_pairs(obj);
                if pairs >= 2 {
                    return Confidence::Definite;
                }
                if pairs == 1 {
                    return Confidence::High;
                }
            }
        }
        // Fallback: string heuristics for malformed files
        if s.contains("_one\"") && s.contains("_other\"") {
            return Confidence::High;
        }
        if s.contains("_other\"") {
            return Confidence::Low;
        }
        Confidence::None
    }

    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError> {
        let s = std::str::from_utf8(content)
            .map_err(|e| ParseError::InvalidFormat(format!("Invalid UTF-8: {e}")))?;
        let root: serde_json::Value = serde_json::from_str(s)
            .map_err(|e| ParseError::Json(format!("JSON parse error: {e}")))?;

        let obj = root
            .as_object()
            .ok_or_else(|| ParseError::InvalidFormat("Root must be a JSON object".to_string()))?;

        // Flatten nested structure into dot-separated keys with string values
        let mut flat: IndexMap<String, String> = IndexMap::new();
        flatten_json(obj, "", &mut flat);

        // Group plural suffixed keys and build entries
        let entries = group_entries(&flat);

        Ok(I18nResource {
            metadata: ResourceMetadata {
                source_format: FormatId::I18nextJson,
                ..Default::default()
            },
            entries,
        })
    }

    fn capabilities(&self) -> FormatCapabilities {
        FormatCapabilities {
            plurals: true,
            arrays: false,
            comments: false,
            context: true,
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
        // Expand entries back into flat key-value pairs with plural suffixes
        let mut flat: IndexMap<String, String> = IndexMap::new();
        for (key, entry) in &resource.entries {
            match &entry.value {
                EntryValue::Simple(s) => {
                    flat.insert(key.clone(), s.clone());
                }
                EntryValue::Plural(ps) => {
                    let prefix = if ps.ordinal {
                        write_ordinal_plural(key, ps, &mut flat);
                        continue;
                    } else {
                        key.as_str()
                    };
                    if let Some(ref v) = ps.zero {
                        flat.insert(format!("{prefix}_zero"), v.clone());
                    }
                    if let Some(ref v) = ps.one {
                        flat.insert(format!("{prefix}_one"), v.clone());
                    }
                    if let Some(ref v) = ps.two {
                        flat.insert(format!("{prefix}_two"), v.clone());
                    }
                    if let Some(ref v) = ps.few {
                        flat.insert(format!("{prefix}_few"), v.clone());
                    }
                    if let Some(ref v) = ps.many {
                        flat.insert(format!("{prefix}_many"), v.clone());
                    }
                    flat.insert(format!("{prefix}_other"), ps.other.clone());
                }
                EntryValue::Array(arr) => {
                    // i18next doesn't natively support arrays, serialize as JSON array string
                    let json_arr = serde_json::to_string(arr).unwrap_or_else(|_| "[]".to_string());
                    flat.insert(key.clone(), json_arr);
                }
                EntryValue::Select(_) | EntryValue::MultiVariablePlural(_) => {
                    // Best-effort: store as-is using the key
                    // These types don't have native i18next representation
                    if let EntryValue::Select(ss) = &entry.value {
                        if let Some(other) = ss.cases.get("other") {
                            flat.insert(key.clone(), other.clone());
                        }
                    }
                }
            }
        }

        // Reconstruct nested JSON from dot-separated keys
        let nested = unflatten_to_json(&flat);

        let json_str = serde_json::to_string_pretty(&nested)
            .map_err(|e| WriteError::Serialization(format!("JSON serialization error: {e}")))?;
        let mut output = json_str.into_bytes();
        output.push(b'\n');
        Ok(output)
    }

    fn capabilities(&self) -> FormatCapabilities {
        Parser.capabilities()
    }
}

/// Count i18next-style plural key pairs across all levels of a JSON object.
/// A pair requires `base_other` plus at least one sibling (`base_one`, `base_zero`, etc.).
fn count_plural_pairs(obj: &serde_json::Map<String, serde_json::Value>) -> usize {
    let mut count = 0;
    let keys: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
    for key in &keys {
        if let Some(base) = key.strip_suffix("_other") {
            for suffix in &["_one", "_zero", "_many", "_few", "_two"] {
                let sibling = format!("{base}{suffix}");
                if keys.contains(&sibling.as_str()) {
                    count += 1;
                    break;
                }
            }
        }
    }
    for value in obj.values() {
        if let serde_json::Value::Object(nested) = value {
            count += count_plural_pairs(nested);
        }
    }
    count
}

/// Flatten a nested JSON object into dot-separated keys.
fn flatten_json(
    obj: &serde_json::Map<String, serde_json::Value>,
    prefix: &str,
    out: &mut IndexMap<String, String>,
) {
    for (key, value) in obj {
        let full_key = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        match value {
            serde_json::Value::Object(nested) => {
                flatten_json(nested, &full_key, out);
            }
            serde_json::Value::String(s) => {
                out.insert(full_key, s.clone());
            }
            serde_json::Value::Number(n) => {
                out.insert(full_key, n.to_string());
            }
            serde_json::Value::Bool(b) => {
                out.insert(full_key, b.to_string());
            }
            serde_json::Value::Null => {
                out.insert(full_key, String::new());
            }
            serde_json::Value::Array(_) => {
                // Preserve array as JSON string
                out.insert(full_key, value.to_string());
            }
        }
    }
}

/// Group flat keys into IR entries, detecting plural suffixes.
fn group_entries(flat: &IndexMap<String, String>) -> IndexMap<String, I18nEntry> {
    let mut entries: IndexMap<String, I18nEntry> = IndexMap::new();
    // Track which keys have been consumed as part of a plural group
    let mut consumed: std::collections::HashSet<String> = std::collections::HashSet::new();

    // First pass: identify plural groups
    // We scan for keys ending in _other (the required plural form) and check if
    // there are sibling keys with other plural suffixes.
    let all_keys: Vec<String> = flat.keys().cloned().collect();

    // Check for ordinal plurals first (longer suffix match)
    for key in &all_keys {
        if consumed.contains(key) {
            continue;
        }
        if let Some(base) = strip_ordinal_suffix(key) {
            // Only trigger grouping if we find _ordinal_other
            if key.ends_with("_ordinal_other") || has_ordinal_sibling(base, flat) {
                let ps = collect_ordinal_plural(base, flat, &mut consumed);
                let placeholders = extract_placeholders_from_plural(&ps);
                entries.insert(
                    base.to_string(),
                    I18nEntry {
                        key: base.to_string(),
                        value: EntryValue::Plural(ps),
                        placeholders,
                        ..Default::default()
                    },
                );
            }
        }
    }

    // Second pass: cardinal plurals
    for key in &all_keys {
        if consumed.contains(key) {
            continue;
        }
        if let Some((base, _cat)) = strip_cardinal_suffix(key) {
            // Check if we have at least _other to form a valid plural group
            let other_key = format!("{base}_other");
            if flat.contains_key(&other_key) && !consumed.contains(&other_key) {
                let ps = collect_cardinal_plural(base, flat, &mut consumed);
                let placeholders = extract_placeholders_from_plural(&ps);
                entries.insert(
                    base.to_string(),
                    I18nEntry {
                        key: base.to_string(),
                        value: EntryValue::Plural(ps),
                        placeholders,
                        ..Default::default()
                    },
                );
            }
        }
    }

    // Third pass: remaining keys are simple entries
    for (key, value) in flat {
        if consumed.contains(key) {
            continue;
        }
        let placeholders = extract_placeholders(value);
        entries.insert(
            key.clone(),
            I18nEntry {
                key: key.clone(),
                value: EntryValue::Simple(value.clone()),
                placeholders,
                ..Default::default()
            },
        );
    }

    entries
}

/// Strip a cardinal plural suffix from a key, returning the base key and category.
fn strip_cardinal_suffix(key: &str) -> Option<(&str, PluralCategory)> {
    for (suffix, cat) in PLURAL_SUFFIXES {
        if let Some(base) = key.strip_suffix(suffix) {
            if !base.is_empty() {
                return Some((base, *cat));
            }
        }
    }
    None
}

/// Strip an ordinal plural suffix from a key, returning the base key.
fn strip_ordinal_suffix(key: &str) -> Option<&str> {
    for (suffix, _) in ORDINAL_SUFFIXES {
        if let Some(base) = key.strip_suffix(suffix) {
            if !base.is_empty() {
                return Some(base);
            }
        }
    }
    None
}

/// Check if there's at least one ordinal sibling for the given base.
fn has_ordinal_sibling(base: &str, flat: &IndexMap<String, String>) -> bool {
    ORDINAL_SUFFIXES
        .iter()
        .any(|(suffix, _)| flat.contains_key(&format!("{base}{suffix}")))
}

/// Collect all cardinal plural forms for a base key.
fn collect_cardinal_plural(
    base: &str,
    flat: &IndexMap<String, String>,
    consumed: &mut std::collections::HashSet<String>,
) -> PluralSet {
    let mut ps = PluralSet {
        ordinal: false,
        ..Default::default()
    };

    for (suffix, cat) in PLURAL_SUFFIXES {
        let full_key = format!("{base}{suffix}");
        if let Some(value) = flat.get(&full_key) {
            consumed.insert(full_key);
            match cat {
                PluralCategory::Zero => ps.zero = Some(value.clone()),
                PluralCategory::One => ps.one = Some(value.clone()),
                PluralCategory::Two => ps.two = Some(value.clone()),
                PluralCategory::Few => ps.few = Some(value.clone()),
                PluralCategory::Many => ps.many = Some(value.clone()),
                PluralCategory::Other => ps.other = value.clone(),
            }
        }
    }

    ps
}

/// Collect all ordinal plural forms for a base key.
fn collect_ordinal_plural(
    base: &str,
    flat: &IndexMap<String, String>,
    consumed: &mut std::collections::HashSet<String>,
) -> PluralSet {
    let mut ps = PluralSet {
        ordinal: true,
        ..Default::default()
    };

    for (suffix, cat) in ORDINAL_SUFFIXES {
        let full_key = format!("{base}{suffix}");
        if let Some(value) = flat.get(&full_key) {
            consumed.insert(full_key);
            match cat {
                PluralCategory::Zero => ps.zero = Some(value.clone()),
                PluralCategory::One => ps.one = Some(value.clone()),
                PluralCategory::Two => ps.two = Some(value.clone()),
                PluralCategory::Few => ps.few = Some(value.clone()),
                PluralCategory::Many => ps.many = Some(value.clone()),
                PluralCategory::Other => ps.other = value.clone(),
            }
        }
    }

    ps
}

static RE_I18NEXT_PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\{\{(\s*\w+(?:\s*,\s*\w+)?\s*)\}\}").expect("valid regex pattern")
});

/// Extract {{name}} style placeholders from a string.
fn extract_placeholders(value: &str) -> Vec<Placeholder> {
    let mut placeholders = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for cap in RE_I18NEXT_PLACEHOLDER.captures_iter(value) {
        let full_match = cap.get(0).expect("regex group 0 always captures").as_str();
        let inner = cap
            .get(1)
            .expect("regex group 1 always captures")
            .as_str()
            .trim();
        // The name is the part before any comma (format hint)
        let name = inner
            .split(',')
            .next()
            .expect("split always yields at least one element")
            .trim()
            .to_string();
        if seen.contains(&name) {
            continue;
        }
        seen.insert(name.clone());
        placeholders.push(Placeholder {
            name,
            original_syntax: full_match.to_string(),
            placeholder_type: Some(PlaceholderType::String),
            position: None,
            example: None,
            description: None,
            format: None,
            optional_parameters: None,
        });
    }
    placeholders
}

/// Extract placeholders from all forms of a PluralSet.
fn extract_placeholders_from_plural(ps: &PluralSet) -> Vec<Placeholder> {
    let mut all_text = ps.other.clone();
    for v in [&ps.zero, &ps.one, &ps.two, &ps.few, &ps.many]
        .into_iter()
        .flatten()
    {
        all_text.push(' ');
        all_text.push_str(v);
    }
    extract_placeholders(&all_text)
}

/// Write ordinal plural suffixed keys.
fn write_ordinal_plural(base: &str, ps: &PluralSet, flat: &mut IndexMap<String, String>) {
    if let Some(ref v) = ps.zero {
        flat.insert(format!("{base}_ordinal_zero"), v.clone());
    }
    if let Some(ref v) = ps.one {
        flat.insert(format!("{base}_ordinal_one"), v.clone());
    }
    if let Some(ref v) = ps.two {
        flat.insert(format!("{base}_ordinal_two"), v.clone());
    }
    if let Some(ref v) = ps.few {
        flat.insert(format!("{base}_ordinal_few"), v.clone());
    }
    if let Some(ref v) = ps.many {
        flat.insert(format!("{base}_ordinal_many"), v.clone());
    }
    flat.insert(format!("{base}_ordinal_other"), ps.other.clone());
}

/// Reconstruct a nested JSON object from dot-separated keys.
fn unflatten_to_json(flat: &IndexMap<String, String>) -> serde_json::Value {
    let mut root = serde_json::Map::new();

    for (key, value) in flat {
        let parts: Vec<&str> = key.split('.').collect();
        insert_nested(&mut root, &parts, value);
    }

    serde_json::Value::Object(root)
}

/// Insert a value into a nested JSON structure following the path of key parts.
fn insert_nested(
    obj: &mut serde_json::Map<String, serde_json::Value>,
    parts: &[&str],
    value: &str,
) {
    if parts.len() == 1 {
        obj.insert(
            parts[0].to_string(),
            serde_json::Value::String(value.to_string()),
        );
        return;
    }

    let child = obj
        .entry(parts[0].to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));

    if let serde_json::Value::Object(ref mut child_obj) = child {
        insert_nested(child_obj, &parts[1..], value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_cardinal_suffix() {
        assert_eq!(
            strip_cardinal_suffix("item_one"),
            Some(("item", PluralCategory::One))
        );
        assert_eq!(
            strip_cardinal_suffix("item_other"),
            Some(("item", PluralCategory::Other))
        );
        assert_eq!(strip_cardinal_suffix("item"), None);
        assert_eq!(
            strip_cardinal_suffix("some.key_few"),
            Some(("some.key", PluralCategory::Few))
        );
    }

    #[test]
    fn test_strip_ordinal_suffix() {
        assert_eq!(strip_ordinal_suffix("ordinal_ordinal_one"), Some("ordinal"));
        assert_eq!(strip_ordinal_suffix("item_one"), None);
    }

    #[test]
    fn test_extract_placeholders() {
        let placeholders = extract_placeholders("Hello, {{name}}! You have {{count}} items.");
        assert_eq!(placeholders.len(), 2);
        assert_eq!(placeholders[0].name, "name");
        assert_eq!(placeholders[0].original_syntax, "{{name}}");
        assert_eq!(placeholders[1].name, "count");
    }

    #[test]
    fn test_extract_placeholders_with_format() {
        let placeholders = extract_placeholders("Total: {{price, currency}}");
        assert_eq!(placeholders.len(), 1);
        assert_eq!(placeholders[0].name, "price");
        assert_eq!(placeholders[0].original_syntax, "{{price, currency}}");
    }

    #[test]
    fn test_flatten_json() {
        let json: serde_json::Value =
            serde_json::from_str(r#"{"a": {"b": "c", "d": "e"}, "f": "g"}"#).unwrap();
        let obj = json.as_object().unwrap();
        let mut flat = IndexMap::new();
        flatten_json(obj, "", &mut flat);
        assert_eq!(flat.get("a.b"), Some(&"c".to_string()));
        assert_eq!(flat.get("a.d"), Some(&"e".to_string()));
        assert_eq!(flat.get("f"), Some(&"g".to_string()));
    }

    #[test]
    fn test_unflatten_to_json() {
        let mut flat = IndexMap::new();
        flat.insert("a.b".to_string(), "c".to_string());
        flat.insert("a.d".to_string(), "e".to_string());
        flat.insert("f".to_string(), "g".to_string());

        let result = unflatten_to_json(&flat);
        assert_eq!(result["a"]["b"], "c");
        assert_eq!(result["a"]["d"], "e");
        assert_eq!(result["f"], "g");
    }
}
