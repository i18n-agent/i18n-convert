use super::*;
use std::io::Cursor;

pub struct Parser;
pub struct Writer;

/// Keys that are stringsdict metadata, not plural categories.
const META_KEYS: &[&str] = &["NSStringFormatSpecTypeKey", "NSStringFormatValueTypeKey"];

/// Regex pattern for `%#@varname@` references in the format key.
fn extract_variable_names(pattern: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut remaining = pattern;
    while let Some(start) = remaining.find("%#@") {
        let after_prefix = &remaining[start + 3..];
        if let Some(end) = after_prefix.find('@') {
            names.push(after_prefix[..end].to_string());
            remaining = &after_prefix[end + 1..];
        } else {
            break;
        }
    }
    names
}

/// Parse a plist Dictionary representing a single variable's plural rules.
fn parse_variable_dict(name: &str, dict: &plist::Dictionary) -> Result<PluralVariable, ParseError> {
    let format_spec_type = dict
        .get("NSStringFormatSpecTypeKey")
        .and_then(|v| v.as_string())
        .map(|s| s.to_string());

    // Validate that it's a plural rule type (the only type we support)
    if let Some(ref spec_type) = format_spec_type {
        if spec_type != "NSStringPluralRuleType" {
            return Err(ParseError::InvalidFormat(format!(
                "Unsupported NSStringFormatSpecTypeKey: {spec_type}"
            )));
        }
    }

    let format_specifier = dict
        .get("NSStringFormatValueTypeKey")
        .and_then(|v| v.as_string())
        .map(|s| s.to_string());

    let mut plural_set = PluralSet::default();

    for (key, value) in dict.iter() {
        if META_KEYS.contains(&key.as_str()) {
            continue;
        }
        let text = value.as_string().unwrap_or("").to_string();
        match key.as_str() {
            "zero" => plural_set.zero = Some(text),
            "one" => plural_set.one = Some(text),
            "two" => plural_set.two = Some(text),
            "few" => plural_set.few = Some(text),
            "many" => plural_set.many = Some(text),
            "other" => plural_set.other = text,
            _ => {
                // Unknown keys are ignored (future-proofing)
            }
        }
    }

    Ok(PluralVariable {
        name: name.to_string(),
        format_specifier,
        arg_num: None,
        plural_set,
    })
}

