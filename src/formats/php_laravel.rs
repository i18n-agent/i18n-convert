use super::*;

pub struct Parser;
pub struct Writer;

// ---------------------------------------------------------------------------
// Parser helpers
// ---------------------------------------------------------------------------

/// Unescape a PHP single-quoted string: only \' and \\ are escape sequences.
fn unescape_single_quoted(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.peek() {
                Some(&'\'') => {
                    chars.next();
                    out.push('\'');
                }
                Some(&'\\') => {
                    chars.next();
                    out.push('\\');
                }
                _ => {
                    // In PHP single-quoted strings, other backslashes are literal
                    out.push('\\');
                }
            }
        } else {
            out.push(ch);
        }
    }
    out
}

/// Unescape a PHP double-quoted string: handles \", \\, \n, \t, \r, \$, etc.
fn unescape_double_quoted(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.peek() {
                Some(&'"') => {
                    chars.next();
                    out.push('"');
                }
                Some(&'\\') => {
                    chars.next();
                    out.push('\\');
                }
                Some(&'n') => {
                    chars.next();
                    out.push('\n');
                }
                Some(&'t') => {
                    chars.next();
                    out.push('\t');
                }
                Some(&'r') => {
                    chars.next();
                    out.push('\r');
                }
                Some(&'$') => {
                    chars.next();
                    out.push('$');
                }
                _ => {
                    out.push('\\');
                }
            }
        } else {
            out.push(ch);
        }
    }
    out
}

/// Escape a string value for PHP single-quoted output.
fn escape_single_quoted(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\'' => out.push_str("\\'"),
            '\\' => out.push_str("\\\\"),
            other => out.push(other),
        }
    }
    out
}

/// A parsed token from the PHP array content.
#[derive(Debug)]
enum PhpToken {
    /// A key-value pair with optional preceding comment
    Entry {
        key: String,
        value: String,
        comment: Option<String>,
    },
    /// A nested array: 'key' => [ ... ]
    NestedArray {
        key: String,
        entries: Vec<PhpToken>,
        comment: Option<String>,
    },
}

/// Skip whitespace characters in the char iterator.
fn skip_ws(chars: &mut std::iter::Peekable<std::str::Chars>) {
    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() {
            chars.next();
        } else {
            break;
        }
    }
}

/// Parse a PHP quoted string (single or double quoted).
/// The opening quote should already be consumed; `quote_char` indicates which.
fn parse_php_string(
    chars: &mut std::iter::Peekable<std::str::Chars>,
    quote_char: char,
) -> Result<String, ParseError> {
    let mut raw = String::new();
    loop {
        match chars.next() {
            Some('\\') => {
                raw.push('\\');
                match chars.next() {
                    Some(c) => raw.push(c),
                    None => {
                        return Err(ParseError::InvalidFormat(
                            "Unexpected end of input in escape sequence".to_string(),
                        ));
                    }
                }
            }
            Some(c) if c == quote_char => {
                // End of string
                return if quote_char == '\'' {
                    Ok(unescape_single_quoted(&raw))
                } else {
                    Ok(unescape_double_quoted(&raw))
                };
            }
            Some(c) => raw.push(c),
            None => {
                return Err(ParseError::InvalidFormat(
                    "Unterminated string literal".to_string(),
                ));
            }
        }
    }
}

/// Try to consume and return a comment (// or /* */) at the current position.
/// Returns None if the next character is not the start of a comment.
fn try_parse_comment(
    chars: &mut std::iter::Peekable<std::str::Chars>,
) -> Result<Option<String>, ParseError> {
    if chars.peek() != Some(&'/') {
        return Ok(None);
    }

    // We need to peek ahead two chars. Clone the iterator to check.
    let mut lookahead = chars.clone();
    lookahead.next(); // consume '/'
    match lookahead.peek() {
        Some(&'/') => {
            // Single-line comment
            chars.next(); // consume '/'
            chars.next(); // consume '/'
            let mut text = String::new();
            for c in chars.by_ref() {
                if c == '\n' {
                    break;
                }
                text.push(c);
            }
            Ok(Some(text.trim().to_string()))
        }
        Some(&'*') => {
            // Block comment
            chars.next(); // consume '/'
            chars.next(); // consume '*'
            let mut text = String::new();
            let mut found_end = false;
            while let Some(c) = chars.next() {
                if c == '*' {
                    if chars.peek() == Some(&'/') {
                        chars.next(); // consume '/'
                        found_end = true;
                        break;
                    } else {
                        text.push(c);
                    }
                } else {
                    text.push(c);
                }
            }
            if !found_end {
                return Err(ParseError::InvalidFormat(
                    "Unterminated block comment".to_string(),
                ));
            }
            Ok(Some(text.trim().to_string()))
        }
        _ => Ok(None),
    }
}

