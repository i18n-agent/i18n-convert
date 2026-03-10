use super::*;
use indexmap::IndexMap;

pub struct Parser;
pub struct Writer;

// ---------------------------------------------------------------------------
//  PO Parser
// ---------------------------------------------------------------------------

/// Represents one raw PO block before conversion to IR.
#[derive(Debug, Default)]
struct PoBlock {
    translator_comments: Vec<String>,
    extracted_comments: Vec<String>,
    references: Vec<String>,
    flags: Vec<String>,
    previous_msgid: Option<String>,
    msgctxt: Option<String>,
    msgid: String,
    msgid_plural: Option<String>,
    msgstr: Option<String>,               // for non-plural
    msgstr_plural: IndexMap<u32, String>, // msgstr[N]
    obsolete: bool,
}

/// Unescape PO string escapes: \n, \t, \\, \"
fn po_unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('\\') => out.push('\\'),
                Some('"') => out.push('"'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Escape a string for PO output.
fn po_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out
}

/// Extract the quoted string content from a line like `"some text"`.
/// Returns None if the line doesn't start with a quote.
fn extract_quoted(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        let inner = &trimmed[1..trimmed.len() - 1];
        Some(po_unescape(inner))
    } else {
        None
    }
}

/// Parse the keyword value from a line like `msgid "hello"` or `msgstr[0] "text"`.
/// Returns the unescaped string value.
fn extract_keyword_value(line: &str, keyword: &str) -> Option<String> {
    let trimmed = line.trim();
    if !trimmed.starts_with(keyword) {
        return None;
    }
    let after = &trimmed[keyword.len()..];
    // There should be at least a space before the quote
    let after = after.trim_start();
    extract_quoted(after)
}

/// Parse all PO blocks from the content.
fn parse_blocks(content: &str) -> Vec<PoBlock> {
    let mut blocks: Vec<PoBlock> = Vec::new();
    let mut current = PoBlock::default();
    let mut in_block = false;
    // Tracks which field we are accumulating multiline strings for
    let mut current_field: Option<CurrentField> = None;

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip completely empty lines - they delimit blocks
        if trimmed.is_empty() {
            if in_block {
                blocks.push(current);
                current = PoBlock::default();
                in_block = false;
                current_field = None;
            }
            continue;
        }

        // Handle obsolete entries (#~ ...)
        if let Some(stripped) = trimmed.strip_prefix("#~") {
            in_block = true;
            current.obsolete = true;
            let rest = stripped.trim();
            // Parse the actual keyword from the obsolete line
            if let Some(val) = extract_keyword_value(rest, "msgid") {
                current.msgid = val;
                current_field = Some(CurrentField::Msgid);
            } else if let Some(val) = extract_keyword_value(rest, "msgstr") {
                current.msgstr = Some(val);
                current_field = Some(CurrentField::Msgstr);
            } else if let Some(val) = extract_quoted(rest) {
                // Multiline continuation of obsolete
                append_to_field(&mut current, &current_field, &val);
            }
            continue;
        }

        // Comment lines
        if trimmed.starts_with('#') && !trimmed.starts_with("#~") {
            in_block = true;
            current_field = None;

            if let Some(stripped) = trimmed.strip_prefix("#.") {
                // Extracted comment
                let text = stripped.trim().to_string();
                current.extracted_comments.push(text);
            } else if let Some(stripped) = trimmed.strip_prefix("#:") {
                // Reference
                let text = stripped.trim().to_string();
                current.references.push(text);
            } else if let Some(stripped) = trimmed.strip_prefix("#,") {
                // Flags
                let text = stripped.trim();
                for flag in text.split(',') {
                    let flag = flag.trim().to_string();
                    if !flag.is_empty() {
                        current.flags.push(flag);
                    }
                }
            } else if let Some(stripped) = trimmed.strip_prefix("#|") {
                // Previous msgid
                let rest = stripped.trim();
                if let Some(val) = extract_keyword_value(rest, "msgid") {
                    current.previous_msgid = Some(val);
                } else if let Some(existing) = &current.previous_msgid {
                    // Multiline continuation of previous msgid
                    if let Some(val) = extract_quoted(rest) {
                        let mut combined = existing.clone();
                        combined.push_str(&val);
                        current.previous_msgid = Some(combined);
                    }
                }
            } else if trimmed.starts_with("# ") || trimmed == "#" {
                // Translator comment
                let text = if trimmed.len() > 2 {
                    trimmed[2..].to_string()
                } else {
                    String::new()
                };
                current.translator_comments.push(text);
            }
            continue;
        }

        // Keyword lines
        if let Some(val) = extract_keyword_value(trimmed, "msgctxt") {
            in_block = true;
            current.msgctxt = Some(val);
            current_field = Some(CurrentField::Msgctxt);
        } else if let Some(val) = extract_keyword_value(trimmed, "msgid_plural") {
            in_block = true;
            current.msgid_plural = Some(val);
            current_field = Some(CurrentField::MsgidPlural);
        } else if trimmed.starts_with("msgstr[") {
            in_block = true;
            // Parse msgstr[N] "value"
            if let Some(bracket_end) = trimmed.find(']') {
                let idx_str = &trimmed[7..bracket_end];
                if let Ok(idx) = idx_str.parse::<u32>() {
                    let rest = trimmed[bracket_end + 1..].trim();
                    if let Some(val) = extract_quoted(rest) {
                        current.msgstr_plural.insert(idx, val);
                        current_field = Some(CurrentField::MsgstrPlural(idx));
                    }
                }
            }
        } else if let Some(val) = extract_keyword_value(trimmed, "msgstr") {
            in_block = true;
            current.msgstr = Some(val);
            current_field = Some(CurrentField::Msgstr);
        } else if let Some(val) = extract_keyword_value(trimmed, "msgid") {
            in_block = true;
            current.msgid = val;
            current_field = Some(CurrentField::Msgid);
        } else if let Some(val) = extract_quoted(trimmed) {
            // Continuation line (multiline string)
            append_to_field(&mut current, &current_field, &val);
        }
    }

    // Don't forget the last block
    if in_block {
        blocks.push(current);
    }

    blocks
}

