use super::*;
use indexmap::IndexMap;
use regex::Regex;
use std::sync::LazyLock;

pub struct Parser;
pub struct Writer;

// ---------------------------------------------------------------------------
// Comment extraction
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct LineComment {
    text: String,
    line: usize,
}

fn extract_line_comments(text: &str) -> Vec<LineComment> {
    let mut comments = Vec::new();
    for (line_no, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            let text = trimmed[2..].trim().to_string();
            comments.push(LineComment { text, line: line_no });
        }
    }
    comments
}

fn find_comments_for_line(comments: &[LineComment], key_line: usize) -> Vec<String> {
    if key_line == 0 {
        return Vec::new();
    }
    let mut result = Vec::new();
    let mut check_line = key_line - 1;
    loop {
        if let Some(c) = comments.iter().find(|c| c.line == check_line) {
            result.push(c.text.clone());
            if check_line == 0 {
                break;
            }
            check_line -= 1;
        } else {
            break;
        }
    }
    result.reverse();
    result
}

fn find_key_line(text: &str, key: &str) -> Option<usize> {
    for (line_no, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with(&format!("{}:", key))
            || trimmed.starts_with(&format!("{} :", key))
            || trimmed.starts_with(&format!("\"{}\":", key))
            || trimmed.starts_with(&format!("\"{}\" :", key))
            || trimmed.starts_with(&format!("'{}':", key))
            || trimmed.starts_with(&format!("'{}' :", key))
        {
            return Some(line_no);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Export style & type annotation detection
// ---------------------------------------------------------------------------

static RE_EXPORT_DEFAULT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*export\s+default\b").expect("valid regex")
});

static RE_EXPORT_CONST: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*export\s+const\b").expect("valid regex")
});

/// Detect `const <name>: <TypeAnnotation> = {` or similar patterns.
static RE_TYPE_ANNOTATION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)(?:const|let|var)\s+\w+\s*:\s*([^=]+?)\s*=").expect("valid regex")
});

fn detect_export_style(text: &str) -> Option<String> {
    if RE_EXPORT_DEFAULT.is_match(text) {
        Some("export default".to_string())
    } else if RE_EXPORT_CONST.is_match(text) {
        Some("export const".to_string())
    } else {
        None
    }
}

fn detect_type_annotation(text: &str) -> Option<String> {
    RE_TYPE_ANNOTATION
        .captures(text)
        .map(|cap| cap.get(1).expect("group 1").as_str().trim().to_string())
}

fn detect_quote_style(text: &str) -> Option<char> {
    let single_count = text.matches('\'').count();
    let double_count = text.matches('"').count();
    if single_count > double_count {
        Some('\'')
    } else if double_count > 0 {
        Some('"')
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Object literal extraction & cleaning
// (shared logic with js_format, but kept self-contained to avoid cross-module deps)
// ---------------------------------------------------------------------------

fn extract_object_literal(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let bytes = text.as_bytes();
    let mut depth = 0;
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut in_template = false;
    let mut escape = false;

    for i in start..bytes.len() {
        let ch = bytes[i] as char;

        if escape {
            escape = false;
            continue;
        }

        if ch == '\\' && (in_single_quote || in_double_quote || in_template) {
            escape = true;
            continue;
        }

        if in_single_quote {
            if ch == '\'' {
                in_single_quote = false;
            }
            continue;
        }

        if in_double_quote {
            if ch == '"' {
                in_double_quote = false;
            }
            continue;
        }

        if in_template {
            if ch == '`' {
                in_template = false;
            }
            continue;
        }

        match ch {
            '\'' => in_single_quote = true,
            '"' => in_double_quote = true,
            '`' => in_template = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&text[start..=i]);
                }
            }
            _ => {}
        }
    }
    None
}

fn strip_js_comments(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if chars[i] == '"' || chars[i] == '\'' || chars[i] == '`' {
            let quote = chars[i];
            result.push(quote);
            i += 1;
            while i < len {
                if chars[i] == '\\' {
                    result.push(chars[i]);
                    i += 1;
                    if i < len {
                        result.push(chars[i]);
                        i += 1;
                    }
                    continue;
                }
                if chars[i] == quote {
                    result.push(quote);
                    i += 1;
                    break;
                }
                result.push(chars[i]);
                i += 1;
            }
            continue;
        }

        if i + 1 < len && chars[i] == '/' && chars[i + 1] == '/' {
            while i < len && chars[i] != '\n' {
                i += 1;
            }
            if i < len {
                result.push('\n');
                i += 1;
            }
            continue;
        }

        if i + 1 < len && chars[i] == '/' && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < len {
                if chars[i] == '*' && chars[i + 1] == '/' {
                    i += 2;
                    break;
                }
                if chars[i] == '\n' {
                    result.push('\n');
                }
                i += 1;
            }
            continue;
        }

        result.push(chars[i]);
        i += 1;
    }

    result
}

