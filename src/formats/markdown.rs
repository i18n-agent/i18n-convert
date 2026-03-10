use super::*;

pub struct Parser;
pub struct Writer;

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

/// Convert heading text to a kebab-case key segment.
/// e.g. "Getting Started" -> "getting-started"
///      "How do I reset my password?" -> "how-do-i-reset-my-password"
fn heading_to_key(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut prev_was_separator = false;

    for ch in text.chars() {
        if ch.is_alphanumeric() {
            if prev_was_separator && !result.is_empty() {
                result.push('-');
            }
            result.push(ch.to_lowercase().next().unwrap_or(ch));
            prev_was_separator = false;
        } else if ch == ' ' || ch == '_' || ch == '-' {
            prev_was_separator = true;
        }
        // Other punctuation (?, !, etc.) is silently dropped
    }

    result
}

/// Count the heading level from a line starting with '#' characters.
/// Returns (level, heading_text) or None if not a heading line.
fn parse_heading_line(line: &str) -> Option<(usize, String)> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return None;
    }

    let level = trimmed.chars().take_while(|&c| c == '#').count();
    if level == 0 || level > 6 {
        return None;
    }

    // Must have a space after the '#' chars (standard Markdown)
    let rest = &trimmed[level..];
    if !rest.starts_with(' ') && !rest.is_empty() {
        return None;
    }

    let text = rest.trim().to_string();
    Some((level, text))
}

/// Parse front matter delimited by `---` markers.
/// Returns (front_matter_content, remaining_content).
fn extract_front_matter(content: &str) -> (Option<String>, &str) {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return (None, content);
    }

    // Find the opening ---
    let after_first = &trimmed[3..];
    let after_first = after_first.trim_start_matches(|c: char| c == '-'); // handle ---- etc.

    // Find the closing ---
    if let Some(end_pos) = after_first.find("\n---") {
        let fm_content = after_first[..end_pos].trim_start_matches('\n');
        let fm_content = fm_content.trim_end();

        // Find where after the closing ---
        let remaining_start = end_pos + 1; // skip the \n
        let after_closing = &after_first[remaining_start..];
        // Skip the --- and any trailing content on that line
        let after_closing = if let Some(newline_pos) = after_closing.find('\n') {
            &after_closing[newline_pos + 1..]
        } else {
            ""
        };

        (Some(fm_content.to_string()), after_closing)
    } else {
        // No closing --- found, treat as not front matter
        (None, content)
    }
}

/// Build a full dot-separated key from the heading stack and a new heading.
fn build_key(heading_stack: &[(usize, String)], level: usize, text: &str) -> String {
    let key_segment = heading_to_key(text);

    // Find parent headings: all headings with level < current level
    let mut parts: Vec<&str> = Vec::new();
    for (h_level, h_key) in heading_stack {
        if *h_level < level {
            parts.push(h_key);
        } else {
            break;
        }
    }
    parts.push(&key_segment);
    parts.join(".")
}