#[derive(Debug, Clone)]
enum CurrentField {
    Msgctxt,
    Msgid,
    MsgidPlural,
    Msgstr,
    MsgstrPlural(u32),
}

fn append_to_field(block: &mut PoBlock, field: &Option<CurrentField>, value: &str) {
    match field {
        Some(CurrentField::Msgctxt) => {
            if let Some(ref mut ctx) = block.msgctxt {
                ctx.push_str(value);
            }
        }
        Some(CurrentField::Msgid) => {
            block.msgid.push_str(value);
        }
        Some(CurrentField::MsgidPlural) => {
            if let Some(ref mut p) = block.msgid_plural {
                p.push_str(value);
            }
        }
        Some(CurrentField::Msgstr) => {
            if let Some(ref mut s) = block.msgstr {
                s.push_str(value);
            }
        }
        Some(CurrentField::MsgstrPlural(idx)) => {
            let idx = *idx;
            if let Some(existing) = block.msgstr_plural.get_mut(&idx) {
                existing.push_str(value);
            }
        }
        None => {}
    }
}

/// Map plural index to CLDR plural category based on nplurals.
/// This is a simplified mapping for common languages.
fn plural_index_to_category(index: u32, nplurals: u32) -> &'static str {
    match nplurals {
        1 => "other",
        2 => match index {
            0 => "one",
            _ => "other",
        },
        3 => match index {
            0 => "one",
            1 => "few",
            _ => "other",
        },
        4 => match index {
            0 => "one",
            1 => "few",
            2 => "many",
            _ => "other",
        },
        5 => match index {
            0 => "one",
            1 => "two",
            2 => "few",
            3 => "many",
            _ => "other",
        },
        6 => match index {
            0 => "zero",
            1 => "one",
            2 => "two",
            3 => "few",
            4 => "many",
            _ => "other",
        },
        _ => "other",
    }
}