/// Parse the contents of a PHP array (between `[` and `]` or `array(` and `)`).
/// Returns a list of PhpTokens representing keys, values, and nested arrays.
fn parse_php_array_contents(
    chars: &mut std::iter::Peekable<std::str::Chars>,
    closing: char, // ']' or ')'
) -> Result<Vec<PhpToken>, ParseError> {
    let mut tokens = Vec::new();
    let mut pending_comment: Option<String> = None;

    loop {
        skip_ws(chars);

        match chars.peek() {
            None => {
                return Err(ParseError::InvalidFormat(format!(
                    "Unexpected end of input, expected '{closing}'"
                )));
            }
            Some(&c) if c == closing => {
                chars.next(); // consume closing bracket/paren
                return Ok(tokens);
            }
            Some(&'/') => {
                if let Some(comment) = try_parse_comment(chars)? {
                    pending_comment = Some(comment);
                    continue;
                } else {
                    return Err(ParseError::InvalidFormat(
                        "Unexpected '/' in array".to_string(),
                    ));
                }
            }
            Some(&'\'' | &'"') => {
                let quote = chars.next().expect("peeked successfully");
                let key = parse_php_string(chars, quote)?;

                skip_ws(chars);

                // Expect '=>'
                match (chars.next(), chars.next()) {
                    (Some('='), Some('>')) => {}
                    _ => {
                        return Err(ParseError::InvalidFormat(format!(
                            "Expected '=>' after key '{key}'"
                        )));
                    }
                }

                skip_ws(chars);

                // Check if value is a nested array or a string
                match chars.peek() {
                    Some(&'[') => {
                        chars.next(); // consume '['
                        let nested = parse_php_array_contents(chars, ']')?;
                        tokens.push(PhpToken::NestedArray {
                            key,
                            entries: nested,
                            comment: pending_comment.take(),
                        });
                    }
                    Some(&'a') => {
                        // Could be array(...)
                        let mut lookahead = chars.clone();
                        let word: String = lookahead.by_ref().take(5).collect();
                        if word == "array" {
                            // Check for '('
                            skip_ws(&mut lookahead);
                            if lookahead.peek() == Some(&'(') {
                                // Consume "array(" from the real iterator
                                for _ in 0..5 {
                                    chars.next();
                                }
                                skip_ws(chars);
                                if chars.peek() == Some(&'(') {
                                    chars.next();
                                }
                                let nested = parse_php_array_contents(chars, ')')?;
                                tokens.push(PhpToken::NestedArray {
                                    key,
                                    entries: nested,
                                    comment: pending_comment.take(),
                                });
                            } else {
                                return Err(ParseError::InvalidFormat(
                                    "Expected '(' after 'array'".to_string(),
                                ));
                            }
                        } else {
                            return Err(ParseError::InvalidFormat(format!(
                                "Unexpected token after '=>' for key '{key}'"
                            )));
                        }
                    }
                    Some(&'\'' | &'"') => {
                        let vquote = chars.next().expect("peeked successfully");
                        let value = parse_php_string(chars, vquote)?;
                        tokens.push(PhpToken::Entry {
                            key,
                            value,
                            comment: pending_comment.take(),
                        });
                    }
                    other => {
                        return Err(ParseError::InvalidFormat(format!(
                            "Unexpected character {other:?} after '=>' for key '{key}'"
                        )));
                    }
                }

                skip_ws(chars);

                // Optional trailing comma
                if chars.peek() == Some(&',') {
                    chars.next();
                }
            }
            Some(&c) => {
                return Err(ParseError::InvalidFormat(format!(
                    "Unexpected character '{c}' in PHP array"
                )));
            }
        }
    }
}