fn clean_object_for_json(text: &str) -> String {
    let no_comments = strip_js_comments(text);
    let chars: Vec<char> = no_comments.chars().collect();
    let len = chars.len();
    let mut result = String::with_capacity(len);
    let mut i = 0;

    while i < len {
        // Double-quoted strings: pass through
        if chars[i] == '"' {
            result.push('"');
            i += 1;
            while i < len {
                if chars[i] == '\\' {
                    result.push(chars[i]);
                    i += 1;
                    if i < len {
                        result.push(chars[i]);
                        i += 1;
                    }
                    continue;
                }
                if chars[i] == '"' {
                    result.push('"');
                    i += 1;
                    break;
                }
                result.push(chars[i]);
                i += 1;
            }
            continue;
        }

        // Single-quoted strings: convert to double-quoted
        if chars[i] == '\'' {
            result.push('"');
            i += 1;
            while i < len {
                if chars[i] == '\\' {
                    i += 1;
                    if i < len {
                        match chars[i] {
                            '\'' => result.push('\''),
                            '"' => {
                                result.push('\\');
                                result.push('"');
                            }
                            other => {
                                result.push('\\');
                                result.push(other);
                            }
                        }
                        i += 1;
                    }
                    continue;
                }
                if chars[i] == '\'' {
                    result.push('"');
                    i += 1;
                    break;
                }
                if chars[i] == '"' {
                    result.push('\\');
                    result.push('"');
                    i += 1;
                    continue;
                }
                result.push(chars[i]);
                i += 1;
            }
            continue;
        }

        // Unquoted identifiers (potential keys)
        if is_js_identifier_start(chars[i]) {
            let start = i;
            while i < len && is_js_identifier_char(chars[i]) {
                i += 1;
            }
            let ident = &no_comments[start..i];

            let mut j = i;
            while j < len && chars[j].is_whitespace() {
                j += 1;
            }

            if j < len && chars[j] == ':' {
                result.push('"');
                result.push_str(ident);
                result.push('"');
            } else {
                result.push_str(ident);
            }
            continue;
        }

        result.push(chars[i]);
        i += 1;
    }

    remove_trailing_commas(&result)
}

fn is_js_identifier_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || c == '$'
}

fn is_js_identifier_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '$'
}

fn remove_trailing_commas(text: &str) -> String {
    static RE_TRAILING_COMMA: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r",(\s*[}\]])").expect("valid regex")
    });
    RE_TRAILING_COMMA.replace_all(text, "$1").to_string()
}

// ---------------------------------------------------------------------------
// Flatten JSON to dot-separated keys
// ---------------------------------------------------------------------------

