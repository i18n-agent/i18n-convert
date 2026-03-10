use super::*;
use std::io::Cursor;

pub struct Parser;
pub struct Writer;

// ---------------------------------------------------------------------------
// Header detection helpers
// ---------------------------------------------------------------------------

/// Column role detected from header text.
#[derive(Debug, Clone, Copy, PartialEq)]
enum ColumnRole {
    Key,
    Value,
    Comment,
}

/// Detect the role of a header column by its name (case-insensitive).
fn detect_column_role(header: &str) -> Option<ColumnRole> {
    let lower = header.trim().to_lowercase();
    match lower.as_str() {
        "key" | "id" | "name" | "identifier" | "string_key" => Some(ColumnRole::Key),
        "value" | "text" | "translation" | "message" | "string" => Some(ColumnRole::Value),
        "comment" | "note" | "description" | "notes" | "comments" => Some(ColumnRole::Comment),
        _ => {
            // Locale codes like "en", "en-US", "zh-Hans"
            if is_locale_code(&lower) {
                Some(ColumnRole::Value)
            } else {
                None
            }
        }
    }
}

/// Simple check for common locale code patterns.
fn is_locale_code(s: &str) -> bool {
    // Matches patterns like "en", "en-us", "zh-hans", "pt-br"
    let parts: Vec<&str> = s.split('-').collect();
    match parts.len() {
        1 => parts[0].len() == 2 && parts[0].chars().all(|c| c.is_ascii_lowercase()),
        2 => {
            let lang = parts[0];
            let region = parts[1];
            lang.len() == 2
                && lang.chars().all(|c| c.is_ascii_lowercase())
                && (region.len() == 2 || region.len() == 4)
                && region.chars().all(|c| c.is_ascii_alphanumeric())
        }
        _ => false,
    }
}

/// Extract a string value from a calamine Data cell.
fn cell_to_string(cell: &calamine::Data) -> Option<String> {
    use calamine::DataType;
    if cell.is_empty() {
        return None;
    }
    cell.as_string()
}