/// Flatten PhpTokens into IR entries with dot-separated keys.
fn flatten_tokens(tokens: &[PhpToken], prefix: &str, entries: &mut IndexMap<String, I18nEntry>) {
    for token in tokens {
        match token {
            PhpToken::Entry {
                key,
                value,
                comment,
            } => {
                let full_key = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                let mut entry = I18nEntry {
                    key: full_key.clone(),
                    value: EntryValue::Simple(value.clone()),
                    ..Default::default()
                };
                if let Some(comment_text) = comment {
                    if !comment_text.is_empty() {
                        entry.comments.push(Comment {
                            text: comment_text.clone(),
                            role: CommentRole::General,
                            priority: None,
                            annotates: None,
                        });
                    }
                }
                entries.insert(full_key, entry);
            }
            PhpToken::NestedArray {
                key,
                entries: nested,
                comment,
            } => {
                let new_prefix = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                // If there's a comment on the nested array itself, we attach it
                // to the first entry inside (if any).
                let _ = comment; // Comments on groups are not directly representable in flat IR
                flatten_tokens(nested, &new_prefix, entries);
            }
        }
    }
}

/// Detect the quote style used for keys by scanning for the first `'=>` pattern.
fn detect_key_quote_style(content: &str) -> Option<String> {
    // Find `'=>` or `"=>` (with optional space before `=>`) to detect key quoting
    let bytes = content.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'=' && i + 1 < bytes.len() && bytes[i + 1] == b'>' {
            // Walk backwards past whitespace to find the closing quote
            let mut j = i;
            while j > 0 {
                j -= 1;
                let c = bytes[j];
                if c == b'\'' {
                    return Some("single".to_string());
                } else if c == b'"' {
                    return Some("double".to_string());
                } else if c == b' ' || c == b'\t' {
                    continue;
                } else {
                    break;
                }
            }
        }
    }
    Some("single".to_string())
}

/// Parse the full PHP file content into an I18nResource.
fn parse_php(content: &str) -> Result<I18nResource, ParseError> {
    let trimmed = content.trim();

    // Find `return [` or `return array(`
    let array_start = if let Some(pos) = trimmed.find("return [") {
        let after_return = pos + "return [".len();
        let remaining = &trimmed[after_return..];
        let mut chars = remaining.chars().peekable();

        parse_php_array_contents(&mut chars, ']')?
    } else if let Some(pos) = trimmed.find("return array(") {
        let after_return = pos + "return array(".len();
        let remaining = &trimmed[after_return..];
        let mut chars = remaining.chars().peekable();

        parse_php_array_contents(&mut chars, ')')?
    } else {
        return Err(ParseError::InvalidFormat(
            "PHP file must contain 'return [' or 'return array('".to_string(),
        ));
    };

    let mut entries = IndexMap::new();
    flatten_tokens(&array_start, "", &mut entries);

    // Detect quote style by checking the first key quote character in the source
    let quote_style = detect_key_quote_style(content);

    Ok(I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::PhpLaravel,
            format_ext: Some(FormatExtension::PhpLaravel(PhpLaravelExt { quote_style })),
            ..Default::default()
        },
        entries,
    })
}

// ---------------------------------------------------------------------------
// Writer helpers
// ---------------------------------------------------------------------------

/// Escape a string value for PHP double-quoted output.
fn escape_double_quoted(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '$' => out.push_str("\\$"),
            other => out.push(other),
        }
    }
    out
}

/// Build nested structure from dot-separated keys, then write PHP array syntax.
fn write_php(resource: &I18nResource) -> String {
    let use_double = matches!(
        &resource.metadata.format_ext,
        Some(FormatExtension::PhpLaravel(PhpLaravelExt { quote_style: Some(ref qs) })) if qs == "double"
    );

    let mut out = String::new();
    out.push_str("<?php\n\nreturn [\n");

    // Build a tree structure from dot-separated keys
    let tree = build_tree(&resource.entries);
    write_tree_entries(&tree, &resource.entries, &mut out, 1, use_double);

    out.push_str("];\n");
    out
}