// ---------------------------------------------------------------------------
// Parser implementation
// ---------------------------------------------------------------------------

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        let ext_lower = extension.to_lowercase();
        if ext_lower == ".md" || ext_lower == ".markdown" {
            // Check for heading patterns to increase confidence
            if let Ok(s) = std::str::from_utf8(content) {
                if s.contains("\n# ") || s.starts_with("# ") || s.starts_with("---") {
                    return Confidence::High;
                }
            }
            return Confidence::Low;
        }
        Confidence::None
    }

    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError> {
        let text = std::str::from_utf8(content)
            .map_err(|e| ParseError::InvalidFormat(format!("Invalid UTF-8: {e}")))?;

        let (front_matter, body) = extract_front_matter(text);

        // Extract locale from front matter if present
        let locale = front_matter.as_ref().and_then(|fm| {
            for line in fm.lines() {
                let trimmed = line.trim();
                if let Some(rest) = trimmed.strip_prefix("locale:") {
                    return Some(rest.trim().to_string());
                }
            }
            None
        });

        let mut entries = IndexMap::new();
        // Stack of (level, key_segment) for building nested keys
        let mut heading_stack: Vec<(usize, String)> = Vec::new();
        let mut current_content_lines: Vec<String> = Vec::new();
        let mut current_key: Option<String> = None;

        let lines: Vec<&str> = body.lines().collect();

        for line in &lines {
            if let Some((level, heading_text)) = parse_heading_line(line) {
                // Flush previous section
                if let Some(key) = current_key.take() {
                    let value = finalize_content(&current_content_lines);
                    if !value.is_empty() {
                        entries.insert(
                            key.clone(),
                            I18nEntry {
                                key,
                                value: EntryValue::Simple(value),
                                ..Default::default()
                            },
                        );
                    }
                    current_content_lines.clear();
                }

                // Update heading stack: remove headings at same or deeper level
                while heading_stack
                    .last()
                    .map_or(false, |(l, _)| *l >= level)
                {
                    heading_stack.pop();
                }

                let key_segment = heading_to_key(&heading_text);
                let full_key = build_key(&heading_stack, level, &heading_text);

                heading_stack.push((level, key_segment));
                current_key = Some(full_key);
            } else {
                // Accumulate content lines
                current_content_lines.push(line.to_string());
            }
        }

        // Flush final section
        if let Some(key) = current_key.take() {
            let value = finalize_content(&current_content_lines);
            if !value.is_empty() {
                entries.insert(
                    key.clone(),
                    I18nEntry {
                        key,
                        value: EntryValue::Simple(value),
                        ..Default::default()
                    },
                );
            }
        }

        Ok(I18nResource {
            metadata: ResourceMetadata {
                source_format: FormatId::Markdown,
                locale,
                format_ext: Some(FormatExtension::Markdown(MarkdownExt {
                    front_matter,
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
            nested_keys: true,
            inline_markup: true,
            alternatives: false,
            source_references: false,
            custom_properties: true,
        }
    }
}

/// Convert accumulated content lines into a single value string.
/// Leading/trailing blank lines are stripped, and paragraphs are joined with \n\n.
fn finalize_content(lines: &[String]) -> String {
    // Trim leading and trailing empty lines
    let mut start = 0;
    while start < lines.len() && lines[start].trim().is_empty() {
        start += 1;
    }
    let mut end = lines.len();
    while end > start && lines[end - 1].trim().is_empty() {
        end -= 1;
    }

    if start >= end {
        return String::new();
    }

    // Join the lines, preserving paragraph breaks (blank lines become \n\n)
    let trimmed = &lines[start..end];
    let mut result = String::new();
    let mut prev_blank = false;

    for (i, line) in trimmed.iter().enumerate() {
        if line.trim().is_empty() {
            prev_blank = true;
        } else {
            if i > 0 {
                if prev_blank {
                    result.push_str("\n\n");
                } else {
                    result.push('\n');
                }
            }
            result.push_str(line);
            prev_blank = false;
        }
    }

    result
}

// ---------------------------------------------------------------------------
// Writer implementation
// ---------------------------------------------------------------------------

impl FormatWriter for Writer {
    fn write(&self, resource: &I18nResource) -> Result<Vec<u8>, WriteError> {
        let mut output = String::new();

        // Write front matter if present
        if let Some(FormatExtension::Markdown(ext)) = &resource.metadata.format_ext {
            if let Some(fm) = &ext.front_matter {
                output.push_str("---\n");
                output.push_str(fm);
                output.push_str("\n---\n\n");
            }
        }

        // Track which heading prefixes have already been emitted so we can
        // insert intermediate headings for gaps in nesting depth.
        let mut emitted_prefixes: std::collections::HashSet<String> = std::collections::HashSet::new();

        // Write entries as headings with content
        for (_key, entry) in &resource.entries {
            let parts: Vec<&str> = entry.key.split('.').collect();

            // Emit any missing intermediate headings.
            // For key "a.b.c" (depth 3), ensure headings for "a" (depth 1) and "a.b" (depth 2)
            // exist. Only emit headings that haven't been emitted yet (as real entries or intermediates).
            for i in 1..parts.len() {
                let prefix: String = parts[..i].join(".");
                if !emitted_prefixes.contains(&prefix) {
                    let level = i.min(6);
                    let hashes: String = "#".repeat(level);
                    let heading_text = key_to_heading(parts[i - 1]);
                    output.push_str(&format!("{} {}\n\n", hashes, heading_text));
                    emitted_prefixes.insert(prefix);
                }
            }

            // Emit the heading for this entry itself
            let depth = parts.len();
            let heading_level = depth.min(6);
            let hashes: String = "#".repeat(heading_level);
            let heading_text = key_to_heading(parts.last().expect("key should have at least one part"));
            output.push_str(&format!("{} {}\n\n", hashes, heading_text));
            emitted_prefixes.insert(entry.key.clone());

            // Write the content
            let value_str = match &entry.value {
                EntryValue::Simple(s) => s.clone(),
                EntryValue::Plural(ps) => ps.other.clone(),
                EntryValue::Array(arr) => arr.join("\n"),
                EntryValue::Select(ss) => {
                    ss.cases.get("other").cloned().unwrap_or_default()
                }
                EntryValue::MultiVariablePlural(mvp) => mvp.pattern.clone(),
            };

            if !value_str.is_empty() {
                output.push_str(&value_str);
                output.push_str("\n\n");
            }
        }

        // Remove trailing extra newline (keep exactly one)
        while output.ends_with("\n\n") {
            output.pop();
        }
        if !output.ends_with('\n') {
            output.push('\n');
        }

        Ok(output.into_bytes())
    }

    fn capabilities(&self) -> FormatCapabilities {
        Parser.capabilities()
    }
}

/// Convert a kebab-case key segment back to a human-readable heading.
/// e.g. "getting-started" -> "Getting Started"
///      "how-do-i-reset-my-password" -> "How Do I Reset My Password"
fn key_to_heading(key: &str) -> String {
    key.split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    let upper: String = first.to_uppercase().collect();
                    upper + &chars.collect::<String>()
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