/// Parse nplurals from the Plural-Forms header.
fn parse_nplurals(plural_forms: &str) -> u32 {
    // Plural-Forms: nplurals=2; plural=(n != 1);
    for part in plural_forms.split(';') {
        let part = part.trim();
        if let Some(rest) = part.strip_prefix("nplurals=") {
            if let Ok(n) = rest.trim().parse::<u32>() {
                return n;
            }
        }
    }
    2 // default
}

/// Build the entry key from msgctxt and msgid.
/// If msgctxt is present, the key is "msgctxt\x04msgid" (standard PO convention).
fn make_entry_key(msgctxt: &Option<String>, msgid: &str) -> String {
    match msgctxt {
        Some(ctx) => format!("{ctx}\x04{msgid}"),
        None => msgid.to_string(),
    }
}

/// Parse source references from "#: file:line" strings.
fn parse_source_refs(refs: &[String]) -> Vec<SourceRef> {
    let mut result = Vec::new();
    for ref_str in refs {
        // Each #: line can have multiple space-separated references
        for part in ref_str.split_whitespace() {
            if let Some(colon_pos) = part.rfind(':') {
                let file = &part[..colon_pos];
                let line_str = &part[colon_pos + 1..];
                if let Ok(line) = line_str.parse::<u32>() {
                    result.push(SourceRef {
                        file: file.to_string(),
                        line: Some(line),
                    });
                } else {
                    result.push(SourceRef {
                        file: part.to_string(),
                        line: None,
                    });
                }
            } else {
                result.push(SourceRef {
                    file: part.to_string(),
                    line: None,
                });
            }
        }
    }
    result
}

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        if extension == ".po" || extension == ".pot" {
            return Confidence::Definite;
        }
        if let Ok(s) = std::str::from_utf8(content) {
            if s.contains("msgid") && s.contains("msgstr") {
                return Confidence::Definite;
            }
        }
        Confidence::None
    }

    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError> {
        let text = std::str::from_utf8(content)
            .map_err(|e| ParseError::InvalidFormat(format!("Invalid UTF-8: {e}")))?;

        let blocks = parse_blocks(text);
        let mut metadata = ResourceMetadata {
            source_format: FormatId::Po,
            ..Default::default()
        };
        let mut entries = IndexMap::new();
        let mut plural_forms_header: Option<String> = None;
        let mut nplurals: u32 = 2;

        for block in blocks {
            // Header block: msgid is empty string
            if block.msgid.is_empty() && block.msgctxt.is_none() && !block.obsolete {
                if let Some(ref msgstr) = block.msgstr {
                    // Parse header lines
                    for header_line in msgstr.split('\n') {
                        let header_line = header_line.trim();
                        if header_line.is_empty() {
                            continue;
                        }
                        if let Some(colon_pos) = header_line.find(':') {
                            let key = header_line[..colon_pos].trim().to_string();
                            let val = header_line[colon_pos + 1..].trim().to_string();

                            if key == "Language" {
                                metadata.locale = Some(val.clone());
                            }
                            if key == "Plural-Forms" {
                                plural_forms_header = Some(val.clone());
                                nplurals = parse_nplurals(&val);
                            }
                            metadata.headers.insert(key, val);
                        }
                    }
                }
                // Store translator comments from header as PoExt
                if !block.translator_comments.is_empty() {
                    metadata.format_ext = Some(FormatExtension::Po(PoExt {
                        plural_forms_header: plural_forms_header.clone(),
                        translator_comments: block.translator_comments.clone(),
                    }));
                } else {
                    metadata.format_ext = Some(FormatExtension::Po(PoExt {
                        plural_forms_header: plural_forms_header.clone(),
                        translator_comments: Vec::new(),
                    }));
                }
                continue;
            }

            // Regular entry
            let key = make_entry_key(&block.msgctxt, &block.msgid);

            let value = if block.msgid_plural.is_some() {
                // Plural entry
                let mut ps = PluralSet::default();
                for (&idx, val) in &block.msgstr_plural {
                    let category = plural_index_to_category(idx, nplurals);
                    match category {
                        "zero" => ps.zero = Some(val.clone()),
                        "one" => ps.one = Some(val.clone()),
                        "two" => ps.two = Some(val.clone()),
                        "few" => ps.few = Some(val.clone()),
                        "many" => ps.many = Some(val.clone()),
                        "other" => ps.other = val.clone(),
                        _ => ps.other = val.clone(),
                    }
                }
                EntryValue::Plural(ps)
            } else {
                EntryValue::Simple(block.msgstr.unwrap_or_default())
            };

            // Store msgid_plural in properties for round-trip fidelity
            let mut properties = IndexMap::new();
            if let Some(ref plural) = block.msgid_plural {
                properties.insert("msgid_plural".to_string(), plural.clone());
            }

            let mut comments = Vec::new();
            for c in &block.translator_comments {
                comments.push(Comment {
                    text: c.clone(),
                    role: CommentRole::Translator,
                    priority: None,
                    annotates: None,
                });
            }
            for c in &block.extracted_comments {
                comments.push(Comment {
                    text: c.clone(),
                    role: CommentRole::Extracted,
                    priority: None,
                    annotates: None,
                });
            }

            let source_references = parse_source_refs(&block.references);

            let mut state = None;
            if block.flags.iter().any(|f| f == "fuzzy") {
                state = Some(TranslationState::NeedsReview);
            }

            let mut contexts = Vec::new();
            if let Some(ref ctx) = block.msgctxt {
                contexts.push(ContextEntry {
                    context_type: ContextType::Disambiguation,
                    value: ctx.clone(),
                    purpose: None,
                });
            }

            let entry = I18nEntry {
                key: key.clone(),
                value,
                comments,
                contexts,
                source: Some(block.msgid.clone()),
                previous_source: block.previous_msgid,
                previous_comment: None,
                placeholders: Vec::new(),
                translatable: None,
                state,
                state_qualifier: None,
                approved: None,
                obsolete: block.obsolete,
                max_width: None,
                min_width: None,
                max_height: None,
                min_height: None,
                size_unit: None,
                max_bytes: None,
                min_bytes: None,
                source_references,
                flags: block.flags,
                device_variants: None,
                alternatives: Vec::new(),
                properties,
                resource_type: None,
                resource_name: None,
                format_ext: None,
            };

            entries.insert(key, entry);
        }

        // Ensure PoExt is always set even if no header block
        if metadata.format_ext.is_none() {
            metadata.format_ext = Some(FormatExtension::Po(PoExt {
                plural_forms_header: None,
                translator_comments: Vec::new(),
            }));
        }

        Ok(I18nResource { metadata, entries })
    }

    fn capabilities(&self) -> FormatCapabilities {
        FormatCapabilities {
            plurals: true,
            arrays: false,
            comments: true,
            context: true,
            source_string: true,
            translatable_flag: false,
            translation_state: true,
            max_width: false,
            device_variants: false,
            select_gender: false,
            nested_keys: false,
            inline_markup: false,
            alternatives: false,
            source_references: true,
            custom_properties: false,
        }
    }
}

