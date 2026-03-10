use super::*;

pub struct Parser;
pub struct Writer;

// ---------------------------------------------------------------------------
// Parser helpers
// ---------------------------------------------------------------------------

/// Known column names that represent the "key" column.
const KEY_COLUMN_NAMES: &[&str] = &["key", "id", "name", "identifier", "msgid", "string_id"];

/// Known column names that represent a "comment" column.
const COMMENT_COLUMN_NAMES: &[&str] = &["comment", "comments", "note", "notes", "description"];

/// Known locale codes (common ones) used to detect value columns.
const LOCALE_PATTERNS: &[&str] = &[
    "en", "de", "fr", "es", "it", "pt", "ja", "ko", "zh", "ru", "ar", "nl", "pl", "sv", "da",
    "fi", "nb", "tr", "th", "vi", "id", "ms", "hi", "bn", "uk", "cs", "ro", "hu", "el", "he",
    "ca", "hr", "sk", "sl", "bg", "sr", "lt", "lv", "et",
    // With regions
    "en-us", "en-gb", "en-au", "pt-br", "zh-cn", "zh-tw", "zh-hans", "zh-hant",
    "es-mx", "es-ar", "fr-ca", "fr-fr", "de-de", "de-at", "de-ch",
    // Underscore variants
    "en_us", "en_gb", "pt_br", "zh_cn", "zh_tw",
];

/// Check if a column name looks like a locale code.
fn is_locale_column(name: &str) -> bool {
    let lower = name.to_lowercase();
    // Direct match
    if LOCALE_PATTERNS.contains(&lower.as_str()) {
        return true;
    }
    // Also match "value" as a generic value column
    if lower == "value" || lower == "text" || lower == "translation" || lower == "target" {
        return true;
    }
    false
}

/// Detect the delimiter by checking if the content has tabs.
fn detect_delimiter(content: &str, extension: &str) -> u8 {
    if extension == ".tsv" {
        return b'\t';
    }
    // Heuristic: if the first line contains tabs and no commas, it's TSV
    if let Some(first_line) = content.lines().next() {
        let tabs = first_line.chars().filter(|&c| c == '\t').count();
        let commas = first_line.chars().filter(|&c| c == ',').count();
        if tabs > 0 && commas == 0 {
            return b'\t';
        }
    }
    b','
}

/// Check if content starts with a UTF-8 BOM.
fn has_bom(content: &[u8]) -> bool {
    content.starts_with(&[0xEF, 0xBB, 0xBF])
}

/// Strip the UTF-8 BOM if present.
fn strip_bom(content: &[u8]) -> &[u8] {
    if has_bom(content) {
        &content[3..]
    } else {
        content
    }
}

/// Parse CSV content into an I18nResource.
fn parse_csv(content: &[u8], extension: &str) -> Result<I18nResource, ParseError> {
    let bom_present = has_bom(content);
    let clean_content = strip_bom(content);

    let text = std::str::from_utf8(clean_content)
        .map_err(|e| ParseError::InvalidFormat(format!("Invalid UTF-8: {}", e)))?;

    let delimiter = detect_delimiter(text, extension);

    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .flexible(true)
        .has_headers(true)
        .from_reader(clean_content);

    // Read headers
    let headers = reader
        .headers()
        .map_err(|e| ParseError::InvalidFormat(format!("Failed to read CSV headers: {}", e)))?
        .clone();

    if headers.is_empty() {
        return Err(ParseError::InvalidFormat(
            "CSV file has no headers".to_string(),
        ));
    }

    // Find key column, value column, and comment column
    let mut key_col: Option<usize> = None;
    let mut value_col: Option<usize> = None;
    let mut comment_col: Option<usize> = None;
    let mut value_col_name: Option<String> = None;
    let mut key_col_name: Option<String> = None;

    // First pass: find key and comment columns
    for (i, header) in headers.iter().enumerate() {
        let lower = header.to_lowercase().trim().to_string();
        if key_col.is_none() && KEY_COLUMN_NAMES.contains(&lower.as_str()) {
            key_col = Some(i);
            key_col_name = Some(header.trim().to_string());
        }
        if comment_col.is_none() && COMMENT_COLUMN_NAMES.contains(&lower.as_str()) {
            comment_col = Some(i);
        }
    }

    // Second pass: find value column (first non-key, non-comment column that looks like a locale or value)
    for (i, header) in headers.iter().enumerate() {
        if Some(i) == key_col || Some(i) == comment_col {
            continue;
        }
        let trimmed = header.trim();
        if is_locale_column(trimmed) {
            value_col = Some(i);
            value_col_name = Some(trimmed.to_string());
            break;
        }
    }

    // Fallback: if no key column found, use column 0
    if key_col.is_none() {
        key_col = Some(0);
        key_col_name = Some(headers.get(0).unwrap_or("key").trim().to_string());
    }

    // Fallback: if no value column found, use the first column that isn't key or comment
    if value_col.is_none() {
        for i in 0..headers.len() {
            if Some(i) != key_col && Some(i) != comment_col {
                value_col = Some(i);
                value_col_name = Some(headers.get(i).unwrap_or("value").trim().to_string());
                break;
            }
        }
    }

    let key_col = key_col.ok_or_else(|| {
        ParseError::InvalidFormat("Could not determine key column".to_string())
    })?;
    let value_col = value_col.ok_or_else(|| {
        ParseError::InvalidFormat("Could not determine value column".to_string())
    })?;

    // Determine locale from value column name
    let locale = value_col_name.as_ref().and_then(|name| {
        let lower = name.to_lowercase();
        if LOCALE_PATTERNS.contains(&lower.as_str()) {
            Some(name.clone())
        } else {
            None
        }
    });

    // Parse records
    let mut entries = IndexMap::new();

    for result in reader.records() {
        let record = result
            .map_err(|e| ParseError::InvalidFormat(format!("CSV parse error: {}", e)))?;

        let key = record.get(key_col).unwrap_or("").trim().to_string();
        if key.is_empty() {
            continue; // Skip rows with empty keys
        }

        let value = record.get(value_col).unwrap_or("").to_string();

        let mut entry = I18nEntry {
            key: key.clone(),
            value: EntryValue::Simple(value),
            ..Default::default()
        };

        // Add comment if present
        if let Some(cc) = comment_col {
            if let Some(comment_text) = record.get(cc) {
                let comment_text = comment_text.trim().to_string();
                if !comment_text.is_empty() {
                    entry.comments.push(Comment {
                        text: comment_text,
                        role: CommentRole::General,
                        priority: None,
                        annotates: None,
                    });
                }
            }
        }

        entries.insert(key, entry);
    }

    let delimiter_char = if delimiter == b'\t' { '\t' } else { ',' };

    Ok(I18nResource {
        metadata: ResourceMetadata {
            source_format: FormatId::Csv,
            locale,
            format_ext: Some(FormatExtension::Csv(CsvExt {
                delimiter: Some(delimiter_char),
                key_column: key_col_name,
                value_column: value_col_name,
                has_bom: Some(bom_present),
            })),
            ..Default::default()
        },
        entries,
    })
}

