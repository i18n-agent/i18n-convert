use super::*;

pub struct Parser;
pub struct Writer;

// ---------------------------------------------------------------------------
// Parser helpers
// ---------------------------------------------------------------------------

/// Unescape an iOS .strings value: handles \", \\, \n, \t, and \UXXXX.
fn unescape_strings_value(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.peek() {
                Some(&'"') => { chars.next(); out.push('"'); }
                Some(&'\\') => { chars.next(); out.push('\\'); }
                Some(&'n') => { chars.next(); out.push('\n'); }
                Some(&'t') => { chars.next(); out.push('\t'); }
                Some(&'U') | Some(&'u') => {
                    let u_char = *chars.peek().expect("already matched U/u");
                    chars.next(); // consume U/u
                    // Read exactly 4 hex digits (Apple .strings standard)
                    let mut hex = String::new();
                    for _ in 0..4 {
                        match chars.peek() {
                            Some(&c) if c.is_ascii_hexdigit() => {
                                hex.push(c);
                                chars.next();
                            }
                            _ => break,
                        }
                    }
                    // If we got exactly 4 digits and the value is 0000-001F
                    // (indicating an extended codepoint like \U0001F600),
                    // try to read 4 more hex digits for 8-digit form.
                    if hex.len() == 4 {
                        if let Ok(short_val) = u32::from_str_radix(&hex, 16) {
                            if short_val <= 0x001F {
                                // Likely an 8-digit Unicode escape
                                let mut ext_hex = String::new();
                                for _ in 0..4 {
                                    match chars.peek() {
                                        Some(&c) if c.is_ascii_hexdigit() => {
                                            ext_hex.push(c);
                                            chars.next();
                                        }
                                        _ => break,
                                    }
                                }
                                if !ext_hex.is_empty() {
                                    hex.push_str(&ext_hex);
                                }
                            }
                        }
                    }
                    if let Ok(code) = u32::from_str_radix(&hex, 16) {
                        if let Some(unicode_char) = char::from_u32(code) {
                            out.push(unicode_char);
                        } else {
                            out.push('\\');
                            out.push(u_char);
                            out.push_str(&hex);
                        }
                    } else {
                        out.push('\\');
                        out.push(u_char);
                        out.push_str(&hex);
                    }
                }
                Some(&other) => {
                    chars.next();
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(ch);
        }
    }
    out
}

/// Escape a string for iOS .strings output: escapes ", \, newlines, tabs.
fn escape_strings_value(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out
}

/// Parse .strings content into an I18nResource.
///
/// The format is line-based:
///   /* comment */
///   "key" = "value";
///
/// We handle:
/// - Block comments /* ... */ (potentially spanning multiple lines)
/// - Key-value pairs: "key" = "value";
/// - Blank lines
/// - // single-line comments (less common but valid)
fn parse_strings(content: &str) -> Result<I18nResource, ParseError> {
    let mut entries = IndexMap::new();
    let mut pending_comment: Option<String> = None;

    let mut chars = content.chars().peekable();

    loop {
        // Skip whitespace (spaces, tabs, newlines)
        while let Some(&ch) = chars.peek() {
            if ch.is_whitespace() {
                chars.next();
            } else {
                break;
            }
        }

        // Check if we've reached the end
        if chars.peek().is_none() {
            break;
        }

        let ch = *chars.peek().expect("checked is_none above");

        // Block comment: /* ... */
        if ch == '/' {
            chars.next();
            match chars.peek() {
                Some(&'*') => {
                    chars.next(); // consume '*'
                    let mut comment_text = String::new();
                    let mut found_end = false;
                    while let Some(c) = chars.next() {
                        if c == '*' {
                            if let Some(&'/') = chars.peek() {
                                chars.next(); // consume '/'
                                found_end = true;
                                break;
                            } else {
                                comment_text.push(c);
                            }
                        } else {
                            comment_text.push(c);
                        }
                    }
                    if !found_end {
                        return Err(ParseError::InvalidFormat(
                            "Unterminated block comment".to_string(),
                        ));
                    }
                    pending_comment = Some(comment_text.trim().to_string());
                    continue;
                }
                Some(&'/') => {
                    // Single-line comment: // ...
                    chars.next(); // consume second '/'
                    let mut comment_text = String::new();
                    while let Some(c) = chars.next() {
                        if c == '\n' {
                            break;
                        }
                        comment_text.push(c);
                    }
                    pending_comment = Some(comment_text.trim().to_string());
                    continue;
                }
                _ => {
                    return Err(ParseError::InvalidFormat(
                        "Unexpected '/' not followed by '*' or '/'".to_string(),
                    ));
                }
            }
        }

        // Key-value pair: "key" = "value";
        if ch == '"' {
            chars.next(); // consume opening quote
            let key = parse_quoted_string(&mut chars)?;

            // Skip whitespace
            skip_whitespace(&mut chars);

            // Expect '='
            match chars.next() {
                Some('=') => {}
                other => {
                    return Err(ParseError::InvalidFormat(format!(
                        "Expected '=' after key \"{}\", got {:?}",
                        key, other
                    )));
                }
            }

            // Skip whitespace
            skip_whitespace(&mut chars);

            // Expect '"'
            match chars.next() {
                Some('"') => {}
                other => {
                    return Err(ParseError::InvalidFormat(format!(
                        "Expected '\"' to start value for key \"{}\", got {:?}",
                        key, other
                    )));
                }
            }

            let raw_value = parse_quoted_string(&mut chars)?;

            // Skip whitespace
            skip_whitespace(&mut chars);

            // Expect ';'
            match chars.next() {
                Some(';') => {}
                other => {
                    return Err(ParseError::InvalidFormat(format!(
                        "Expected ';' after value for key \"{}\", got {:?}",
                        key, other
                    )));
                }
            }

            let value = unescape_strings_value(&raw_value);

            let mut entry = I18nEntry {
                key: key.clone(),
                value: EntryValue::Simple(value),
                ..Default::default()
            };

            if let Some(comment_text) = pending_comment.take() {
                if !comment_text.is_empty() {
                    entry.comments.push(Comment {
                        text: comment_text,
                        role: CommentRole::General,
                        priority: None,
                        annotates: None,
                    });
                }
            }

            entries.insert(key, entry);
            continue;
        }

        // Unexpected character
        return Err(ParseError::InvalidFormat(format!(
            "Unexpected character: '{}'",
            ch
        )));
    }

    Ok(I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::IosStrings,
            ..Default::default()
        },
        entries,
    })
}

