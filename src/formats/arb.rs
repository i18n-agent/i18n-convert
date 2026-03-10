use super::*;
use serde_json::{Map, Value};

pub struct Parser;
pub struct Writer;

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        if extension == ".arb" {
            return Confidence::Definite;
        }
        if extension == ".json" {
            if let Ok(s) = std::str::from_utf8(content) {
                if s.contains("\"@@locale\"") {
                    return Confidence::Definite;
                }
            }
        }
        Confidence::None
    }

    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError> {
        let text = std::str::from_utf8(content)
            .map_err(|e| ParseError::InvalidFormat(format!("Invalid UTF-8: {e}")))?;

        let root: Map<String, Value> = serde_json::from_str(text)
            .map_err(|e| ParseError::Json(format!("Failed to parse ARB JSON: {e}")))?;

        let mut metadata = ResourceMetadata {
            source_format: FormatId::Arb,
            ..Default::default()
        };

        let mut entries = IndexMap::new();
        let mut custom_fields: IndexMap<String, Value> = IndexMap::new();

        // First pass: extract file-level @@ metadata
        for (key, value) in &root {
            if !key.starts_with("@@") {
                continue;
            }
            match key.as_str() {
                "@@locale" => {
                    if let Value::String(s) = value {
                        metadata.locale = Some(s.clone());
                    }
                }
                "@@last_modified" => {
                    if let Value::String(s) = value {
                        metadata.modified_at = Some(s.clone());
                    }
                }
                "@@author" => {
                    if let Value::String(s) = value {
                        metadata.created_by = Some(s.clone());
                    }
                }
                "@@context" => {
                    if let Value::String(s) = value {
                        metadata.headers.insert("@@context".to_string(), s.clone());
                    }
                }
                other => {
                    // @@x-* custom fields or other unknown @@ fields
                    if other.starts_with("@@x-") {
                        custom_fields.insert(other.to_string(), value.clone());
                    } else {
                        // Store other unknown @@ fields in headers
                        if let Value::String(s) = value {
                            metadata.headers.insert(other.to_string(), s.clone());
                        }
                    }
                }
            }
        }

        // Set format extension on metadata if we have custom fields
        if !custom_fields.is_empty() {
            metadata.format_ext = Some(FormatExtension::Arb(ArbExt {
                message_type: None,
                custom_fields: custom_fields.clone(),
            }));
        }

        // Second pass: extract entries (keys not starting with @)
        for (key, value) in &root {
            if key.starts_with('@') {
                continue;
            }

            let value_str = match value {
                Value::String(s) => s.clone(),
                _ => continue,
            };

            let mut entry = I18nEntry {
                key: key.clone(),
                value: EntryValue::Simple(value_str),
                ..Default::default()
            };

            // Look for @key metadata
            let meta_key = format!("@{key}");
            if let Some(Value::Object(meta)) = root.get(&meta_key) {
                parse_entry_metadata(&mut entry, meta);
            }

            entries.insert(key.clone(), entry);
        }

        Ok(I18nResource { metadata, entries })
    }

    fn capabilities(&self) -> FormatCapabilities {
        FormatCapabilities {
            plurals: true,
            arrays: false,
            comments: true,
            context: true,
            source_string: false,
            translatable_flag: false,
            translation_state: false,
            max_width: false,
            device_variants: false,
            select_gender: true,
            nested_keys: false,
            inline_markup: false,
            alternatives: false,
            source_references: false,
            custom_properties: true,
        }
    }
}

/// Parse @key metadata object into entry fields
fn parse_entry_metadata(entry: &mut I18nEntry, meta: &Map<String, Value>) {
    // description → Comment with CommentRole::Extracted
    if let Some(Value::String(desc)) = meta.get("description") {
        entry.comments.push(Comment {
            text: desc.clone(),
            role: CommentRole::Extracted,
            priority: None,
            annotates: None,
        });
    }

    // context → ContextEntry
    if let Some(Value::String(ctx)) = meta.get("context") {
        entry.contexts.push(ContextEntry {
            context_type: ContextType::Description,
            value: ctx.clone(),
            purpose: None,
        });
    }

    // type → ArbExt.message_type
    let message_type = meta
        .get("type")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Collect custom/unknown fields from @key metadata
    let mut entry_custom: IndexMap<String, Value> = IndexMap::new();
    for (mk, mv) in meta {
        match mk.as_str() {
            "description" | "context" | "type" | "placeholders" | "source" => {}
            _ => {
                entry_custom.insert(mk.clone(), mv.clone());
            }
        }
    }

    if message_type.is_some() || !entry_custom.is_empty() {
        entry.format_ext = Some(FormatExtension::Arb(ArbExt {
            message_type,
            custom_fields: entry_custom,
        }));
    }

    // source → entry.source
    if let Some(Value::String(src)) = meta.get("source") {
        entry.source = Some(src.clone());
    }

    // placeholders → entry.placeholders
    if let Some(Value::Object(placeholders)) = meta.get("placeholders") {
        for (ph_name, ph_value) in placeholders {
            if let Value::Object(ph_obj) = ph_value {
                let placeholder = parse_placeholder(ph_name, ph_obj);
                entry.placeholders.push(placeholder);
            }
        }
    }
}

