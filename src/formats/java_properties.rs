use super::*;
use indexmap::IndexMap;

pub struct Parser;
pub struct Writer;

// ---------------------------------------------------------------------------
// Parser helpers
// ---------------------------------------------------------------------------

/// Unescape a Java .properties value/key.
/// Handles \n, \t, \\, \=, \:, \#, \!, \<space>, and \uXXXX.
fn properties_unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.peek().copied() {
                Some('n') => {
                    chars.next();
                    out.push('\n');
                }
                Some('t') => {
                    chars.next();
                    out.push('\t');
                }
                Some('\\') => {
                    chars.next();
                    out.push('\\');
                }
                Some('=') => {
                    chars.next();
                    out.push('=');
                }
                Some(':') => {
                    chars.next();
                    out.push(':');
                }
                Some('#') => {
                    chars.next();
                    out.push('#');
                }
                Some('!') => {
                    chars.next();
                    out.push('!');
                }
                Some(' ') => {
                    chars.next();
                    out.push(' ');
                }
                Some('u') => {
                    chars.next(); // consume 'u'
                    let mut hex = String::with_capacity(4);
                    for _ in 0..4 {
                        match chars.peek() {
                            Some(&c) if c.is_ascii_hexdigit() => {
                                hex.push(c);
                                chars.next();
                            }
                            _ => break,
                        }
                    }
                    if hex.len() == 4 {
                        if let Ok(code) = u32::from_str_radix(&hex, 16) {
                            if let Some(unicode_char) = char::from_u32(code) {
                                out.push(unicode_char);
                            } else {
                                out.push_str("\\u");
                                out.push_str(&hex);
                            }
                        } else {
                            out.push_str("\\u");
                            out.push_str(&hex);
                        }
                    } else {
                        // Not enough hex digits, emit literally
                        out.push_str("\\u");
                        out.push_str(&hex);
                    }
                }
                Some(other) => {
                    chars.next();
                    out.push('\\');
                    out.push(other);
                }
                None => {
                    // Trailing backslash (not a continuation - that's handled before this)
                    out.push('\\');
                }
            }
        } else {
            out.push(ch);
        }
    }
    out
}

/// Escape a key for Java .properties output.
/// Escapes spaces, =, :, #, !, \, newlines, tabs.
fn escape_key(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            ' ' => out.push_str("\\ "),
            '=' => out.push_str("\\="),
            ':' => out.push_str("\\:"),
            '#' => out.push_str("\\#"),
            '!' => out.push_str("\\!"),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out
}

/// Escape a value for Java .properties output.
/// Escapes \, newlines, tabs.  Leading spaces on the first line are escaped.
fn escape_value(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut at_start = true;
    for ch in s.chars() {
        match ch {
            '\\' => {
                out.push_str("\\\\");
                at_start = false;
            }
            '\n' => {
                out.push_str("\\n");
                at_start = false;
            }
            '\t' => {
                out.push_str("\\t");
                at_start = false;
            }
            ' ' if at_start => {
                out.push_str("\\ ");
            }
            other => {
                out.push(other);
                at_start = false;
            }
        }
    }
    out
}

/// Join logical lines: handle continuation lines (trailing backslash).
/// Returns a Vec of logical lines with continuation lines joined and leading
/// whitespace of continuation lines stripped.
fn join_logical_lines(content: &str) -> Vec<String> {
    let mut logical_lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut continuing = false;

    for line in content.lines() {
        if continuing {
            // Strip leading whitespace of continuation line
            current.push_str(line.trim_start());
        } else {
            if !current.is_empty() || logical_lines.is_empty() {
                // Don't push the initial empty buffer
            }
            current = line.to_string();
        }

        // Check if line ends with an odd number of backslashes (continuation)
        let trimmed_end = current.as_str();
        let trailing_backslashes = trimmed_end.chars().rev().take_while(|&c| c == '\\').count();
        if trailing_backslashes % 2 == 1 {
            // Remove the trailing continuation backslash
            current.pop();
            continuing = true;
        } else {
            continuing = false;
            logical_lines.push(current.clone());
            current = String::new();
        }
    }

    // Handle last line if still continuing
    if !current.is_empty() {
        logical_lines.push(current);
    }

    logical_lines
}