/// Parse the content of a quoted string (after the opening `"`).
/// Returns the raw content (with escape sequences still present).
/// Consumes up to and including the closing `"`.
fn parse_quoted_string(
    chars: &mut std::iter::Peekable<std::str::Chars>,
) -> Result<String, ParseError> {
    let mut s = String::new();
    loop {
        match chars.next() {
            Some('\\') => {
                s.push('\\');
                // Push the next character as-is (part of the escape sequence)
                match chars.next() {
                    Some(c) => s.push(c),
                    None => {
                        return Err(ParseError::InvalidFormat(
                            "Unexpected end of input in escape sequence".to_string(),
                        ));
                    }
                }
            }
            Some('"') => {
                // End of quoted string
                return Ok(s);
            }
            Some(c) => s.push(c),
            None => {
                return Err(ParseError::InvalidFormat(
                    "Unterminated quoted string".to_string(),
                ));
            }
        }
    }
}

fn skip_whitespace(chars: &mut std::iter::Peekable<std::str::Chars>) {
    while let Some(&ch) = chars.peek() {
        if ch == ' ' || ch == '\t' {
            chars.next();
        } else {
            break;
        }
    }
}

// ---------------------------------------------------------------------------
// Writer helpers
// ---------------------------------------------------------------------------

fn write_strings(resource: &I18nResource) -> String {
    let mut out = String::new();
    let mut first = true;

    for (_key, entry) in &resource.entries {
        if !first {
            out.push('\n');
        }
        first = false;

        // Write comment if present
        for comment in &entry.comments {
            out.push_str(&format!("/* {} */\n", comment.text));
        }

        // Write key = value
        let value_str = match &entry.value {
            EntryValue::Simple(s) => s.clone(),
            EntryValue::Plural(ps) => {
                // .strings doesn't support plurals; fall back to "other"
                ps.other.clone()
            }
            EntryValue::Array(arr) => {
                // .strings doesn't support arrays; join with newlines
                arr.join("\n")
            }
            EntryValue::Select(ss) => {
                // Fall back to first case or empty
                ss.cases.values().next().cloned().unwrap_or_default()
            }
            EntryValue::MultiVariablePlural(mvp) => {
                mvp.pattern.clone()
            }
        };

        out.push_str(&format!(
            "\"{}\" = \"{}\";\n",
            escape_strings_value(&entry.key),
            escape_strings_value(&value_str)
        ));
    }

    out
}

// ---------------------------------------------------------------------------
// Trait implementations
// ---------------------------------------------------------------------------

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        if extension == ".strings" {
            return Confidence::Definite;
        }
        if let Ok(s) = std::str::from_utf8(content) {
            // Pattern: "key" = "value";
            if s.contains("\" = \"") && s.contains(';') {
                return Confidence::High;
            }
        }
        Confidence::None
    }

    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError> {
        let text = std::str::from_utf8(content)
            .map_err(|e| ParseError::InvalidFormat(format!("Invalid UTF-8: {}", e)))?;
        parse_strings(text)
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
            nested_keys: false,
            inline_markup: false,
            alternatives: false,
            source_references: false,
            custom_properties: false,
        }
    }
}

impl FormatWriter for Writer {
    fn write(&self, resource: &I18nResource) -> Result<Vec<u8>, WriteError> {
        let output = write_strings(resource);
        Ok(output.into_bytes())
    }

    fn capabilities(&self) -> FormatCapabilities {
        Parser.capabilities()
    }
}
