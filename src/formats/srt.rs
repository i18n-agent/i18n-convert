use crate::ir::*;
use super::*;

pub struct Parser;
pub struct Writer;

// ---------------------------------------------------------------------------
// SRT parser
// ---------------------------------------------------------------------------

/// Parse a single timecode like "00:00:01,000"
fn parse_timecode(s: &str) -> Result<String, ParseError> {
    let trimmed = s.trim();
    // Validate format: HH:MM:SS,mmm
    let parts: Vec<&str> = trimmed.splitn(2, ',').collect();
    if parts.len() != 2 {
        return Err(ParseError::InvalidFormat(format!(
            "Invalid SRT timecode (missing comma): '{trimmed}'"
        )));
    }
    let time_parts: Vec<&str> = parts[0].split(':').collect();
    if time_parts.len() != 3 {
        return Err(ParseError::InvalidFormat(format!(
            "Invalid SRT timecode (expected HH:MM:SS): '{trimmed}'"
        )));
    }
    // Validate each part is numeric
    for part in &time_parts {
        if part.parse::<u32>().is_err() {
            return Err(ParseError::InvalidFormat(format!(
                "Invalid SRT timecode component: '{part}'"
            )));
        }
    }
    if parts[1].trim().parse::<u32>().is_err() {
        return Err(ParseError::InvalidFormat(format!(
            "Invalid SRT timecode milliseconds: '{}'",
            parts[1]
        )));
    }
    Ok(trimmed.to_string())
}

/// Parse a timecode line: "HH:MM:SS,mmm --> HH:MM:SS,mmm"
fn parse_timecode_line(line: &str) -> Result<(String, String), ParseError> {
    let parts: Vec<&str> = line.split("-->").collect();
    if parts.len() != 2 {
        return Err(ParseError::InvalidFormat(format!(
            "Invalid SRT timecode line: '{line}'"
        )));
    }
    let start = parse_timecode(parts[0])?;
    let end = parse_timecode(parts[1])?;
    Ok((start, end))
}

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        if extension == ".srt" {
            return Confidence::Definite;
        }
        // Content-based detection
        if let Ok(s) = std::str::from_utf8(content) {
            // Look for pattern: digit(s) followed by a line with timecodes
            let re_pattern = regex::Regex::new(
                r"(?m)^\d+\s*\n\d{2}:\d{2}:\d{2},\d{3}\s*-->"
            );
            if let Ok(re) = re_pattern {
                if re.is_match(s) {
                    return Confidence::High;
                }
            }
        }
        Confidence::None
    }

    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError> {
        let s = std::str::from_utf8(content)
            .map_err(|e| ParseError::InvalidFormat(format!("Invalid UTF-8: {e}")))?;

        // Strip BOM if present
        let s = s.strip_prefix('\u{FEFF}').unwrap_or(s);

        let mut entries = IndexMap::new();

        // Split by blank lines (one or more empty lines between entries)
        let blocks: Vec<&str> = split_srt_blocks(s);

        for block in blocks {
            let block = block.trim();
            if block.is_empty() {
                continue;
            }

            let lines: Vec<&str> = block.lines().collect();
            if lines.len() < 2 {
                continue;
            }

            // Line 0: sequence number
            let seq_str = lines[0].trim();
            let sequence: u32 = seq_str.parse().map_err(|_| {
                ParseError::InvalidFormat(format!(
                    "Invalid SRT sequence number: '{seq_str}'"
                ))
            })?;

            // Line 1: timecodes
            let (start_time, end_time) = parse_timecode_line(lines[1])?;

            // Lines 2+: text content
            let text = if lines.len() > 2 {
                lines[2..].join("\n")
            } else {
                String::new()
            };

            let key = sequence.to_string();
            let mut entry = I18nEntry {
                key: key.clone(),
                value: EntryValue::Simple(text),
                ..Default::default()
            };

            // Store SRT metadata in properties
            entry
                .properties
                .insert("srt.sequence".to_string(), sequence.to_string());
            entry
                .properties
                .insert("srt.start_time".to_string(), start_time.clone());
            entry
                .properties
                .insert("srt.end_time".to_string(), end_time.clone());

            // Also store in format extension
            entry.format_ext = Some(FormatExtension::Srt(SrtExt {
                sequence_number: Some(sequence),
                start_time: Some(start_time),
                end_time: Some(end_time),
            }));

            entries.insert(key, entry);
        }

        Ok(I18nResource {
            metadata: ResourceMetadata {
                source_format: FormatId::Srt,
                format_ext: Some(FormatExtension::Srt(SrtExt::default())),
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
            custom_properties: true,
        }
    }
}

/// Split SRT content into blocks separated by one or more blank lines.
fn split_srt_blocks(s: &str) -> Vec<&str> {
    let mut blocks = Vec::new();
    let mut start = 0;
    let mut in_blank_run = false;
    let bytes = s.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        // Check if current line is blank
        let line_start = i;
        while i < bytes.len() && bytes[i] != b'\n' {
            i += 1;
        }
        let line = &s[line_start..i];
        let is_blank = line.trim().is_empty();

        // Consume the newline
        if i < bytes.len() && bytes[i] == b'\n' {
            i += 1;
        }

        if is_blank {
            if !in_blank_run {
                // End of a block
                let block = &s[start..line_start];
                if !block.trim().is_empty() {
                    blocks.push(block);
                }
                in_blank_run = true;
            }
        } else {
            if in_blank_run {
                start = line_start;
                in_blank_run = false;
            }
        }
    }

    // Add the last block
    if !in_blank_run {
        let block = &s[start..];
        if !block.trim().is_empty() {
            blocks.push(block);
        }
    }

    blocks
}