// ---------------------------------------------------------------------------
//  PO Writer
// ---------------------------------------------------------------------------

/// Format a PO string, potentially as multiline if it contains newlines.
/// Returns lines like `"text"` or `""` + `"line1\n"` + `"line2"`.
fn format_po_string(s: &str) -> Vec<String> {
    let escaped = po_escape(s);
    // If the string contains literal \n (i.e., the escaped form), split into multiline
    if escaped.contains("\\n")
        && escaped != "\\n"
        && escaped.len() > escaped.find("\\n").unwrap_or(0) + 2
    {
        let mut lines = Vec::new();
        lines.push("\"\"".to_string());
        // Split on \n but keep the \n at the end of each segment
        let parts: Vec<&str> = escaped.split("\\n").collect();
        for (i, part) in parts.iter().enumerate() {
            if i < parts.len() - 1 {
                lines.push(format!("\"{part}\\n\""));
            } else if !part.is_empty() {
                lines.push(format!("\"{part}\""));
            }
        }
        lines
    } else {
        vec![format!("\"{}\"", escaped)]
    }
}

/// Get the nplurals value from a resource's PoExt or headers.
fn get_nplurals(resource: &I18nResource) -> u32 {
    if let Some(FormatExtension::Po(ref ext)) = resource.metadata.format_ext {
        if let Some(ref pf) = ext.plural_forms_header {
            return parse_nplurals(pf);
        }
    }
    if let Some(pf) = resource.metadata.headers.get("Plural-Forms") {
        return parse_nplurals(pf);
    }
    2
}