fn flatten_json_object(
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
                flatten_json_object(nested, &full_key, out);
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
            serde_json::Value::Array(arr) => {
                let s = serde_json::to_string(arr).unwrap_or_else(|_| "[]".to_string());
                out.insert(full_key, s);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Group flat keys into IR entries
// ---------------------------------------------------------------------------

fn group_entries(
    flat: &IndexMap<String, String>,
    source_text: &str,
    line_comments: &[LineComment],
) -> IndexMap<String, I18nEntry> {
    let mut entries: IndexMap<String, I18nEntry> = IndexMap::new();
    let mut consumed: std::collections::HashSet<String> = std::collections::HashSet::new();
    let all_keys: Vec<String> = flat.keys().cloned().collect();

    // First pass: plural groups
    for key in &all_keys {
        if consumed.contains(key) {
            continue;
        }
        if let Some((base, _cat)) = strip_plural_suffix(key) {
            let other_key = format!("{base}_other");
            if flat.contains_key(&other_key) && !consumed.contains(&other_key) {
                let ps = collect_plural(base, flat, &mut consumed);
                let comments = build_comments_for_key(base, source_text, line_comments);
                entries.insert(
                    base.to_string(),
                    I18nEntry {
                        key: base.to_string(),
                        value: EntryValue::Plural(ps),
                        comments,
                        ..Default::default()
                    },
                );
            }
        }
    }

    // Second pass: simple entries
    for (key, value) in flat {
        if consumed.contains(key) {
            continue;
        }
        let comments = build_comments_for_key(key, source_text, line_comments);
        entries.insert(
            key.clone(),
            I18nEntry {
                key: key.clone(),
                value: EntryValue::Simple(value.clone()),
                comments,
                ..Default::default()
            },
        );
    }

    entries
}

fn collect_plural(
    base: &str,
    flat: &IndexMap<String, String>,
    consumed: &mut std::collections::HashSet<String>,
) -> PluralSet {
    let mut ps = PluralSet::default();
    for &(suffix, category) in PLURAL_SUFFIXES {
        let full_key = format!("{base}{suffix}");
        if let Some(value) = flat.get(&full_key) {
            consumed.insert(full_key);
            match category {
                "zero" => ps.zero = Some(value.clone()),
                "one" => ps.one = Some(value.clone()),
                "two" => ps.two = Some(value.clone()),
                "few" => ps.few = Some(value.clone()),
                "many" => ps.many = Some(value.clone()),
                "other" => ps.other = value.clone(),
                _ => {}
            }
        }
    }
    ps
}

fn build_comments_for_key(
    key: &str,
    source_text: &str,
    line_comments: &[LineComment],
) -> Vec<Comment> {
    let leaf = key.rsplit('.').next().unwrap_or(key);
    if let Some(line_no) = find_key_line(source_text, leaf) {
        let texts = find_comments_for_line(line_comments, line_no);
        texts
            .into_iter()
            .filter(|t| !t.is_empty())
            .map(|t| Comment {
                text: t,
                role: CommentRole::Translator,
                priority: None,
                annotates: None,
            })
            .collect()
    } else {
        Vec::new()
    }
}

// ---------------------------------------------------------------------------
// Unflatten dot-separated keys to nested JSON
// ---------------------------------------------------------------------------

fn unflatten_to_json(flat: &IndexMap<String, String>) -> serde_json::Value {
    let mut root = serde_json::Map::new();
    for (key, value) in flat {
        let parts: Vec<&str> = key.split('.').collect();
        insert_nested(&mut root, &parts, value);
    }
    serde_json::Value::Object(root)
}

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

// ---------------------------------------------------------------------------
// Writer helpers
// ---------------------------------------------------------------------------

fn expand_entries_to_flat(entries: &IndexMap<String, I18nEntry>) -> IndexMap<String, String> {
    let mut flat = IndexMap::new();
    for (key, entry) in entries {
        match &entry.value {
            EntryValue::Simple(s) => {
                flat.insert(key.clone(), s.clone());
            }
            EntryValue::Plural(ps) => {
                if let Some(ref v) = ps.zero {
                    flat.insert(format!("{key}_zero"), v.clone());
                }
                if let Some(ref v) = ps.one {
                    flat.insert(format!("{key}_one"), v.clone());
                }
                if let Some(ref v) = ps.two {
                    flat.insert(format!("{key}_two"), v.clone());
                }
                if let Some(ref v) = ps.few {
                    flat.insert(format!("{key}_few"), v.clone());
                }
                if let Some(ref v) = ps.many {
                    flat.insert(format!("{key}_many"), v.clone());
                }
                flat.insert(format!("{key}_other"), ps.other.clone());
            }
            EntryValue::Array(arr) => {
                let json_arr =
                    serde_json::to_string(arr).unwrap_or_else(|_| "[]".to_string());
                flat.insert(key.clone(), json_arr);
            }
            EntryValue::Select(ss) => {
                let val = ss.cases.get("other").cloned().unwrap_or_default();
                flat.insert(key.clone(), val);
            }
            EntryValue::MultiVariablePlural(mvp) => {
                flat.insert(key.clone(), mvp.pattern.clone());
            }
        }
    }
    flat
}

static RE_VALID_JS_IDENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[a-zA-Z_$][a-zA-Z0-9_$]*$").expect("valid regex")
});

fn is_valid_js_identifier(s: &str) -> bool {
    RE_VALID_JS_IDENT.is_match(s)
}

fn escape_js_string(s: &str, quote: char) -> String {
    let mut result = String::with_capacity(s.len());
    for ch in s.chars() {
        if ch == quote {
            result.push('\\');
            result.push(ch);
        } else if ch == '\\' {
            result.push('\\');
            result.push('\\');
        } else if ch == '\n' {
            result.push('\\');
            result.push('n');
        } else if ch == '\r' {
            result.push('\\');
            result.push('r');
        } else if ch == '\t' {
            result.push('\\');
            result.push('t');
        } else {
            result.push(ch);
        }
    }
    result
}