/// A tree node for reconstructing nested PHP arrays.
#[derive(Debug)]
enum TreeNode {
    Leaf(String), // full dot-separated key to look up in entries
    Branch(IndexMap<String, TreeNode>),
}

/// Build a tree of TreeNodes from the flat entries map.
fn build_tree(entries: &IndexMap<String, I18nEntry>) -> IndexMap<String, TreeNode> {
    let mut root: IndexMap<String, TreeNode> = IndexMap::new();

    for (full_key, _entry) in entries {
        let parts: Vec<&str> = full_key.split('.').collect();
        insert_into_tree(&mut root, &parts, full_key);
    }

    root
}

fn insert_into_tree(node: &mut IndexMap<String, TreeNode>, parts: &[&str], full_key: &str) {
    if parts.len() == 1 {
        node.insert(parts[0].to_string(), TreeNode::Leaf(full_key.to_string()));
        return;
    }

    let first = parts[0].to_string();
    let rest = &parts[1..];

    let child = node
        .entry(first)
        .or_insert_with(|| TreeNode::Branch(IndexMap::new()));

    match child {
        TreeNode::Branch(ref mut children) => {
            insert_into_tree(children, rest, full_key);
        }
        TreeNode::Leaf(_) => {
            // Conflict: a key is both a leaf and a branch. Treat as leaf (keep as-is).
            // This shouldn't happen in well-formed input.
        }
    }
}

type QuoteStyle = (char, fn(&str) -> String, fn(&str) -> String);

fn write_tree_entries(
    tree: &IndexMap<String, TreeNode>,
    entries: &IndexMap<String, I18nEntry>,
    out: &mut String,
    indent: usize,
    use_double: bool,
) {
    let indent_str = "    ".repeat(indent);
    let (q, esc_key, esc_val): QuoteStyle = if use_double {
        ('"', escape_double_quoted, escape_double_quoted)
    } else {
        ('\'', escape_single_quoted, escape_single_quoted)
    };

    for (key, node) in tree {
        match node {
            TreeNode::Leaf(full_key) => {
                if let Some(entry) = entries.get(full_key) {
                    // Write comments
                    for comment in &entry.comments {
                        out.push_str(&format!("{}// {}\n", indent_str, comment.text));
                    }

                    let value_str = match &entry.value {
                        EntryValue::Simple(s) => s.clone(),
                        EntryValue::Plural(ps) => ps.other.clone(),
                        EntryValue::Array(arr) => arr.join(", "),
                        EntryValue::Select(ss) => {
                            ss.cases.values().next().cloned().unwrap_or_default()
                        }
                        EntryValue::MultiVariablePlural(mvp) => mvp.pattern.clone(),
                    };

                    out.push_str(&format!(
                        "{}{q}{}{q} => {q}{}{q},\n",
                        indent_str,
                        esc_key(key),
                        esc_val(&value_str)
                    ));
                }
            }
            TreeNode::Branch(children) => {
                out.push_str(&format!("{}{q}{}{q} => [\n", indent_str, esc_key(key)));
                write_tree_entries(children, entries, out, indent + 1, use_double);
                out.push_str(&format!("{indent_str}],\n"));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Trait implementations
// ---------------------------------------------------------------------------

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        if extension == ".php" {
            if let Ok(s) = std::str::from_utf8(content) {
                if s.contains("return [") || s.contains("return array(") {
                    return Confidence::Definite;
                }
                // Might be a PHP file but without the return statement visible
                if s.contains("<?php") {
                    return Confidence::Low;
                }
            }
            return Confidence::Low;
        }
        if let Ok(s) = std::str::from_utf8(content) {
            if s.contains("<?php") && (s.contains("return [") || s.contains("return array(")) {
                return Confidence::High;
            }
        }
        Confidence::None
    }

    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError> {
        let text = std::str::from_utf8(content)
            .map_err(|e| ParseError::InvalidFormat(format!("Invalid UTF-8: {e}")))?;
        parse_php(text)
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
        let output = write_php(resource);
        Ok(output.into_bytes())
    }

    fn capabilities(&self) -> FormatCapabilities {
        Parser.capabilities()
    }
}