// ---------------------------------------------------------------------------
// Parser implementation
// ---------------------------------------------------------------------------

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        let ext_lower = extension.to_lowercase();
        if ext_lower == ".xlsx" || ext_lower == ".xls" {
            return Confidence::Definite;
        }
        // Try to open as workbook from bytes
        if content.len() > 4 {
            let cursor = Cursor::new(content.to_vec());
            if calamine::open_workbook_auto_from_rs(cursor).is_ok() {
                return Confidence::High;
            }
        }
        Confidence::None
    }

    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError> {
        use calamine::{DataType, Reader};

        let cursor = Cursor::new(content.to_vec());
        let mut workbook = calamine::open_workbook_auto_from_rs(cursor).map_err(|e| {
            ParseError::InvalidFormat(format!("Failed to open Excel workbook: {e}"))
        })?;

        // Get first sheet
        let sheet_names = workbook.sheet_names().to_vec();
        if sheet_names.is_empty() {
            return Err(ParseError::InvalidFormat(
                "No sheets found in workbook".to_string(),
            ));
        }
        let sheet_name = sheet_names[0].clone();

        let range = workbook.worksheet_range(&sheet_name).map_err(|e| {
            ParseError::InvalidFormat(format!("Failed to read sheet '{sheet_name}': {e}"))
        })?;

        let mut rows_iter = range.rows();

        // First row is the header
        let header_row = rows_iter.next().ok_or_else(|| {
            ParseError::InvalidFormat("Sheet is empty — no header row".to_string())
        })?;

        // Detect column roles from header
        let mut key_col: Option<usize> = None;
        let mut value_col: Option<usize> = None;
        let mut comment_col: Option<usize> = None;

        for (i, cell) in header_row.iter().enumerate() {
            if let Some(header_text) = cell_to_string(cell) {
                match detect_column_role(&header_text) {
                    Some(ColumnRole::Key) if key_col.is_none() => {
                        key_col = Some(i);
                    }
                    Some(ColumnRole::Value) if value_col.is_none() => {
                        value_col = Some(i);
                    }
                    Some(ColumnRole::Comment) if comment_col.is_none() => {
                        comment_col = Some(i);
                    }
                    _ => {}
                }
            }
        }

        let key_col = key_col.ok_or_else(|| {
            ParseError::InvalidFormat(
                "No key column found in header row (expected: key, id, name)".to_string(),
            )
        })?;
        let value_col = value_col.ok_or_else(|| {
            ParseError::InvalidFormat(
                "No value column found in header row (expected: value, text, translation, or locale code)".to_string(),
            )
        })?;

        // Parse data rows
        let mut entries = IndexMap::new();

        for row in rows_iter {
            let key = row.get(key_col).and_then(|c| {
                if c.is_empty() {
                    None
                } else {
                    cell_to_string(c)
                }
            });

            let key = match key {
                Some(k) if !k.is_empty() => k,
                _ => continue, // Skip rows with empty keys
            };

            let value = row
                .get(value_col)
                .and_then(cell_to_string)
                .unwrap_or_default();

            let comment_text = comment_col
                .and_then(|col| row.get(col))
                .and_then(cell_to_string);

            let mut entry = I18nEntry {
                key: key.clone(),
                value: EntryValue::Simple(value),
                ..Default::default()
            };

            if let Some(text) = comment_text {
                if !text.is_empty() {
                    entry.comments.push(Comment {
                        text,
                        role: CommentRole::General,
                        priority: None,
                        annotates: None,
                    });
                }
            }

            entries.insert(key, entry);
        }

        Ok(I18nResource {
            metadata: ResourceMetadata {
                source_format: FormatId::Excel,
                format_ext: Some(FormatExtension::Excel(ExcelExt {
                    sheet_name: Some(sheet_name),
                    key_column: Some(key_col as u32),
                    value_column: Some(value_col as u32),
                })),
                ..Default::default()
            },
            entries,
        })
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

// ---------------------------------------------------------------------------
// Writer implementation
// ---------------------------------------------------------------------------

impl FormatWriter for Writer {
    fn write(&self, resource: &I18nResource) -> Result<Vec<u8>, WriteError> {
        let mut workbook = rust_xlsxwriter::Workbook::new();
        let worksheet = workbook.add_worksheet();

        // Determine if we have any comments
        let has_comments = resource.entries.values().any(|e| !e.comments.is_empty());

        // Write header row
        worksheet
            .write_string(0, 0, "key")
            .map_err(|e| WriteError::Serialization(format!("Failed to write header: {e}")))?;
        worksheet
            .write_string(0, 1, "value")
            .map_err(|e| WriteError::Serialization(format!("Failed to write header: {e}")))?;
        if has_comments {
            worksheet
                .write_string(0, 2, "comment")
                .map_err(|e| WriteError::Serialization(format!("Failed to write header: {e}")))?;
        }

        // Write data rows
        for (row_idx, (_key, entry)) in resource.entries.iter().enumerate() {
            let row = (row_idx + 1) as u32;

            worksheet
                .write_string(row, 0, &entry.key)
                .map_err(|e| WriteError::Serialization(format!("Failed to write key: {e}")))?;

            let value_str = match &entry.value {
                EntryValue::Simple(s) => s.clone(),
                EntryValue::Plural(ps) => ps.other.clone(),
                EntryValue::Array(arr) => arr.join(", "),
                EntryValue::Select(ss) => ss.cases.get("other").cloned().unwrap_or_default(),
                EntryValue::MultiVariablePlural(mvp) => mvp.pattern.clone(),
            };

            worksheet
                .write_string(row, 1, &value_str)
                .map_err(|e| WriteError::Serialization(format!("Failed to write value: {e}")))?;

            if has_comments {
                let comment_text: String = entry
                    .comments
                    .iter()
                    .map(|c| c.text.as_str())
                    .collect::<Vec<_>>()
                    .join("; ");
                if !comment_text.is_empty() {
                    worksheet.write_string(row, 2, &comment_text).map_err(|e| {
                        WriteError::Serialization(format!("Failed to write comment: {e}"))
                    })?;
                }
            }
        }

        // Set sheet name from extension data if available
        if let Some(FormatExtension::Excel(ext)) = &resource.metadata.format_ext {
            if let Some(name) = &ext.sheet_name {
                worksheet.set_name(name).map_err(|e| {
                    WriteError::Serialization(format!("Failed to set sheet name: {e}"))
                })?;
            }
        }

        let buf = workbook
            .save_to_buffer()
            .map_err(|e| WriteError::Serialization(format!("Failed to save workbook: {e}")))?;

        Ok(buf)
    }

    fn capabilities(&self) -> FormatCapabilities {
        Parser.capabilities()
    }
}
