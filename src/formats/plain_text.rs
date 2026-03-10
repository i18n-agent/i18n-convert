use crate::ir::*;
use super::*;
use indexmap::IndexMap;

pub struct Parser;
pub struct Writer;

/// The delimiter line used to separate multiple sections in a plain text file.
const SECTION_DELIMITER: &str = "---";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Detect the line ending used in the content.
/// Returns "\r\n" if Windows-style endings are found, "\n" otherwise.
fn detect_line_ending(content: &str) -> String {
    if content.contains("\r\n") {
        "\r\n".to_string()
    } else {
        "\n".to_string()
    }
}

/// Normalize line endings to \n for internal processing.
fn normalize_line_endings(content: &str) -> String {
    content.replace("\r\n", "\n")
}

/// Convert \n line endings back to the detected line ending.
fn restore_line_endings(content: &str, line_ending: &str) -> String {
    if line_ending == "\r\n" {
        content.replace('\n', "\r\n")
    } else {
        content.to_string()
    }
}

// ---------------------------------------------------------------------------
// Trait implementations
// ---------------------------------------------------------------------------

impl FormatParser for Parser {
    fn detect(&self, extension: &str, _content: &[u8]) -> Confidence {
        if extension == ".txt" {
            Confidence::Definite
        } else {
            Confidence::None
        }
    }

    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError> {
        let text = std::str::from_utf8(content)
            .map_err(|e| ParseError::InvalidFormat(format!("Invalid UTF-8: {}", e)))?;

        let line_ending = detect_line_ending(text);
        let normalized = normalize_line_endings(text);

        let mut entries = IndexMap::new();

        if normalized.is_empty() {
            // Empty file: no entries
            return Ok(I18nResource {
                metadata: ResourceMetadata {
                    source_format: FormatId::PlainText,
                    format_ext: Some(FormatExtension::PlainText(PlainTextExt {
                        line_ending: Some(line_ending),
                    })),
                    ..Default::default()
                },
                entries,
            });
        }

        // Check if the content has section delimiters
        let has_delimiters = normalized.lines().any(|l| l.trim() == SECTION_DELIMITER);

        if has_delimiters {
            // Split on "---" delimiter lines into sections
            let mut sections: Vec<String> = Vec::new();
            let mut current_section = String::new();

            for line in normalized.lines() {
                if line.trim() == SECTION_DELIMITER {
                    sections.push(current_section.trim_end_matches('\n').to_string());
                    current_section = String::new();
                } else {
                    if !current_section.is_empty() {
                        current_section.push('\n');
                    }
                    current_section.push_str(line);
                }
            }
            // Push the last section
            sections.push(current_section.trim_end_matches('\n').to_string());

            for (i, section) in sections.iter().enumerate() {
                let key = format!("content.{}", i);
                entries.insert(
                    key.clone(),
                    I18nEntry {
                        key,
                        value: EntryValue::Simple(section.clone()),
                        ..Default::default()
                    },
                );
            }
        } else {
            // Single content entry: the entire file content
            // Strip a single trailing newline if present (common in text files),
            // but preserve internal content exactly.
            let value = normalized.strip_suffix('\n').unwrap_or(&normalized).to_string();
            entries.insert(
                "content".to_string(),
                I18nEntry {
                    key: "content".to_string(),
                    value: EntryValue::Simple(value),
                    ..Default::default()
                },
            );
        }

        Ok(I18nResource {
            metadata: ResourceMetadata {
                source_format: FormatId::PlainText,
                format_ext: Some(FormatExtension::PlainText(PlainTextExt {
                    line_ending: Some(line_ending),
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
            comments: false,
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
        // Determine line ending from extension
        let line_ending = match &resource.metadata.format_ext {
            Some(FormatExtension::PlainText(ext)) => {
                ext.line_ending.as_deref().unwrap_or("\n")
            }
            _ => "\n",
        };

        if resource.entries.is_empty() {
            return Ok(Vec::new());
        }

        // Check if we have sectioned content (content.0, content.1, etc.)
        let has_sections = resource.entries.keys().any(|k| k.starts_with("content."));

        let output = if has_sections {
            // Collect sections in order
            let mut sections: Vec<(usize, &str)> = Vec::new();
            for (key, entry) in &resource.entries {
                if let Some(idx_str) = key.strip_prefix("content.") {
                    if let Ok(idx) = idx_str.parse::<usize>() {
                        if let EntryValue::Simple(ref s) = entry.value {
                            sections.push((idx, s.as_str()));
                        }
                    }
                }
            }
            sections.sort_by_key(|(idx, _)| *idx);

            let joined = sections
                .iter()
                .map(|(_, s)| *s)
                .collect::<Vec<_>>()
                .join(&format!("\n{}\n", SECTION_DELIMITER));

            restore_line_endings(&joined, line_ending)
        } else {
            // Single content entry
            let text = resource
                .entries
                .values()
                .next()
                .map(|e| match &e.value {
                    EntryValue::Simple(s) => s.as_str(),
                    _ => "",
                })
                .unwrap_or("");

            restore_line_endings(text, line_ending)
        };

        // Add trailing newline
        let mut bytes = output.into_bytes();
        let newline_bytes = line_ending.as_bytes();
        if !bytes.ends_with(newline_bytes) {
            bytes.extend_from_slice(newline_bytes);
        }

        Ok(bytes)
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
    use crate::formats::{FormatParser, FormatWriter};

    #[test]
    fn test_detect_txt() {
        let parser = Parser;
        assert_eq!(parser.detect(".txt", b""), Confidence::Definite);
    }

    #[test]
    fn test_detect_non_txt() {
        let parser = Parser;
        assert_eq!(parser.detect(".json", b""), Confidence::None);
        assert_eq!(parser.detect(".yml", b""), Confidence::None);
    }

    #[test]
    fn test_parse_simple() {
        let parser = Parser;
        let content = b"Hello, world!\n";
        let resource = parser.parse(content).unwrap();

        assert_eq!(resource.metadata.source_format, FormatId::PlainText);
        assert_eq!(resource.entries.len(), 1);
        assert_eq!(
            resource.entries["content"].value,
            EntryValue::Simple("Hello, world!".to_string())
        );
    }

    #[test]
    fn test_parse_multiline() {
        let parser = Parser;
        let content = b"Line one.\nLine two.\nLine three.\n";
        let resource = parser.parse(content).unwrap();

        assert_eq!(resource.entries.len(), 1);
        assert_eq!(
            resource.entries["content"].value,
            EntryValue::Simple("Line one.\nLine two.\nLine three.".to_string())
        );
    }

    #[test]
    fn test_parse_sections() {
        let parser = Parser;
        let content = b"Section one.\n---\nSection two.\n---\nSection three.";
        let resource = parser.parse(content).unwrap();

        assert_eq!(resource.entries.len(), 3);
        assert_eq!(
            resource.entries["content.0"].value,
            EntryValue::Simple("Section one.".to_string())
        );
        assert_eq!(
            resource.entries["content.1"].value,
            EntryValue::Simple("Section two.".to_string())
        );
        assert_eq!(
            resource.entries["content.2"].value,
            EntryValue::Simple("Section three.".to_string())
        );
    }

    #[test]
    fn test_parse_empty() {
        let parser = Parser;
        let resource = parser.parse(b"").unwrap();
        assert_eq!(resource.entries.len(), 0);
    }

    #[test]
    fn test_detect_line_ending_lf() {
        assert_eq!(detect_line_ending("hello\nworld\n"), "\n");
    }

    #[test]
    fn test_detect_line_ending_crlf() {
        assert_eq!(detect_line_ending("hello\r\nworld\r\n"), "\r\n");
    }

    #[test]
    fn test_roundtrip_simple() {
        let parser = Parser;
        let writer = Writer;
        let content = b"Hello, world!\n";
        let resource = parser.parse(content).unwrap();
        let output = writer.write(&resource).unwrap();
        let resource2 = parser.parse(&output).unwrap();

        assert_eq!(resource.entries.len(), resource2.entries.len());
        for (key, entry) in &resource.entries {
            assert_eq!(entry.value, resource2.entries[key].value);
        }
    }
}