/// Detect the separator character (=, :, or whitespace) from a logical line.
/// Returns (key_raw, value_raw, separator_char).
/// The key and value are raw (still escaped).
fn split_key_value(line: &str) -> Option<(String, String, char)> {
    let trimmed = line.trim_start();
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('!') {
        return None;
    }

    let mut key = String::new();
    let mut chars = trimmed.chars().peekable();
    let mut separator = ' ';

    // Parse the key: consume characters, handling escapes
    while let Some(&ch) = chars.peek() {
        match ch {
            '\\' => {
                key.push(ch);
                chars.next();
                // Push the escaped character too (it's part of the raw key)
                if let Some(&next) = chars.peek() {
                    key.push(next);
                    chars.next();
                }
            }
            '=' | ':' => {
                separator = ch;
                chars.next();
                break;
            }
            ' ' | '\t' => {
                // Whitespace ends the key; check if followed by = or :
                // Skip whitespace
                while let Some(&ws) = chars.peek() {
                    if ws == ' ' || ws == '\t' {
                        chars.next();
                    } else {
                        break;
                    }
                }
                // Check if the next char is = or :
                if let Some(&next) = chars.peek() {
                    if next == '=' || next == ':' {
                        separator = next;
                        chars.next();
                    } else {
                        separator = ' ';
                    }
                } else {
                    separator = ' ';
                }
                break;
            }
            _ => {
                key.push(ch);
                chars.next();
            }
        }
    }

    if key.is_empty() {
        return None;
    }

    // Skip leading whitespace of the value
    while let Some(&ch) = chars.peek() {
        if ch == ' ' || ch == '\t' {
            chars.next();
        } else {
            break;
        }
    }

    let value: String = chars.collect();

    Some((key, value, separator))
}

/// Parse Java .properties content into an I18nResource.
fn parse_properties(content: &str) -> Result<I18nResource, ParseError> {
    let logical_lines = join_logical_lines(content);
    let mut entries = IndexMap::new();
    let mut pending_comments: Vec<(String, char)> = Vec::new();

    // Track the first separator and comment char seen for resource-level metadata
    let mut first_separator: Option<char> = None;
    let mut first_comment_char: Option<char> = None;

    for line in &logical_lines {
        let trimmed = line.trim_start();

        // Blank lines reset pending comments
        if trimmed.is_empty() {
            pending_comments.clear();
            continue;
        }

        // Comment lines
        if trimmed.starts_with('#') || trimmed.starts_with('!') {
            let comment_ch = trimmed.chars().next().expect("non-empty trimmed string");
            if first_comment_char.is_none() {
                first_comment_char = Some(comment_ch);
            }
            // Strip the comment char and optional leading space
            let text = &trimmed[1..];
            let text = if let Some(stripped) = text.strip_prefix(' ') {
                stripped
            } else {
                text
            };
            pending_comments.push((text.to_string(), comment_ch));
            continue;
        }

        // Key-value line
        if let Some((raw_key, raw_value, sep)) = split_key_value(trimmed) {
            if first_separator.is_none() {
                first_separator = Some(sep);
            }

            let key = properties_unescape(&raw_key);
            let value = properties_unescape(&raw_value);

            let mut entry = I18nEntry {
                key: key.clone(),
                value: EntryValue::Simple(value),
                format_ext: Some(FormatExtension::JavaProperties(JavaPropertiesExt {
                    separator: Some(sep),
                    comment_char: pending_comments.first().map(|(_, ch)| *ch),
                })),
                ..Default::default()
            };

            for (comment_text, _) in &pending_comments {
                if !comment_text.is_empty() {
                    entry.comments.push(Comment {
                        text: comment_text.clone(),
                        role: CommentRole::General,
                        priority: None,
                        annotates: None,
                    });
                }
            }
            pending_comments.clear();

            entries.insert(key, entry);
        }
    }

    Ok(I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::JavaProperties,
            format_ext: Some(FormatExtension::JavaProperties(JavaPropertiesExt {
                separator: first_separator,
                comment_char: first_comment_char,
            })),
            ..Default::default()
        },
        entries,
    })
}

// ---------------------------------------------------------------------------
// Writer
// ---------------------------------------------------------------------------

fn write_properties(resource: &I18nResource) -> String {
    let mut out = String::new();

    // Determine default separator from resource-level extension
    let default_sep = match &resource.metadata.format_ext {
        Some(FormatExtension::JavaProperties(ext)) => ext.separator.unwrap_or('='),
        _ => '=',
    };

    let mut first = true;
    for (_key, entry) in &resource.entries {
        if !first {
            out.push('\n');
        }
        first = false;

        // Determine per-entry separator
        let sep = match &entry.format_ext {
            Some(FormatExtension::JavaProperties(ext)) => ext.separator.unwrap_or(default_sep),
            _ => default_sep,
        };

        // Write comments
        for comment in &entry.comments {
            out.push_str(&format!("# {}\n", comment.text));
        }

        // Write key = value
        let value_str = match &entry.value {
            EntryValue::Simple(s) => s.clone(),
            EntryValue::Plural(ps) => ps.other.clone(),
            EntryValue::Array(arr) => arr.join("\n"),
            EntryValue::Select(ss) => ss.cases.values().next().cloned().unwrap_or_default(),
            EntryValue::MultiVariablePlural(mvp) => mvp.pattern.clone(),
        };

        let sep_str = if sep == ' ' {
            " ".to_string()
        } else {
            format!(" {sep} ")
        };

        out.push_str(&format!(
            "{}{}{}\n",
            escape_key(&entry.key),
            sep_str,
            escape_value(&value_str)
        ));
    }

    out
}