// ---------------------------------------------------------------------------
// Writer
// ---------------------------------------------------------------------------

impl FormatWriter for Writer {
    fn write(&self, resource: &I18nResource) -> Result<Vec<u8>, WriteError> {
        let mut output = String::new();
        let mut seq_counter: u32 = 0;

        for (_, entry) in &resource.entries {
            if seq_counter > 0 {
                output.push('\n');
            }

            let text = match &entry.value {
                EntryValue::Simple(s) => s.clone(),
                EntryValue::Plural(ps) => ps.other.clone(),
                EntryValue::Array(arr) => arr.join("\n"),
                EntryValue::Select(ss) => {
                    ss.cases.get("other").cloned().unwrap_or_default()
                }
                EntryValue::MultiVariablePlural(mvp) => mvp.pattern.clone(),
            };

            // Get sequence number from properties, format_ext, or generate
            let sequence = entry
                .properties
                .get("srt.sequence")
                .and_then(|s| s.parse::<u32>().ok())
                .or_else(|| {
                    if let Some(FormatExtension::Srt(ext)) = &entry.format_ext {
                        ext.sequence_number
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| {
                    seq_counter += 1;
                    seq_counter
                });

            // Ensure seq_counter tracks properly
            if sequence > seq_counter {
                seq_counter = sequence;
            }

            // Get timecodes from properties, format_ext, or generate synthetic ones
            let start_time = entry
                .properties
                .get("srt.start_time")
                .cloned()
                .or_else(|| {
                    if let Some(FormatExtension::Srt(ext)) = &entry.format_ext {
                        ext.start_time.clone()
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| {
                    let secs = (sequence - 1) as u32;
                    format!("00:00:{:02},000", secs)
                });

            let end_time = entry
                .properties
                .get("srt.end_time")
                .cloned()
                .or_else(|| {
                    if let Some(FormatExtension::Srt(ext)) = &entry.format_ext {
                        ext.end_time.clone()
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| {
                    let secs = sequence as u32;
                    format!("00:00:{:02},000", secs)
                });

            // Write entry
            output.push_str(&format!("{}\n", sequence));
            output.push_str(&format!("{} --> {}\n", start_time, end_time));
            output.push_str(&text);
            output.push('\n');
        }

        Ok(output.into_bytes())
    }

    fn capabilities(&self) -> FormatCapabilities {
        Parser.capabilities()
    }
}