/// Parse a single placeholder definition from ARB @key.placeholders
fn parse_placeholder(name: &str, obj: &Map<String, Value>) -> Placeholder {
    let raw_type = obj
        .get("type")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let placeholder_type = raw_type.as_deref().map(|t| match t.to_lowercase().as_str() {
        "string" => PlaceholderType::String,
        "int" | "integer" => PlaceholderType::Integer,
        "double" => PlaceholderType::Double,
        "float" => PlaceholderType::Float,
        "datetime" => PlaceholderType::DateTime,
        "object" => PlaceholderType::Object,
        other => PlaceholderType::Other(other.to_string()),
    });

    let format = obj
        .get("format")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let example = obj
        .get("example")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let description = obj
        .get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let optional_parameters = obj.get("optionalParameters").and_then(|v| {
        if let Value::Object(params) = v {
            let map: IndexMap<String, String> = params
                .iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect();
            if map.is_empty() {
                None
            } else {
                Some(map)
            }
        } else {
            None
        }
    });

    Placeholder {
        name: name.to_string(),
        original_syntax: format!("{{{name}}}"),
        placeholder_type,
        position: None,
        example,
        description,
        format,
        optional_parameters,
    }
}

impl FormatWriter for Writer {
    fn write(&self, resource: &I18nResource) -> Result<Vec<u8>, WriteError> {
        let mut root = Map::new();

        // Write @@locale
        if let Some(locale) = &resource.metadata.locale {
            root.insert("@@locale".to_string(), Value::String(locale.clone()));
        }

        // Write @@last_modified
        if let Some(modified) = &resource.metadata.modified_at {
            root.insert("@@last_modified".to_string(), Value::String(modified.clone()));
        }

        // Write @@author
        if let Some(author) = &resource.metadata.created_by {
            root.insert("@@author".to_string(), Value::String(author.clone()));
        }

        // Write @@context from headers
        if let Some(ctx) = resource.metadata.headers.get("@@context") {
            root.insert("@@context".to_string(), Value::String(ctx.clone()));
        }

        // Write other headers (non-standard @@ fields)
        for (key, value) in &resource.metadata.headers {
            if key == "@@context" {
                continue; // already handled
            }
            if key.starts_with("@@") {
                root.insert(key.clone(), Value::String(value.clone()));
            }
        }

        // Write @@x-* custom fields from resource metadata format extension
        if let Some(FormatExtension::Arb(arb_ext)) = &resource.metadata.format_ext {
            for (key, value) in &arb_ext.custom_fields {
                root.insert(key.clone(), value.clone());
            }
        }

        // Write entries
        for (key, entry) in &resource.entries {
            let value_str = match &entry.value {
                EntryValue::Simple(s) => s.clone(),
                EntryValue::Plural(ps) => plural_set_to_icu(ps, "count"),
                EntryValue::Select(ss) => select_set_to_icu(ss),
                EntryValue::Array(arr) => arr.join(", "),
                EntryValue::MultiVariablePlural(mvp) => mvp.pattern.clone(),
            };

            root.insert(key.clone(), Value::String(value_str));

            // Build @key metadata object
            let mut meta = Map::new();
            let mut has_meta = false;

            // description from Extracted comments
            for comment in &entry.comments {
                if comment.role == CommentRole::Extracted {
                    meta.insert("description".to_string(), Value::String(comment.text.clone()));
                    has_meta = true;
                    break;
                }
            }

            // context from contexts
            for ctx in &entry.contexts {
                if ctx.context_type == ContextType::Description {
                    meta.insert("context".to_string(), Value::String(ctx.value.clone()));
                    has_meta = true;
                    break;
                }
            }

            // type and custom fields from ArbExt
            if let Some(FormatExtension::Arb(arb_ext)) = &entry.format_ext {
                if let Some(msg_type) = &arb_ext.message_type {
                    meta.insert("type".to_string(), Value::String(msg_type.clone()));
                    has_meta = true;
                }
                for (k, v) in &arb_ext.custom_fields {
                    meta.insert(k.clone(), v.clone());
                    has_meta = true;
                }
            }

            // source
            if let Some(src) = &entry.source {
                meta.insert("source".to_string(), Value::String(src.clone()));
                has_meta = true;
            }

            // placeholders
            if !entry.placeholders.is_empty() {
                let mut ph_map = Map::new();
                for ph in &entry.placeholders {
                    let mut ph_obj = Map::new();

                    if let Some(pt) = &ph.placeholder_type {
                        let type_str = match pt {
                            PlaceholderType::String => "String".to_string(),
                            PlaceholderType::Integer => "int".to_string(),
                            PlaceholderType::Float => "float".to_string(),
                            PlaceholderType::Double => "double".to_string(),
                            PlaceholderType::DateTime => "DateTime".to_string(),
                            PlaceholderType::Currency => "currency".to_string(),
                            PlaceholderType::Object => "Object".to_string(),
                            PlaceholderType::Other(s) => s.clone(),
                        };
                        ph_obj.insert("type".to_string(), Value::String(type_str));
                    }

                    if let Some(fmt) = &ph.format {
                        ph_obj.insert("format".to_string(), Value::String(fmt.clone()));
                    }

                    if let Some(ex) = &ph.example {
                        ph_obj.insert("example".to_string(), Value::String(ex.clone()));
                    }

                    if let Some(desc) = &ph.description {
                        ph_obj.insert("description".to_string(), Value::String(desc.clone()));
                    }

                    if let Some(opt_params) = &ph.optional_parameters {
                        let mut params = Map::new();
                        for (k, v) in opt_params {
                            params.insert(k.clone(), Value::String(v.clone()));
                        }
                        ph_obj.insert("optionalParameters".to_string(), Value::Object(params));
                    }

                    ph_map.insert(ph.name.clone(), Value::Object(ph_obj));
                }
                meta.insert("placeholders".to_string(), Value::Object(ph_map));
                has_meta = true;
            }

            if has_meta {
                root.insert(format!("@{key}"), Value::Object(meta));
            }
        }

        let json = serde_json::to_string_pretty(&root)
            .map_err(|e| WriteError::Serialization(format!("Failed to serialize ARB: {e}")))?;

        let mut output = json.into_bytes();
        output.push(b'\n');
        Ok(output)
    }