/// Parse a single entry dict (the value for a top-level key).
fn parse_entry_dict(key: &str, dict: &plist::Dictionary) -> Result<I18nEntry, ParseError> {
    let format_key = dict
        .get("NSStringLocalizedFormatKey")
        .and_then(|v| v.as_string())
        .ok_or_else(|| {
            ParseError::InvalidFormat(format!("Entry '{key}' missing NSStringLocalizedFormatKey"))
        })?
        .to_string();

    let var_names = extract_variable_names(&format_key);

    // Parse all variable dicts from the entry
    let mut variables = IndexMap::new();
    for (sub_key, sub_value) in dict.iter() {
        if sub_key == "NSStringLocalizedFormatKey" {
            continue;
        }
        if let plist::Value::Dictionary(var_dict) = sub_value {
            let var = parse_variable_dict(sub_key, var_dict)?;
            variables.insert(sub_key.to_string(), var);
        }
    }

    // Determine whether this is a single-variable plural or multi-variable plural.
    // Single-variable: format key is exactly `%#@varname@` (one variable, nothing else)
    let is_single_var = variables.len() == 1
        && var_names.len() == 1
        && format_key == format!("%#@{}@", var_names[0]);

    let value = if is_single_var {
        // Single variable → flatten to EntryValue::Plural
        let var = variables
            .into_values()
            .next()
            .expect("len == 1 checked above");
        EntryValue::Plural(var.plural_set)
    } else {
        // Multi-variable → EntryValue::MultiVariablePlural
        EntryValue::MultiVariablePlural(MultiVariablePlural {
            pattern: format_key,
            variables,
        })
    };

    // Store the format_spec_type in extension for round-trip fidelity.
    // For single-var entries, also preserve the format_specifier and variable name.
    let format_ext = match &value {
        EntryValue::Plural(_) => {
            // For single var, we need to remember the variable name and format specifier
            // so the writer can reconstruct the stringsdict structure.
            // We store the variable name in properties and format_specifier in ext.
            None
        }
        _ => None,
    };

    Ok(I18nEntry {
        key: key.to_string(),
        value,
        format_ext,
        ..Default::default()
    })
}

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        if extension == ".stringsdict" {
            return Confidence::Definite;
        }
        if extension == ".plist" || extension == ".xml" {
            if let Ok(s) = std::str::from_utf8(content) {
                if s.contains("NSStringLocalizedFormatKey") {
                    return Confidence::Definite;
                }
            }
        }
        Confidence::None
    }

    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError> {
        let plist_value = plist::Value::from_reader_xml(Cursor::new(content))
            .map_err(|e| ParseError::Xml(format!("Failed to parse plist XML: {e}")))?;

        let root_dict = plist_value.as_dictionary().ok_or_else(|| {
            ParseError::InvalidFormat("Root plist value is not a dictionary".to_string())
        })?;

        let mut entries = IndexMap::new();

        for (key, value) in root_dict.iter() {
            match value {
                plist::Value::Dictionary(entry_dict) => {
                    let entry = parse_entry_dict(key, entry_dict)?;
                    entries.insert(key.to_string(), entry);
                }
                _ => {
                    return Err(ParseError::InvalidFormat(format!(
                        "Expected dictionary for key '{}', got {:?}",
                        key,
                        value.as_string().unwrap_or("<non-string>")
                    )));
                }
            }
        }

        Ok(I18nResource {
            metadata: ResourceMetadata {
                source_format: FormatId::Stringsdict,
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
            context: false,
            source_string: false,
            translatable_flag: false,
            translation_state: false,
            max_width: false,
            device_variants: false,
            select_gender: false,
            nested_keys: false,
            inline_markup: false,
            alternatives: false,
            source_references: false,
            custom_properties: false,
        }
    }
}

// ─── Writer ───────────────────────────────────────────────────────────────────

/// Write a PluralVariable into a plist Dictionary.
fn write_variable_dict(var: &PluralVariable) -> plist::Dictionary {
    let mut dict = plist::Dictionary::new();
    dict.insert(
        "NSStringFormatSpecTypeKey".to_string(),
        plist::Value::String("NSStringPluralRuleType".to_string()),
    );
    dict.insert(
        "NSStringFormatValueTypeKey".to_string(),
        plist::Value::String(
            var.format_specifier
                .clone()
                .unwrap_or_else(|| "d".to_string()),
        ),
    );

    // Write plural categories in canonical order
    if let Some(ref zero) = var.plural_set.zero {
        dict.insert("zero".to_string(), plist::Value::String(zero.clone()));
    }
    if let Some(ref one) = var.plural_set.one {
        dict.insert("one".to_string(), plist::Value::String(one.clone()));
    }
    if let Some(ref two) = var.plural_set.two {
        dict.insert("two".to_string(), plist::Value::String(two.clone()));
    }
    if let Some(ref few) = var.plural_set.few {
        dict.insert("few".to_string(), plist::Value::String(few.clone()));
    }
    if let Some(ref many) = var.plural_set.many {
        dict.insert("many".to_string(), plist::Value::String(many.clone()));
    }
    dict.insert(
        "other".to_string(),
        plist::Value::String(var.plural_set.other.clone()),
    );

    dict
}

/// Derive a variable name from the entry key for single-variable plurals.
/// Uses the entry key itself as the variable name.
fn derive_var_name(key: &str) -> String {
    key.to_string()
}

/// Infer the format specifier from the plural text values.
/// Looks for common printf-style patterns like %d, %lld, %@, %f, etc.
fn infer_format_specifier(plural_set: &PluralSet) -> String {
    // Check all non-None values for a format specifier pattern
    let values: Vec<&str> = [
        plural_set.zero.as_deref(),
        plural_set.one.as_deref(),
        plural_set.two.as_deref(),
        plural_set.few.as_deref(),
        plural_set.many.as_deref(),
        Some(plural_set.other.as_str()),
    ]
    .into_iter()
    .flatten()
    .collect();

    for val in &values {
        // Look for %lld, %d, %f, %@, %lu, etc.
        if let Some(pos) = val.find('%') {
            let after = &val[pos + 1..];
            // Skip %# (variable references) and %% (literal percent)
            if after.starts_with('#') || after.starts_with('%') {
                continue;
            }
            // Try to extract the format specifier
            // Patterns: %d, %lld, %lu, %f, %@, %ld, etc.
            let spec_chars: String = after
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '@')
                .collect();
            if !spec_chars.is_empty() {
                return spec_chars;
            }
        }
    }

    // Default to "d" (integer) if we can't detect
    "d".to_string()
}