// ---------------------------------------------------------------------------
// Trait implementations
// ---------------------------------------------------------------------------

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        if extension == ".properties" {
            return Confidence::Definite;
        }
        if let Ok(s) = std::str::from_utf8(content) {
            // Heuristic: look for key=value or key:value patterns
            let lines: Vec<&str> = s.lines().collect();
            let mut kv_count = 0;
            for line in &lines {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('!') {
                    continue;
                }
                if trimmed.contains('=') || trimmed.contains(':') {
                    kv_count += 1;
                }
            }
            if kv_count >= 2 {
                return Confidence::Low;
            }
        }
        Confidence::None
    }

    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError> {
        let text = std::str::from_utf8(content)
            .map_err(|e| ParseError::InvalidFormat(format!("Invalid UTF-8: {e}")))?;
        parse_properties(text)
    }

    fn capabilities(&self) -> FormatCapabilities {
        FormatCapabilities {
            plurals: false,
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
        let output = write_properties(resource);
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
    fn test_unescape_basic() {
        assert_eq!(properties_unescape("hello\\nworld"), "hello\nworld");
        assert_eq!(properties_unescape("hello\\tworld"), "hello\tworld");
        assert_eq!(properties_unescape("hello\\\\world"), "hello\\world");
        assert_eq!(properties_unescape("key\\=value"), "key=value");
        assert_eq!(properties_unescape("key\\:value"), "key:value");
        assert_eq!(properties_unescape("key\\ value"), "key value");
    }

    #[test]
    fn test_unescape_unicode() {
        assert_eq!(properties_unescape("Hello \\u0057orld"), "Hello World");
        assert_eq!(properties_unescape("caf\\u00E9"), "caf\u{00E9}");
    }

    #[test]
    fn test_escape_key() {
        assert_eq!(escape_key("key with spaces"), "key\\ with\\ spaces");
        assert_eq!(escape_key("key=value"), "key\\=value");
        assert_eq!(escape_key("key:value"), "key\\:value");
    }

    #[test]
    fn test_escape_value() {
        assert_eq!(escape_value("hello\nworld"), "hello\\nworld");
        assert_eq!(escape_value("hello\\world"), "hello\\\\world");
    }

    #[test]
    fn test_split_key_value_equals() {
        let result = split_key_value("greeting = Hello, World!");
        assert!(result.is_some());
        let (key, value, sep) = result.expect("should parse");
        assert_eq!(key, "greeting");
        assert_eq!(value, "Hello, World!");
        assert_eq!(sep, '=');
    }

    #[test]
    fn test_split_key_value_colon() {
        let result = split_key_value("farewell : Goodbye!");
        assert!(result.is_some());
        let (key, value, sep) = result.expect("should parse");
        assert_eq!(key, "farewell");
        assert_eq!(value, "Goodbye!");
        assert_eq!(sep, ':');
    }

    #[test]
    fn test_split_key_value_no_spaces() {
        let result = split_key_value("app.title=My Application");
        assert!(result.is_some());
        let (key, value, sep) = result.expect("should parse");
        assert_eq!(key, "app.title");
        assert_eq!(value, "My Application");
        assert_eq!(sep, '=');
    }

    #[test]
    fn test_join_logical_lines_continuation() {
        let input =
            "multiline.value = This is a \\\n    long value that spans \\\n    multiple lines";
        let lines = join_logical_lines(input);
        assert_eq!(lines.len(), 1);
        assert_eq!(
            lines[0],
            "multiline.value = This is a long value that spans multiple lines"
        );
    }

    #[test]
    fn test_parse_comments() {
        let input = "# This is a comment\ngreeting = Hello";
        let resource = parse_properties(input).expect("should parse");
        let entry = resource
            .entries
            .get("greeting")
            .expect("greeting should exist");
        assert_eq!(entry.comments.len(), 1);
        assert_eq!(entry.comments[0].text, "This is a comment");
    }

    #[test]
    fn test_parse_exclamation_comment() {
        let input = "! This is also a comment\ngreeting = Hello";
        let resource = parse_properties(input).expect("should parse");
        let entry = resource
            .entries
            .get("greeting")
            .expect("greeting should exist");
        assert_eq!(entry.comments.len(), 1);
        assert_eq!(entry.comments[0].text, "This is also a comment");
    }
}