/// Map CLDR category back to plural index for a given nplurals count.
#[allow(dead_code)]
fn category_to_plural_index(category: &str, nplurals: u32) -> Option<u32> {
    match nplurals {
        1 => match category {
            "other" => Some(0),
            _ => None,
        },
        2 => match category {
            "one" => Some(0),
            "other" => Some(1),
            _ => None,
        },
        3 => match category {
            "one" => Some(0),
            "few" => Some(1),
            "other" => Some(2),
            _ => None,
        },
        4 => match category {
            "one" => Some(0),
            "few" => Some(1),
            "many" => Some(2),
            "other" => Some(3),
            _ => None,
        },
        5 => match category {
            "one" => Some(0),
            "two" => Some(1),
            "few" => Some(2),
            "many" => Some(3),
            "other" => Some(4),
            _ => None,
        },
        6 => match category {
            "zero" => Some(0),
            "one" => Some(1),
            "two" => Some(2),
            "few" => Some(3),
            "many" => Some(4),
            "other" => Some(5),
            _ => None,
        },
        _ => None,
    }
}

impl FormatWriter for Writer {
    fn write(&self, resource: &I18nResource) -> Result<Vec<u8>, WriteError> {
        let mut out = String::new();
        let nplurals = get_nplurals(resource);

        // Write header block
        write_header(&mut out, resource);

        // Write entries
        for (_key, entry) in &resource.entries {
            out.push('\n');
            write_entry(&mut out, entry, nplurals);
        }

        Ok(out.into_bytes())
    }

    fn capabilities(&self) -> FormatCapabilities {
        Parser.capabilities()
    }
}

fn write_header(out: &mut String, resource: &I18nResource) {
    // Write header translator comments from PoExt
    if let Some(FormatExtension::Po(ref ext)) = resource.metadata.format_ext {
        for comment in &ext.translator_comments {
            out.push_str(&format!("# {comment}\n"));
        }
    }

    out.push_str("msgid \"\"\n");
    out.push_str("msgstr \"\"\n");

    // Write headers
    for (key, val) in &resource.metadata.headers {
        out.push_str(&format!("\"{}: {}\\n\"\n", po_escape(key), po_escape(val)));
    }
}