impl FormatWriter for Writer {
    fn write(&self, resource: &I18nResource) -> Result<Vec<u8>, WriteError> {
        let mut root = plist::Dictionary::new();

        for (key, entry) in &resource.entries {
            let entry_dict = match &entry.value {
                EntryValue::Plural(plural_set) => {
                    // Single-variable plural → reconstruct stringsdict structure
                    let var_name = derive_var_name(key);
                    let format_specifier = infer_format_specifier(plural_set);

                    let var = PluralVariable {
                        name: var_name.clone(),
                        format_specifier: Some(format_specifier),
                        arg_num: None,
                        plural_set: plural_set.clone(),
                    };

                    let mut dict = plist::Dictionary::new();
                    dict.insert(
                        "NSStringLocalizedFormatKey".to_string(),
                        plist::Value::String(format!("%#@{var_name}@")),
                    );
                    dict.insert(
                        var_name,
                        plist::Value::Dictionary(write_variable_dict(&var)),
                    );
                    dict
                }
                EntryValue::MultiVariablePlural(mvp) => {
                    let mut dict = plist::Dictionary::new();
                    dict.insert(
                        "NSStringLocalizedFormatKey".to_string(),
                        plist::Value::String(mvp.pattern.clone()),
                    );
                    for (var_name, var) in &mvp.variables {
                        dict.insert(
                            var_name.clone(),
                            plist::Value::Dictionary(write_variable_dict(var)),
                        );
                    }
                    dict
                }
                EntryValue::Simple(text) => {
                    // Simple strings get wrapped: format key is just the text
                    // (no variable references needed)
                    let mut dict = plist::Dictionary::new();
                    dict.insert(
                        "NSStringLocalizedFormatKey".to_string(),
                        plist::Value::String(text.clone()),
                    );
                    dict
                }
                _ => {
                    eprintln!(
                        "Warning: skipping entry '{key}' with unsupported value type for stringsdict"
                    );
                    continue;
                }
            };

            root.insert(key.clone(), plist::Value::Dictionary(entry_dict));
        }

        let plist_value = plist::Value::Dictionary(root);
        let mut buf = Vec::new();
        plist_value
            .to_writer_xml(&mut buf)
            .map_err(|e| WriteError::Serialization(format!("Failed to write plist XML: {e}")))?;

        Ok(buf)
    }

    fn capabilities(&self) -> FormatCapabilities {
        Parser.capabilities()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_variable_names_single() {
        let names = extract_variable_names("%#@items@");
        assert_eq!(names, vec!["items"]);
    }

    #[test]
    fn test_extract_variable_names_multi() {
        let names = extract_variable_names("%#@files@ in %#@folders@");
        assert_eq!(names, vec!["files", "folders"]);
    }

    #[test]
    fn test_extract_variable_names_none() {
        let names = extract_variable_names("Hello world");
        assert!(names.is_empty());
    }

    #[test]
    fn test_extract_variable_names_with_text_around() {
        let names = extract_variable_names("You have %#@count@ new %#@type@.");
        assert_eq!(names, vec!["count", "type"]);
    }

    #[test]
    fn test_infer_format_specifier_integer() {
        let ps = PluralSet {
            one: Some("%d item".to_string()),
            other: "%d items".to_string(),
            ..Default::default()
        };
        assert_eq!(infer_format_specifier(&ps), "d");
    }

    #[test]
    fn test_infer_format_specifier_long_long() {
        let ps = PluralSet {
            one: Some("%lld item".to_string()),
            other: "%lld items".to_string(),
            ..Default::default()
        };
        assert_eq!(infer_format_specifier(&ps), "lld");
    }

    #[test]
    fn test_infer_format_specifier_string() {
        let ps = PluralSet {
            other: "Hello %@".to_string(),
            ..Default::default()
        };
        // %@ is Obj-C string format
        assert_eq!(infer_format_specifier(&ps), "@");
    }

    #[test]
    fn test_infer_format_specifier_default() {
        let ps = PluralSet {
            other: "No format here".to_string(),
            ..Default::default()
        };
        assert_eq!(infer_format_specifier(&ps), "d");
    }

    #[test]
    fn test_detect_stringsdict_extension() {
        let parser = Parser;
        assert_eq!(parser.detect(".stringsdict", b""), Confidence::Definite);
    }

    #[test]
    fn test_detect_plist_with_content() {
        let parser = Parser;
        let content = b"<plist><dict><key>NSStringLocalizedFormatKey</key></dict></plist>";
        assert_eq!(parser.detect(".plist", content), Confidence::Definite);
    }

    #[test]
    fn test_detect_unrelated() {
        let parser = Parser;
        assert_eq!(parser.detect(".json", b"{}"), Confidence::None);
    }
}