fn json_to_js_object(
    value: &serde_json::Value,
    indent: usize,
    quote: char,
) -> String {
    let indent_str = "  ".repeat(indent);
    let inner_indent = "  ".repeat(indent + 1);

    match value {
        serde_json::Value::Object(map) => {
            if map.is_empty() {
                return "{}".to_string();
            }
            let mut lines = Vec::new();
            lines.push("{".to_string());
            let entries: Vec<_> = map.iter().collect();
            for (i, (key, val)) in entries.iter().enumerate() {
                let comma = if i < entries.len() - 1 { "," } else { "" };
                let formatted_val = json_to_js_object(val, indent + 1, quote);
                let key_str = if is_valid_js_identifier(key) {
                    key.to_string()
                } else {
                    format!("{quote}{key}{quote}")
                };
                lines.push(format!("{inner_indent}{key_str}: {formatted_val}{comma}"));
            }
            lines.push(format!("{indent_str}}}"));
            lines.join("\n")
        }
        serde_json::Value::String(s) => {
            let escaped = escape_js_string(s, quote);
            format!("{quote}{escaped}{quote}")
        }
        serde_json::Value::Array(arr) => {
            if arr.is_empty() {
                return "[]".to_string();
            }
            let items: Vec<String> = arr
                .iter()
                .map(|v| json_to_js_object(v, indent + 1, quote))
                .collect();
            format!("[{}]", items.join(", "))
        }
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".to_string(),
    }
}

fn write_js_object_with_comments(
    value: &serde_json::Value,
    entries: &IndexMap<String, I18nEntry>,
    indent: usize,
    quote: char,
) -> String {
    write_js_object_inner(value, entries, indent, quote, "")
}

fn write_js_object_inner(
    value: &serde_json::Value,
    entries: &IndexMap<String, I18nEntry>,
    indent: usize,
    quote: char,
    key_prefix: &str,
) -> String {
    let indent_str = "  ".repeat(indent);
    let inner_indent = "  ".repeat(indent + 1);

    match value {
        serde_json::Value::Object(map) => {
            if map.is_empty() {
                return "{}".to_string();
            }
            let mut lines = Vec::new();
            lines.push("{".to_string());
            let map_entries: Vec<_> = map.iter().collect();
            for (i, (key, val)) in map_entries.iter().enumerate() {
                let full_key = if key_prefix.is_empty() {
                    key.to_string()
                } else {
                    format!("{key_prefix}.{key}")
                };

                let comma = if i < map_entries.len() - 1 { "," } else { "" };

                if let Some(entry) = entries.get(&full_key) {
                    for c in &entry.comments {
                        if !c.text.is_empty() {
                            lines.push(format!("{inner_indent}// {}", c.text));
                        }
                    }
                }

                let key_str = if is_valid_js_identifier(key) {
                    key.to_string()
                } else {
                    format!("{quote}{key}{quote}")
                };

                if val.is_object() {
                    let nested_str =
                        write_js_object_inner(val, entries, indent + 1, quote, &full_key);
                    lines.push(format!("{inner_indent}{key_str}: {nested_str}{comma}"));
                } else {
                    let formatted_val = json_to_js_object(val, indent + 1, quote);
                    lines.push(format!("{inner_indent}{key_str}: {formatted_val}{comma}"));
                }
            }
            lines.push(format!("{indent_str}}}"));
            lines.join("\n")
        }
        other => json_to_js_object(other, indent, quote),
    }
}

