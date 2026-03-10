use super::*;

pub struct Parser;
pub struct Writer;

// ---------------------------------------------------------------------------
// HJSON custom parser
// ---------------------------------------------------------------------------

/// A parsed HJSON value (intermediate representation before converting to IR).
#[derive(Debug, Clone)]
enum HjsonValue {
    Str(String),
    Object(IndexMap<String, HjsonValue>),
    Array(Vec<HjsonValue>),
}

struct HjsonParser {
    chars: Vec<char>,
    pos: usize,
}

impl HjsonParser {
    fn new(input: &str) -> Self {
        Self {
            chars: input.chars().collect(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.chars.get(self.pos).copied();
        if ch.is_some() {
            self.pos += 1;
        }
        ch
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            // Skip whitespace
            while let Some(ch) = self.peek() {
                if ch.is_whitespace() {
                    self.advance();
                } else {
                    break;
                }
            }

            // Check for comments
            if self.pos + 1 < self.chars.len() {
                let ch = self.chars[self.pos];
                let next = self.chars[self.pos + 1];

                if ch == '/' && next == '/' {
                    // Single-line comment: skip to end of line
                    while let Some(c) = self.advance() {
                        if c == '\n' {
                            break;
                        }
                    }
                    continue;
                }
                if ch == '/' && next == '*' {
                    // Block comment: skip to */
                    self.advance(); // consume /
                    self.advance(); // consume *
                    loop {
                        match self.advance() {
                            Some('*') => {
                                if self.peek() == Some('/') {
                                    self.advance();
                                    break;
                                }
                            }
                            None => break,
                            _ => {}
                        }
                    }
                    continue;
                }
                if ch == '#' {
                    // Hash comment: skip to end of line
                    while let Some(c) = self.advance() {
                        if c == '\n' {
                            break;
                        }
                    }
                    continue;
                }
            } else if let Some(ch) = self.peek() {
                if ch == '#' {
                    // Hash comment at end of input
                    while self.advance().is_some() {}
                    continue;
                }
            }

            break;
        }
    }

    fn parse_root(&mut self) -> Result<IndexMap<String, HjsonValue>, ParseError> {
        self.skip_whitespace_and_comments();
        match self.peek() {
            Some('{') => {
                if let HjsonValue::Object(map) = self.parse_object()? {
                    Ok(map)
                } else {
                    Err(ParseError::InvalidFormat("Root must be an object".into()))
                }
            }
            _ => Err(ParseError::InvalidFormat(
                "HJSON root must start with '{'".into(),
            )),
        }
    }

    fn parse_object(&mut self) -> Result<HjsonValue, ParseError> {
        self.advance(); // consume '{'
        let mut map = IndexMap::new();

        loop {
            self.skip_whitespace_and_comments();
            if self.peek() == Some('}') {
                self.advance();
                return Ok(HjsonValue::Object(map));
            }
            if self.peek().is_none() {
                return Err(ParseError::InvalidFormat(
                    "Unterminated object".into(),
                ));
            }

            // Parse key
            let key = self.parse_key()?;

            self.skip_whitespace_and_comments();

            // Expect ':'
            match self.peek() {
                Some(':') => {
                    self.advance();
                }
                other => {
                    return Err(ParseError::InvalidFormat(format!(
                        "Expected ':' after key \"{}\", got {:?}",
                        key, other
                    )));
                }
            }

            self.skip_whitespace_and_comments();

            // Parse value (in object context, commas allowed inside unquoted values)
            let value = self.parse_value(false)?;
            map.insert(key, value);

            // Skip optional comma
            self.skip_whitespace_and_comments();
            if self.peek() == Some(',') {
                self.advance();
            }
        }
    }

    fn parse_array(&mut self) -> Result<HjsonValue, ParseError> {
        self.advance(); // consume '['
        let mut items = Vec::new();

        loop {
            self.skip_whitespace_and_comments();
            if self.peek() == Some(']') {
                self.advance();
                return Ok(HjsonValue::Array(items));
            }
            if self.peek().is_none() {
                return Err(ParseError::InvalidFormat(
                    "Unterminated array".into(),
                ));
            }

            let value = self.parse_value(true)?;
            items.push(value);

            // Skip optional comma
            self.skip_whitespace_and_comments();
            if self.peek() == Some(',') {
                self.advance();
            }
        }
    }

    fn parse_key(&mut self) -> Result<String, ParseError> {
        match self.peek() {
            Some('"') => self.parse_double_quoted_string(),
            Some(ch) if is_unquoted_key_start(ch) => self.parse_unquoted_key(),
            other => Err(ParseError::InvalidFormat(format!(
                "Expected key, got {:?}",
                other
            ))),
        }
    }

    fn parse_value(&mut self, in_array: bool) -> Result<HjsonValue, ParseError> {
        self.skip_whitespace_and_comments();
        match self.peek() {
            Some('{') => self.parse_object(),
            Some('[') => self.parse_array(),
            Some('"') => {
                let s = self.parse_double_quoted_string()?;
                Ok(HjsonValue::Str(s))
            }
            Some('\'') => {
                // Check for multi-line string '''
                if self.pos + 2 < self.chars.len()
                    && self.chars[self.pos + 1] == '\''
                    && self.chars[self.pos + 2] == '\''
                {
                    self.parse_multiline_string()
                } else {
                    self.parse_single_quoted_string()
                }
            }
            Some(_) => {
                // Unquoted value: read until end of line
                // In arrays, commas separate items; in objects, commas are part of the value
                self.parse_unquoted_value(in_array)
            }
            None => Err(ParseError::InvalidFormat(
                "Unexpected end of input while parsing value".into(),
            )),
        }
    }

    fn parse_double_quoted_string(&mut self) -> Result<String, ParseError> {
        self.advance(); // consume opening '"'
        let mut s = String::new();
        loop {
            match self.advance() {
                Some('\\') => {
                    match self.advance() {
                        Some('"') => s.push('"'),
                        Some('\\') => s.push('\\'),
                        Some('/') => s.push('/'),
                        Some('n') => s.push('\n'),
                        Some('r') => s.push('\r'),
                        Some('t') => s.push('\t'),
                        Some('b') => s.push('\u{0008}'),
                        Some('f') => s.push('\u{000C}'),
                        Some('u') => {
                            let mut hex = String::new();
                            for _ in 0..4 {
                                match self.advance() {
                                    Some(c) if c.is_ascii_hexdigit() => hex.push(c),
                                    _ => {
                                        return Err(ParseError::InvalidFormat(
                                            "Invalid unicode escape".into(),
                                        ));
                                    }
                                }
                            }
                            let code = u32::from_str_radix(&hex, 16).map_err(|_| {
                                ParseError::InvalidFormat("Invalid unicode escape".into())
                            })?;
                            match char::from_u32(code) {
                                Some(c) => s.push(c),
                                None => {
                                    return Err(ParseError::InvalidFormat(
                                        "Invalid unicode code point".into(),
                                    ));
                                }
                            }
                        }
                        Some(c) => {
                            s.push('\\');
                            s.push(c);
                        }
                        None => {
                            return Err(ParseError::InvalidFormat(
                                "Unterminated escape sequence".into(),
                            ));
                        }
                    }
                }
                Some('"') => return Ok(s),
                Some(c) => s.push(c),
                None => {
                    return Err(ParseError::InvalidFormat(
                        "Unterminated double-quoted string".into(),
                    ));
                }
            }
        }
    }

    fn parse_single_quoted_string(&mut self) -> Result<HjsonValue, ParseError> {
        self.advance(); // consume opening '\''
        let mut s = String::new();
        loop {
            match self.advance() {
                Some('\'') => return Ok(HjsonValue::Str(s)),
                Some(c) => s.push(c),
                None => {
                    return Err(ParseError::InvalidFormat(
                        "Unterminated single-quoted string".into(),
                    ));
                }
            }
        }
    }

    fn parse_multiline_string(&mut self) -> Result<HjsonValue, ParseError> {
        // Consume the opening '''
        self.advance(); // '
        self.advance(); // '
        self.advance(); // '

        // Skip the rest of the line after opening ''' (including newline)
        while let Some(ch) = self.peek() {
            if ch == '\n' {
                self.advance();
                break;
            } else if ch == '\r' {
                self.advance();
                if self.peek() == Some('\n') {
                    self.advance();
                }
                break;
            } else if ch.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }

        let mut lines: Vec<String> = Vec::new();
        let mut current_line = String::new();

        loop {
            match self.peek() {
                Some('\'') => {
                    if self.pos + 2 < self.chars.len()
                        && self.chars[self.pos + 1] == '\''
                        && self.chars[self.pos + 2] == '\''
                    {
                        // Found closing '''
                        self.advance();
                        self.advance();
                        self.advance();

                        // Only add the last line if it has non-whitespace content.
                        // The closing ''' is typically on its own line, so the
                        // current_line would just be whitespace/indentation.
                        let trimmed = current_line.trim_end().to_string();
                        if !trimmed.is_empty() {
                            lines.push(trimmed);
                        }

                        // Remove common leading whitespace (dedent)
                        let result = dedent_lines(&lines);
                        return Ok(HjsonValue::Str(result));
                    } else {
                        current_line.push('\'');
                        self.advance();
                    }
                }
                Some('\n') => {
                    lines.push(current_line.clone());
                    current_line.clear();
                    self.advance();
                }
                Some('\r') => {
                    self.advance();
                    if self.peek() == Some('\n') {
                        self.advance();
                    }
                    lines.push(current_line.clone());
                    current_line.clear();
                }
                Some(c) => {
                    current_line.push(c);
                    self.advance();
                }
                None => {
                    return Err(ParseError::InvalidFormat(
                        "Unterminated multi-line string".into(),
                    ));
                }
            }
        }
    }

    fn parse_unquoted_key(&mut self) -> Result<String, ParseError> {
        let mut key = String::new();
        while let Some(ch) = self.peek() {
            if is_unquoted_key_char(ch) {
                key.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        if key.is_empty() {
            return Err(ParseError::InvalidFormat("Empty key".into()));
        }
        Ok(key)
    }

    fn parse_unquoted_value(&mut self, in_array: bool) -> Result<HjsonValue, ParseError> {
        // Unquoted value in HJSON: everything until end of line, trimmed.
        // `}` and `]` always stop parsing (they close the enclosing structure).
        // In arrays, commas also stop parsing (they separate items).
        // In objects, commas are part of the value text, but a trailing comma is stripped.
        let mut value = String::new();
        loop {
            match self.peek() {
                None => break,
                Some('\n') | Some('\r') => break,
                Some('}') | Some(']') => break,
                Some(',') if in_array => break,
                Some(ch) => {
                    value.push(ch);
                    self.advance();
                }
            }
        }
        // Trim trailing whitespace
        let mut trimmed = value.trim_end().to_string();
        // Strip a single trailing comma if present (HJSON allows optional trailing commas
        // after object values)
        if !in_array && trimmed.ends_with(',') {
            trimmed.pop();
            trimmed = trimmed.trim_end().to_string();
        }
        Ok(HjsonValue::Str(trimmed))
    }
}

fn is_unquoted_key_start(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_' || ch == '-' || ch == '$'
}

fn is_unquoted_key_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_' || ch == '-' || ch == '.' || ch == '$'
}

fn dedent_lines(lines: &[String]) -> String {
    if lines.is_empty() {
        return String::new();
    }

    // Find minimum indentation of non-empty lines
    let min_indent = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);

    let dedented: Vec<&str> = lines
        .iter()
        .map(|line| {
            if line.len() >= min_indent {
                &line[min_indent..]
            } else {
                line.trim()
            }
        })
        .collect();

    dedented.join("\n")
}

/// Flatten an HJSON value tree into IR entries with dot-separated keys.
fn flatten_hjson(
    value: &HjsonValue,
    prefix: &str,
    entries: &mut IndexMap<String, I18nEntry>,
) {
    match value {
        HjsonValue::Str(s) => {
            entries.insert(
                prefix.to_string(),
                I18nEntry {
                    key: prefix.to_string(),
                    value: EntryValue::Simple(s.clone()),
                    ..Default::default()
                },
            );
        }
        HjsonValue::Object(map) => {
            for (key, val) in map {
                let full_key = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                flatten_hjson(val, &full_key, entries);
            }
        }
        HjsonValue::Array(items) => {
            let strings: Vec<String> = items
                .iter()
                .map(|item| match item {
                    HjsonValue::Str(s) => s.clone(),
                    _ => String::new(),
                })
                .collect();
            entries.insert(
                prefix.to_string(),
                I18nEntry {
                    key: prefix.to_string(),
                    value: EntryValue::Array(strings),
                    ..Default::default()
                },
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Trait implementations
// ---------------------------------------------------------------------------

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        if extension == ".hjson" {
            return Confidence::Definite;
        }
        // Try content-based detection: looks like JSON-ish with unquoted values
        if let Ok(s) = std::str::from_utf8(content) {
            let trimmed = s.trim();
            if trimmed.starts_with('{') {
                // Check for HJSON-specific features: unquoted keys, # comments, // comments
                if trimmed.contains("# ") || trimmed.contains("// ") {
                    return Confidence::Low;
                }
            }
        }
        Confidence::None
    }

    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError> {
        let s = std::str::from_utf8(content)
            .map_err(|e| ParseError::InvalidFormat(format!("Invalid UTF-8: {e}")))?;

        let mut parser = HjsonParser::new(s);
        let root = parser.parse_root()?;

        let mut entries = IndexMap::new();
        for (key, value) in &root {
            flatten_hjson(value, key, &mut entries);
        }

        Ok(I18nResource {
            metadata: ResourceMetadata {
                source_format: FormatId::Hjson,
                format_ext: Some(FormatExtension::Hjson(HjsonExt {})),
                ..Default::default()
            },
            entries,
        })
    }

    fn capabilities(&self) -> FormatCapabilities {
        FormatCapabilities {
            plurals: false,
            arrays: true,
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

// ---------------------------------------------------------------------------
// Writer: output as standard JSON (JSON is a subset of HJSON)
// ---------------------------------------------------------------------------

impl FormatWriter for Writer {
    fn write(&self, resource: &I18nResource) -> Result<Vec<u8>, WriteError> {
        let mut root = serde_json::Map::new();

        for (_, entry) in &resource.entries {
            let json_value = match &entry.value {
                EntryValue::Simple(s) => serde_json::Value::String(s.clone()),
                EntryValue::Plural(ps) => serde_json::Value::String(ps.other.clone()),
                EntryValue::Array(arr) => {
                    serde_json::Value::Array(
                        arr.iter()
                            .map(|s| serde_json::Value::String(s.clone()))
                            .collect(),
                    )
                }
                EntryValue::Select(ss) => {
                    serde_json::Value::String(
                        ss.cases.get("other").cloned().unwrap_or_default(),
                    )
                }
                EntryValue::MultiVariablePlural(mvp) => {
                    serde_json::Value::String(mvp.pattern.clone())
                }
            };

            insert_nested(&mut root, &entry.key, json_value)?;
        }

        let json = serde_json::to_string_pretty(&serde_json::Value::Object(root))
            .map_err(|e| WriteError::Serialization(format!("{e}")))?;

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
fn insert_nested(
    root: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    value: serde_json::Value,
) -> Result<(), WriteError> {
    let parts: Vec<&str> = key.split('.').collect();
    if parts.len() == 1 {
        root.insert(key.to_string(), value);
        return Ok(());
    }

    let mut current = root;
    for part in &parts[..parts.len() - 1] {
        current = current
            .entry(part.to_string())
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()))
            .as_object_mut()
            .ok_or_else(|| WriteError::Serialization(
                format!("Key path conflict: '{}' is both a value and an object in key '{}'", part, key)
            ))?;
    }

    let leaf_key = parts[parts.len() - 1];
    current.insert(leaf_key.to_string(), value);
    Ok(())
}
