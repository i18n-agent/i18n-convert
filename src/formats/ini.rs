use crate::ir::*;
use super::*;
use indexmap::IndexMap;

pub struct Parser;
pub struct Writer;

// ---------------------------------------------------------------------------
// Plural suffix handling (same convention as i18next)
// ---------------------------------------------------------------------------

const PLURAL_SUFFIXES: &[(&str, &str)] = &[
    ("_zero", "zero"),
    ("_one", "one"),
    ("_two", "two"),
    ("_few", "few"),
    ("_many", "many"),
    ("_other", "other"),
];

/// Check if a key ends with a plural suffix. Returns (base_key, category) if so.
fn strip_plural_suffix(key: &str) -> Option<(&str, &str)> {
    for &(suffix, category) in PLURAL_SUFFIXES {
        if key.ends_with(suffix) {
            let base = &key[..key.len() - suffix.len()];
            if !base.is_empty() {
                return Some((base, category));
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

/// Represents one raw key-value pair with associated metadata before IR conversion.
#[derive(Debug)]
struct RawEntry {
    section: Option<String>,
    key: String,
    value: String,
    comments: Vec<(String, char)>,
    delimiter: char,
}

fn parse_ini_content(content: &str) -> Result<I18nResource, ParseError> {
    let mut raw_entries: Vec<RawEntry> = Vec::new();
    let mut current_section: Option<String> = None;
    let mut pending_comments: Vec<(String, char)> = Vec::new();
    let mut first_delimiter: Option<char> = None;
    let mut first_comment_char: Option<char> = None;

    for line in content.lines() {
        let trimmed = line.trim();

        // Blank lines reset pending comments
        if trimmed.is_empty() {
            pending_comments.clear();
            continue;
        }

        // Comment lines (# or ;)
        if trimmed.starts_with(';') || trimmed.starts_with('#') {
            let ch = trimmed.chars().next().expect("non-empty trimmed");
            if first_comment_char.is_none() {
                first_comment_char = Some(ch);
            }
            let text = &trimmed[1..];
            let text = if text.starts_with(' ') { &text[1..] } else { text };
            pending_comments.push((text.to_string(), ch));
            continue;
        }

        // Section header [section]
        if trimmed.starts_with('[') {
            if let Some(end) = trimmed.find(']') {
                let section_name = trimmed[1..end].trim().to_string();
                current_section = Some(section_name);
                pending_comments.clear();
                continue;
            }
        }

        // Key-value pair
        if let Some((key, value, delimiter)) = split_ini_kv(trimmed) {
            if first_delimiter.is_none() {
                first_delimiter = Some(delimiter);
            }

            raw_entries.push(RawEntry {
                section: current_section.clone(),
                key,
                value,
                comments: std::mem::take(&mut pending_comments),
                delimiter,
            });
        }
    }

    // Now group plural keys and build IR entries
    let entries = group_entries(raw_entries)?;

    Ok(I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Ini,
            format_ext: Some(FormatExtension::Ini(IniExt {
                section: None,
                delimiter: first_delimiter,
                comment_char: first_comment_char,
            })),
            ..Default::default()
        },
        entries,
    })
}

/// Split a line into key, value, and delimiter.
fn split_ini_kv(line: &str) -> Option<(String, String, char)> {
    // Try = first, then :
    if let Some(pos) = line.find('=') {
        let key = line[..pos].trim().to_string();
        let value = line[pos + 1..].trim().to_string();
        if !key.is_empty() {
            return Some((key, value, '='));
        }
    }
    if let Some(pos) = line.find(':') {
        let key = line[..pos].trim().to_string();
        let value = line[pos + 1..].trim().to_string();
        if !key.is_empty() {
            return Some((key, value, ':'));
        }
    }
    None
}

/// Build the full IR key from section and key name.
fn make_full_key(section: &Option<String>, key: &str) -> String {
    match section {
        Some(sec) => format!("{}.{}", sec, key),
        None => key.to_string(),
    }
}

/// Group raw entries into IR entries, handling plural suffixes.
fn group_entries(raw_entries: Vec<RawEntry>) -> Result<IndexMap<String, I18nEntry>, ParseError> {
    // First pass: collect all full keys to detect plural groups
    // A plural group requires at least an _other form.
    let mut plural_bases: IndexMap<String, IndexMap<String, usize>> = IndexMap::new();
    let mut non_plural_indices: Vec<usize> = Vec::new();

    for (i, raw) in raw_entries.iter().enumerate() {
        let full_key = make_full_key(&raw.section, &raw.key);
        if let Some((base, category)) = strip_plural_suffix(&full_key) {
            plural_bases
                .entry(base.to_string())
                .or_default()
                .insert(category.to_string(), i);
        } else {
            non_plural_indices.push(i);
        }
    }

    let mut entries = IndexMap::new();

    // Process plural groups (only if they have _other)
    let mut consumed: std::collections::HashSet<usize> = std::collections::HashSet::new();
    for (base_key, category_map) in &plural_bases {
        if !category_map.contains_key("other") {
            // Not a valid plural group; treat each as a regular entry
            for &idx in category_map.values() {
                non_plural_indices.push(idx);
            }
            continue;
        }

        let mut ps = PluralSet::default();
        let mut comments = Vec::new();
        let mut section = None;
        let mut delimiter = '=';

        for (category, &idx) in category_map {
            consumed.insert(idx);
            let raw = &raw_entries[idx];
            if section.is_none() {
                section = raw.section.clone();
            }
            delimiter = raw.delimiter;

            match category.as_str() {
                "zero" => ps.zero = Some(raw.value.clone()),
                "one" => ps.one = Some(raw.value.clone()),
                "two" => ps.two = Some(raw.value.clone()),
                "few" => ps.few = Some(raw.value.clone()),
                "many" => ps.many = Some(raw.value.clone()),
                "other" => ps.other = raw.value.clone(),
                _ => {}
            }

            // Collect comments from the first entry (typically _one or _other)
            if comments.is_empty() {
                for (text, _ch) in &raw.comments {
                    if !text.is_empty() {
                        comments.push(Comment {
                            text: text.clone(),
                            role: CommentRole::General,
                            priority: None,
                            annotates: None,
                        });
                    }
                }
            }
        }

        let entry = I18nEntry {
            key: base_key.clone(),
            value: EntryValue::Plural(ps),
            comments,
            format_ext: Some(FormatExtension::Ini(IniExt {
                section,
                delimiter: Some(delimiter),
                comment_char: None,
            })),
            ..Default::default()
        };
        entries.insert(base_key.clone(), entry);
    }

    // Process non-plural entries
    // Sort to maintain original order
    non_plural_indices.sort();
    for idx in non_plural_indices {
        if consumed.contains(&idx) {
            continue;
        }
        let raw = &raw_entries[idx];
        let full_key = make_full_key(&raw.section, &raw.key);

        let mut entry_comments = Vec::new();
        for (text, _ch) in &raw.comments {
            if !text.is_empty() {
                entry_comments.push(Comment {
                    text: text.clone(),
                    role: CommentRole::General,
                    priority: None,
                    annotates: None,
                });
            }
        }

        let entry = I18nEntry {
            key: full_key.clone(),
            value: EntryValue::Simple(raw.value.clone()),
            comments: entry_comments,
            format_ext: Some(FormatExtension::Ini(IniExt {
                section: raw.section.clone(),
                delimiter: Some(raw.delimiter),
                comment_char: None,
            })),
            ..Default::default()
        };
        entries.insert(full_key, entry);
    }

    Ok(entries)
}

// ---------------------------------------------------------------------------
// Writer
// ---------------------------------------------------------------------------

fn write_ini(resource: &I18nResource) -> String {
    let mut out = String::new();

    // Determine default delimiter from resource metadata
    let default_delim = match &resource.metadata.format_ext {
        Some(FormatExtension::Ini(ext)) => ext.delimiter.unwrap_or('='),
        _ => '=',
    };

    // Group entries by section
    let mut sections: IndexMap<Option<String>, Vec<(&String, &I18nEntry)>> = IndexMap::new();

    for (key, entry) in &resource.entries {
        let section = match &entry.format_ext {
            Some(FormatExtension::Ini(ext)) => ext.section.clone(),
            _ => {
                // Try to derive section from dotted key
                if let Some(dot_pos) = key.find('.') {
                    Some(key[..dot_pos].to_string())
                } else {
                    None
                }
            }
        };
        sections.entry(section).or_default().push((key, entry));
    }

    let mut first_section = true;
    for (section, section_entries) in &sections {
        if !first_section {
            out.push('\n');
        }
        first_section = false;

        // Write section header
        if let Some(sec) = section {
            out.push_str(&format!("[{}]\n", sec));
        }

        for (full_key, entry) in section_entries {
            let delim = match &entry.format_ext {
                Some(FormatExtension::Ini(ext)) => ext.delimiter.unwrap_or(default_delim),
                _ => default_delim,
            };

            // Determine the local key (strip section prefix)
            let local_key = match section {
                Some(sec) => {
                    let prefix = format!("{}.", sec);
                    if full_key.starts_with(&prefix) {
                        &full_key[prefix.len()..]
                    } else {
                        full_key.as_str()
                    }
                }
                None => full_key.as_str(),
            };

            // Write comments
            for comment in &entry.comments {
                out.push_str(&format!("# {}\n", comment.text));
            }

            match &entry.value {
                EntryValue::Simple(val) => {
                    out.push_str(&format!("{} {} {}\n", local_key, delim, val));
                }
                EntryValue::Plural(ps) => {
                    // Write each plural form as separate key with suffix
                    if let Some(ref zero) = ps.zero {
                        out.push_str(&format!("{}_zero {} {}\n", local_key, delim, zero));
                    }
                    if let Some(ref one) = ps.one {
                        out.push_str(&format!("{}_one {} {}\n", local_key, delim, one));
                    }
                    if let Some(ref two) = ps.two {
                        out.push_str(&format!("{}_two {} {}\n", local_key, delim, two));
                    }
                    if let Some(ref few) = ps.few {
                        out.push_str(&format!("{}_few {} {}\n", local_key, delim, few));
                    }
                    if let Some(ref many) = ps.many {
                        out.push_str(&format!("{}_many {} {}\n", local_key, delim, many));
                    }
                    out.push_str(&format!("{}_other {} {}\n", local_key, delim, &ps.other));
                }
                EntryValue::Array(arr) => {
                    out.push_str(&format!("{} {} {}\n", local_key, delim, arr.join(", ")));
                }
                EntryValue::Select(ss) => {
                    let val = ss.cases.values().next().cloned().unwrap_or_default();
                    out.push_str(&format!("{} {} {}\n", local_key, delim, val));
                }
                EntryValue::MultiVariablePlural(mvp) => {
                    out.push_str(&format!("{} {} {}\n", local_key, delim, mvp.pattern));
                }
            }
        }
    }

    out
}

// ---------------------------------------------------------------------------
// Trait implementations
// ---------------------------------------------------------------------------

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        if extension == ".ini" {
            return Confidence::Definite;
        }
        if let Ok(s) = std::str::from_utf8(content) {
            // Heuristic: look for [section] headers
            let has_section = s.lines().any(|line| {
                let t = line.trim();
                t.starts_with('[') && t.ends_with(']')
            });
            let has_kv = s.lines().any(|line| {
                let t = line.trim();
                !t.is_empty()
                    && !t.starts_with('#')
                    && !t.starts_with(';')
                    && !t.starts_with('[')
                    && t.contains('=')
            });
            if has_section && has_kv {
                return Confidence::High;
            }
        }
        Confidence::None
    }

    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError> {
        let text = std::str::from_utf8(content)
            .map_err(|e| ParseError::InvalidFormat(format!("Invalid UTF-8: {}", e)))?;
        parse_ini_content(text)
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
        let output = write_ini(resource);
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
        assert_eq!(strip_plural_suffix("items_zero"), Some(("items", "zero")));
        assert_eq!(strip_plural_suffix("items"), None);
        assert_eq!(strip_plural_suffix("_other"), None);
    }

    #[test]
    fn test_split_ini_kv() {
        let result = split_ini_kv("greeting = Hello, World!");
        assert!(result.is_some());
        let (key, value, delim) = result.expect("should parse");
        assert_eq!(key, "greeting");
        assert_eq!(value, "Hello, World!");
        assert_eq!(delim, '=');
    }

    #[test]
    fn test_split_ini_kv_colon() {
        let result = split_ini_kv("greeting : Hello");
        assert!(result.is_some());
        let (key, value, delim) = result.expect("should parse");
        assert_eq!(key, "greeting");
        assert_eq!(value, "Hello");
        assert_eq!(delim, ':');
    }

    #[test]
    fn test_parse_basic_sections() {
        let input = "[general]\ngreeting = Hello\nfarewell = Goodbye\n\n[messages]\nwelcome = Welcome";
        let resource = parse_ini_content(input).expect("should parse");
        assert_eq!(resource.entries.len(), 3);
        assert!(resource.entries.contains_key("general.greeting"));
        assert!(resource.entries.contains_key("general.farewell"));
        assert!(resource.entries.contains_key("messages.welcome"));
    }

    #[test]
    fn test_parse_no_section() {
        let input = "greeting = Hello\nfarewell = Goodbye";
        let resource = parse_ini_content(input).expect("should parse");
        assert_eq!(resource.entries.len(), 2);
        assert!(resource.entries.contains_key("greeting"));
        assert!(resource.entries.contains_key("farewell"));
    }

    #[test]
    fn test_parse_comments() {
        let input = "; This is a comment\ngreeting = Hello";
        let resource = parse_ini_content(input).expect("should parse");
        let entry = resource.entries.get("greeting").expect("should exist");
        assert_eq!(entry.comments.len(), 1);
        assert_eq!(entry.comments[0].text, "This is a comment");
    }

    #[test]
    fn test_parse_plurals() {
        let input = "[plurals]\nitems_one = %d item\nitems_other = %d items";
        let resource = parse_ini_content(input).expect("should parse");
        assert_eq!(resource.entries.len(), 1);
        let entry = resource.entries.get("plurals.items").expect("should exist");
        match &entry.value {
            EntryValue::Plural(ps) => {
                assert_eq!(ps.one, Some("%d item".to_string()));
                assert_eq!(ps.other, "%d items".to_string());
            }
            _ => panic!("Expected plural value"),
        }
    }

    #[test]
    fn test_make_full_key() {
        assert_eq!(make_full_key(&Some("general".to_string()), "greeting"), "general.greeting");
        assert_eq!(make_full_key(&None, "greeting"), "greeting");
    }
}