// ---------------------------------------------------------------------------
// Writer helpers
// ---------------------------------------------------------------------------

fn write_csv(resource: &I18nResource) -> Result<Vec<u8>, WriteError> {
    // Determine column names from metadata
    let (key_col_name, value_col_name, delimiter) = if let Some(FormatExtension::Csv(ext)) =
        &resource.metadata.format_ext
    {
        let kc = ext
            .key_column
            .clone()
            .unwrap_or_else(|| "key".to_string());
        let vc = ext
            .value_column
            .clone()
            .unwrap_or_else(|| "value".to_string());
        let delim = ext.delimiter.map(|c| {
            if c == '\t' {
                b'\t'
            } else {
                b','
            }
        }).unwrap_or(b',');
        (kc, vc, delim)
    } else {
        // Use locale from metadata if available
        let vc = resource
            .metadata
            .locale
            .clone()
            .unwrap_or_else(|| "value".to_string());
        ("key".to_string(), vc, b',')
    };

    // Check if any entries have comments
    let has_comments = resource
        .entries
        .values()
        .any(|e| !e.comments.is_empty());

    let mut wtr = csv::WriterBuilder::new()
        .delimiter(delimiter)
        .from_writer(Vec::new());

    // Write header
    if has_comments {
        wtr.write_record([&key_col_name, &value_col_name, "comment"])
            .map_err(|e| WriteError::Serialization(format!("CSV write error: {}", e)))?;
    } else {
        wtr.write_record([&key_col_name, &value_col_name])
            .map_err(|e| WriteError::Serialization(format!("CSV write error: {}", e)))?;
    }

    // Write entries
    for (_key, entry) in &resource.entries {
        let value_str = match &entry.value {
            EntryValue::Simple(s) => s.clone(),
            EntryValue::Plural(ps) => ps.other.clone(),
            EntryValue::Array(arr) => arr.join(", "),
            EntryValue::Select(ss) => ss.cases.values().next().cloned().unwrap_or_default(),
            EntryValue::MultiVariablePlural(mvp) => mvp.pattern.clone(),
        };

        if has_comments {
            let comment = entry
                .comments
                .first()
                .map(|c| c.text.clone())
                .unwrap_or_default();
            wtr.write_record([&entry.key, &value_str, &comment])
                .map_err(|e| WriteError::Serialization(format!("CSV write error: {}", e)))?;
        } else {
            wtr.write_record([&entry.key, &value_str])
                .map_err(|e| WriteError::Serialization(format!("CSV write error: {}", e)))?;
        }
    }

    wtr.flush()
        .map_err(|e| WriteError::Serialization(format!("CSV flush error: {}", e)))?;

    let output = wtr
        .into_inner()
        .map_err(|e| WriteError::Serialization(format!("CSV finalize error: {}", e)))?;

    Ok(output)
}

// ---------------------------------------------------------------------------
// Trait implementations
// ---------------------------------------------------------------------------

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        if extension == ".csv" {
            return Confidence::High;
        }
        if extension == ".tsv" {
            return Confidence::High;
        }
        if let Ok(s) = std::str::from_utf8(strip_bom(content)) {
            // Look for CSV-like header with known column names
            if let Some(first_line) = s.lines().next() {
                let lower = first_line.to_lowercase();
                let has_key_col = KEY_COLUMN_NAMES.iter().any(|&k| lower.contains(k));
                if has_key_col {
                    return Confidence::Low;
                }
            }
        }
        Confidence::None
    }

    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError> {
        // Try to infer extension from content (default to .csv)
        let extension = if let Ok(s) = std::str::from_utf8(strip_bom(content)) {
            if let Some(first_line) = s.lines().next() {
                let tabs = first_line.chars().filter(|&c| c == '\t').count();
                let commas = first_line.chars().filter(|&c| c == ',').count();
                if tabs > 0 && commas == 0 {
                    ".tsv"
                } else {
                    ".csv"
                }
            } else {
                ".csv"
            }
        } else {
            ".csv"
        };
        parse_csv(content, extension)
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
        write_csv(resource)
    }

    fn capabilities(&self) -> FormatCapabilities {
        Parser.capabilities()
    }
}