    fn capabilities(&self) -> FormatCapabilities {
        Parser.capabilities()
    }
}

/// Convert a PluralSet into ICU message syntax
fn plural_set_to_icu(ps: &PluralSet, var_name: &str) -> String {
    let mut parts = Vec::new();

    // Exact matches first
    for (num, val) in &ps.exact_matches {
        parts.push(format!("={num}{{{val}}}"));
    }

    if let Some(zero) = &ps.zero {
        // Only add zero category if not already covered by an exact =0
        if !ps.exact_matches.contains_key(&0) {
            parts.push(format!("zero{{{zero}}}"));
        }
    }
    if let Some(one) = &ps.one {
        parts.push(format!("one{{{one}}}"));
    }
    if let Some(two) = &ps.two {
        parts.push(format!("two{{{two}}}"));
    }
    if let Some(few) = &ps.few {
        parts.push(format!("few{{{few}}}"));
    }
    if let Some(many) = &ps.many {
        parts.push(format!("many{{{many}}}"));
    }
    parts.push(format!("other{{{}}}", ps.other));

    let keyword = if ps.ordinal { "selectordinal" } else { "plural" };
    format!("{{{var_name}, {keyword}, {}}}", parts.join(" "))
}

/// Convert a SelectSet into ICU message syntax
fn select_set_to_icu(ss: &SelectSet) -> String {
    let parts: Vec<String> = ss
        .cases
        .iter()
        .map(|(case, val)| format!("{case}{{{val}}}"))
        .collect();
    format!("{{{}, select, {}}}", ss.variable, parts.join(" "))
}