fn write_entry(out: &mut String, entry: &I18nEntry, nplurals: u32) {
    // Write translator comments
    for comment in &entry.comments {
        match comment.role {
            CommentRole::Translator => {
                out.push_str(&format!("# {}\n", comment.text));
            }
            CommentRole::Extracted => {
                out.push_str(&format!("#. {}\n", comment.text));
            }
            _ => {
                out.push_str(&format!("# {}\n", comment.text));
            }
        }
    }

    // Write source references
    for src_ref in &entry.source_references {
        if let Some(line) = src_ref.line {
            out.push_str(&format!("#: {}:{}\n", src_ref.file, line));
        } else {
            out.push_str(&format!("#: {}\n", src_ref.file));
        }
    }

    // Write flags
    if !entry.flags.is_empty() {
        out.push_str(&format!("#, {}\n", entry.flags.join(", ")));
    } else if entry.state == Some(TranslationState::NeedsReview) {
        out.push_str("#, fuzzy\n");
    }

    // Write previous msgid
    if let Some(ref prev) = entry.previous_source {
        out.push_str(&format!("#| msgid \"{}\"\n", po_escape(prev)));
    }

    // Handle obsolete entries
    if entry.obsolete {
        if let Some(ref source) = entry.source {
            out.push_str(&format!("#~ msgid \"{}\"\n", po_escape(source)));
        }
        match &entry.value {
            EntryValue::Simple(val) => {
                out.push_str(&format!("#~ msgstr \"{}\"\n", po_escape(val)));
            }
            _ => {
                out.push_str("#~ msgstr \"\"\n");
            }
        }
        return;
    }

    // Write msgctxt
    for ctx in &entry.contexts {
        if ctx.context_type == ContextType::Disambiguation {
            let lines = format_po_string(&ctx.value);
            out.push_str(&format!("msgctxt {}\n", lines[0]));
            for line in &lines[1..] {
                out.push_str(&format!("{line}\n"));
            }
        }
    }

    // Write msgid (use source if available, otherwise key)
    let msgid = entry.source.as_deref().unwrap_or(&entry.key);
    let lines = format_po_string(msgid);
    out.push_str(&format!("msgid {}\n", lines[0]));
    for line in &lines[1..] {
        out.push_str(&format!("{line}\n"));
    }

    match &entry.value {
        EntryValue::Plural(ps) => {
            // Write msgid_plural
            // We need the plural source form. Check if there's a msgid_plural stored.
            // For round-trip, we store msgid_plural in entry properties.
            let msgid_plural = entry
                .properties
                .get("msgid_plural")
                .cloned()
                .unwrap_or_else(|| {
                    // Fallback: use msgid + "s" or the source
                    format!("{msgid}s")
                });
            let lines = format_po_string(&msgid_plural);
            out.push_str(&format!("msgid_plural {}\n", lines[0]));
            for line in &lines[1..] {
                out.push_str(&format!("{line}\n"));
            }

            // Write msgstr[N] for each plural form
            let categories = [
                ("zero", &ps.zero),
                ("one", &ps.one),
                ("two", &ps.two),
                ("few", &ps.few),
                ("many", &ps.many),
            ];

            for i in 0..nplurals {
                let category = plural_index_to_category(i, nplurals);
                let val = match category {
                    "zero" => ps.zero.as_deref().unwrap_or(""),
                    "one" => ps.one.as_deref().unwrap_or(""),
                    "two" => ps.two.as_deref().unwrap_or(""),
                    "few" => ps.few.as_deref().unwrap_or(""),
                    "many" => ps.many.as_deref().unwrap_or(""),
                    "other" => &ps.other,
                    _ => "",
                };
                // Avoid unused variable warning
                let _ = categories;
                let lines = format_po_string(val);
                out.push_str(&format!("msgstr[{}] {}\n", i, lines[0]));
                for line in &lines[1..] {
                    out.push_str(&format!("{line}\n"));
                }
            }
        }
        EntryValue::Simple(val) => {
            let lines = format_po_string(val);
            out.push_str(&format!("msgstr {}\n", lines[0]));
            for line in &lines[1..] {
                out.push_str(&format!("{line}\n"));
            }
        }
        _ => {
            // Arrays and other types aren't natively supported in PO
            out.push_str("msgstr \"\"\n");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_po_escape_unescape() {
        assert_eq!(po_escape("hello\nworld"), "hello\\nworld");
        assert_eq!(po_escape("say \"hi\""), "say \\\"hi\\\"");
        assert_eq!(po_unescape("hello\\nworld"), "hello\nworld");
        assert_eq!(po_unescape("say \\\"hi\\\""), "say \"hi\"");
    }

    #[test]
    fn test_extract_quoted() {
        assert_eq!(extract_quoted("\"hello\""), Some("hello".to_string()));
        assert_eq!(extract_quoted("\"\""), Some("".to_string()));
        assert_eq!(
            extract_quoted("\"hello\\nworld\""),
            Some("hello\nworld".to_string())
        );
        assert_eq!(extract_quoted("not quoted"), None);
    }

    #[test]
    fn test_parse_nplurals() {
        assert_eq!(parse_nplurals("nplurals=2; plural=(n != 1);"), 2);
        assert_eq!(parse_nplurals("nplurals=3; plural=(n%10==1 && n%100!=11 ? 0 : n%10>=2 && n%10<=4 && (n%100<10 || n%100>=20) ? 1 : 2);"), 3);
        assert_eq!(parse_nplurals("nplurals=1; plural=0;"), 1);
    }

    #[test]
    fn test_format_po_string_simple() {
        let lines = format_po_string("hello");
        assert_eq!(lines, vec!["\"hello\""]);
    }
}