// ---------------------------------------------------------------------------
// Trait implementations
// ---------------------------------------------------------------------------

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        if extension != ".ts" {
            return Confidence::None;
        }
        // .ts is shared with Qt Linguist (XML). Check if content looks like JS/TS, not XML.
        let text = String::from_utf8_lossy(content);
        if text.contains("export default") || text.contains("export const") {
            Confidence::High
        } else if text.trim_start().starts_with('<') {
            // Looks like XML (Qt Linguist), not TypeScript
            Confidence::None
        } else {
            Confidence::Low
        }
    }

    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError> {
        let text = std::str::from_utf8(content)
            .map_err(|e| ParseError::InvalidFormat(format!("Invalid UTF-8: {e}")))?;

        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Ok(I18nResource {
                metadata: ResourceMetadata {
                    source_format: FormatId::TypeScript,
                    format_ext: Some(FormatExtension::TypeScript(TypeScriptExt::default())),
                    ..Default::default()
                },
                entries: IndexMap::new(),
            });
        }

        let export_style = detect_export_style(text);
        let type_annotation = detect_type_annotation(text);
        let quote_style = detect_quote_style(text);

        let line_comments = extract_line_comments(text);

        let obj_literal = extract_object_literal(text).ok_or_else(|| {
            ParseError::InvalidFormat("No object literal found in TypeScript source".to_string())
        })?;

        let cleaned = clean_object_for_json(obj_literal);
        let json_value: serde_json::Value = serde_json::from_str(&cleaned)
            .map_err(|e| ParseError::Json(format!("Failed to parse TS object as JSON: {e}")))?;

        let obj = json_value.as_object().ok_or_else(|| {
            ParseError::InvalidFormat("Root value is not an object".to_string())
        })?;

        let mut flat = IndexMap::new();
        flatten_json_object(obj, "", &mut flat);

        let entries = group_entries(&flat, text, &line_comments);

        // Use detected quote style in the extension for reference, but don't store
        // it in TypeScriptExt (it doesn't have a quote_style field)
        let _ = quote_style;

        Ok(I18nResource {
            metadata: ResourceMetadata {
                source_format: FormatId::TypeScript,
                format_ext: Some(FormatExtension::TypeScript(TypeScriptExt {
                    export_style,
                    type_annotation,
                })),
                ..Default::default()
            },
            entries,
        })
    }

    fn capabilities(&self) -> FormatCapabilities {
        FormatCapabilities {
            plurals: true,
            arrays: false,
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
        let (export_style, type_annotation) = match &resource.metadata.format_ext {
            Some(FormatExtension::TypeScript(ext)) => {
                let style = ext
                    .export_style
                    .clone()
                    .unwrap_or_else(|| "export default".to_string());
                let annotation = ext.type_annotation.clone();
                (style, annotation)
            }
            _ => ("export default".to_string(), None),
        };

        let quote = '"';

        let flat = expand_entries_to_flat(&resource.entries);
        let nested = unflatten_to_json(&flat);

        let inner_output =
            write_js_object_with_comments(&nested, &resource.entries, 0, quote);

        let mut output = String::new();

        match export_style.as_str() {
            "export const" => {
                if let Some(ref ann) = type_annotation {
                    output.push_str(&format!("const messages: {ann} = {inner_output};\n\nexport default messages;\n"));
                } else {
                    output.push_str(&format!("export const messages = {inner_output};\n"));
                }
            }
            _ => {
                // "export default" is the standard
                output.push_str(&format!("export default {inner_output};\n"));
            }
        }

        Ok(output.into_bytes())
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

    #[test]
    fn test_strip_plural_suffix() {
        assert_eq!(strip_plural_suffix("items_one"), Some(("items", "one")));
        assert_eq!(strip_plural_suffix("items_other"), Some(("items", "other")));
        assert_eq!(strip_plural_suffix("items"), None);
        assert_eq!(strip_plural_suffix("_other"), None);
    }

    #[test]
    fn test_detect_export_style() {
        assert_eq!(
            detect_export_style("export default {};"),
            Some("export default".to_string())
        );
        assert_eq!(
            detect_export_style("export const messages = {};"),
            Some("export const".to_string())
        );
        assert_eq!(detect_export_style("var x = {};"), None);
    }

    #[test]
    fn test_detect_type_annotation() {
        assert_eq!(
            detect_type_annotation("const messages: Record<string, string> = {}"),
            Some("Record<string, string>".to_string())
        );
        assert_eq!(detect_type_annotation("export default {}"), None);
    }

    #[test]
    fn test_extract_object_literal() {
        assert_eq!(
            extract_object_literal("export default { a: 1 };"),
            Some("{ a: 1 }")
        );
    }

    #[test]
    fn test_clean_object_single_quotes() {
        let cleaned = clean_object_for_json("{ greeting: 'Hello' }");
        let parsed: serde_json::Value = serde_json::from_str(&cleaned).unwrap();
        assert_eq!(parsed["greeting"], "Hello");
    }

    #[test]
    fn test_clean_object_trailing_comma() {
        let cleaned = clean_object_for_json("{ greeting: \"Hello\", }");
        let parsed: serde_json::Value = serde_json::from_str(&cleaned).unwrap();
        assert_eq!(parsed["greeting"], "Hello");
    }
}
