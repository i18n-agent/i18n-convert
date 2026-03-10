use crate::ir::*;
use super::*;
use indexmap::IndexMap;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::reader::Reader;
use quick_xml::writer::Writer as XmlWriter;
use std::io::Cursor;

pub struct Parser;
pub struct Writer;

// ---------------------------------------------------------------------------
// State machine for SAX-style parsing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum ParseState {
    Root,
    File,
    Header,
    Body,
    TransUnit,
    Source,
    Target,
    Note,
}

// Accumulator for a single <trans-unit>
#[derive(Debug, Default)]
struct TransUnitBuilder {
    id: String,
    source: String,
    target: Option<String>,
    notes: Vec<NoteBuilder>,
    css_style: Option<String>,
}

#[derive(Debug, Default)]
struct NoteBuilder {
    text: String,
    from: Option<String>,
}

// ---------------------------------------------------------------------------
// Detection helpers
// ---------------------------------------------------------------------------

/// Check if any content marker indicates Adobe Captivate.
fn has_captivate_markers(content: &str) -> bool {
    let lower = content.to_lowercase();

    // Check for "captivate" in file original, tool-id, or tool-name attributes
    if lower.contains("captivate") {
        return true;
    }

    // Check for slide_N_item_M ID patterns in trans-unit IDs
    let re = regex::Regex::new(r#"id\s*=\s*"slide_\d+_item_\d+"#).unwrap();
    re.is_match(content)
}

// ---------------------------------------------------------------------------
// Mapping helpers
// ---------------------------------------------------------------------------

fn map_note_role(from: Option<&str>) -> CommentRole {
    match from {
        Some("developer") => CommentRole::Developer,
        Some("translator") => CommentRole::Translator,
        Some("extracted") => CommentRole::Extracted,
        _ => CommentRole::General,
    }
}

fn role_to_from(role: &CommentRole) -> Option<&'static str> {
    match role {
        CommentRole::Developer => Some("developer"),
        CommentRole::Translator => Some("translator"),
        CommentRole::Extracted => Some("extracted"),
        CommentRole::General => None,
    }
}

/// Parse slide_id and item_id from a trans-unit id like "slide_1_item_2"
fn parse_slide_item_id(id: &str) -> (Option<String>, Option<String>) {
    let re = regex::Regex::new(r"^slide_(\d+)_item_(\d+)$").unwrap();
    if let Some(caps) = re.captures(id) {
        (
            Some(caps[1].to_string()),
            Some(caps[2].to_string()),
        )
    } else {
        (None, None)
    }
}

// Helper to get an attribute value from a BytesStart event
fn get_attr(e: &BytesStart, name: &[u8]) -> Option<String> {
    e.attributes()
        .filter_map(|a| a.ok())
        .find(|a| a.key.as_ref() == name)
        .and_then(|a| String::from_utf8(a.value.to_vec()).ok())
}

// ---------------------------------------------------------------------------
// Inline markup handling
// ---------------------------------------------------------------------------

/// Collect text content from within a <source> or <target> element,
/// preserving inline <g> elements as raw markup in the string.
fn collect_inline_content(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    end_tag: &str,
) -> Result<String, ParseError> {
    let mut result = String::new();
    let mut depth = 0u32;

    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(ref e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let local = tag.split(':').last().unwrap_or(&tag);
                if local == end_tag && depth == 0 {
                    // This shouldn't happen since we already consumed the start
                    break;
                }
                // Reconstruct the opening tag with attributes
                result.push('<');
                result.push_str(local);
                for attr in e.attributes().filter_map(|a| a.ok()) {
                    let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                    let val = String::from_utf8_lossy(&attr.value).to_string();
                    result.push(' ');
                    result.push_str(&key);
                    result.push_str("=\"");
                    result.push_str(&val);
                    result.push('"');
                }
                result.push('>');
                depth += 1;
            }
            Ok(Event::End(ref e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let local = tag.split(':').last().unwrap_or(&tag);
                if local == end_tag && depth == 0 {
                    break;
                }
                result.push_str("</");
                result.push_str(local);
                result.push('>');
                depth = depth.saturating_sub(1);
            }
            Ok(Event::Empty(ref e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let local = tag.split(':').last().unwrap_or(&tag);
                result.push('<');
                result.push_str(local);
                for attr in e.attributes().filter_map(|a| a.ok()) {
                    let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                    let val = String::from_utf8_lossy(&attr.value).to_string();
                    result.push(' ');
                    result.push_str(&key);
                    result.push_str("=\"");
                    result.push_str(&val);
                    result.push('"');
                }
                result.push_str("/>");
            }
            Ok(Event::Text(ref e)) => {
                let t = e.unescape().unwrap_or_default().to_string();
                result.push_str(&t);
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(ParseError::Xml(format!("XML parse error: {e}"))),
            _ => {}
        }
        buf.clear();
    }
    Ok(result)
}

// ---------------------------------------------------------------------------
// Parser implementation
// ---------------------------------------------------------------------------

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        if extension != ".xml" {
            return Confidence::None;
        }
        if let Ok(s) = std::str::from_utf8(content) {
            if has_captivate_markers(s) {
                return Confidence::Definite;
            }
        }
        Confidence::None
    }

    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError> {
        let mut reader = Reader::from_reader(content);
        reader.config_mut().trim_text(true);

        let mut metadata = ResourceMetadata {
            source_format: FormatId::CaptivateXml,
            ..Default::default()
        };

        let mut entries = IndexMap::new();
        let mut state = ParseState::Root;
        let mut buf = Vec::new();
        let mut text_buf = String::new();

        let mut current_tu: Option<TransUnitBuilder> = None;
        let mut current_note: Option<NoteBuilder> = None;

        // File-level attributes
        let mut file_original: Option<String> = None;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Eof) => break,
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let local_name = tag_name.split(':').last().unwrap_or(&tag_name);

                    match local_name {
                        "xliff" => {
                            // Root element
                        }
                        "file" => {
                            state = ParseState::File;
                            metadata.source_locale = get_attr(e, b"source-language");
                            metadata.locale = get_attr(e, b"target-language");
                            file_original = get_attr(e, b"original");
                        }
                        "header" => {
                            state = ParseState::Header;
                        }
                        "tool" => {
                            // Extract tool info from header
                            if let Some(tool_name) = get_attr(e, b"tool-name") {
                                metadata.tool_name = Some(tool_name);
                            }
                        }
                        "body" => {
                            state = ParseState::Body;
                        }
                        "trans-unit" => {
                            state = ParseState::TransUnit;
                            let id = get_attr(e, b"id").unwrap_or_default();
                            let css_style = get_attr(e, b"css-style");

                            current_tu = Some(TransUnitBuilder {
                                id,
                                css_style,
                                ..Default::default()
                            });
                        }
                        "source" if state == ParseState::TransUnit => {
                            // Use inline content collector to handle <g> elements
                            let inline_text = collect_inline_content(
                                &mut reader,
                                &mut buf,
                                "source",
                            )?;
                            if let Some(ref mut tu) = current_tu {
                                tu.source = inline_text;
                            }
                            // After collect_inline_content, the </source> has been consumed
                            state = ParseState::TransUnit;
                            buf.clear();
                            continue;
                        }
                        "target" if state == ParseState::TransUnit => {
                            let inline_text = collect_inline_content(
                                &mut reader,
                                &mut buf,
                                "target",
                            )?;
                            if let Some(ref mut tu) = current_tu {
                                tu.target = Some(inline_text);
                            }
                            state = ParseState::TransUnit;
                            buf.clear();
                            continue;
                        }
                        "note" if state == ParseState::TransUnit => {
                            state = ParseState::Note;
                            current_note = Some(NoteBuilder {
                                from: get_attr(e, b"from"),
                                ..Default::default()
                            });
                            text_buf.clear();
                        }
                        _ => {}
                    }
                }
                Ok(Event::Text(ref e)) => {
                    let t = e.unescape().unwrap_or_default().to_string();
                    text_buf.push_str(&t);
                }
                Ok(Event::End(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let local_name = tag_name.split(':').last().unwrap_or(&tag_name);

                    match local_name {
                        "note" => {
                            if let Some(mut note) = current_note.take() {
                                note.text = text_buf.clone();
                                if let Some(ref mut tu) = current_tu {
                                    tu.notes.push(note);
                                }
                            }
                            state = ParseState::TransUnit;
                            text_buf.clear();
                        }
                        "trans-unit" => {
                            if let Some(tu) = current_tu.take() {
                                let (slide_id, item_id) = parse_slide_item_id(&tu.id);

                                let comments: Vec<Comment> = tu.notes.iter().map(|n| {
                                    Comment {
                                        text: n.text.clone(),
                                        role: map_note_role(n.from.as_deref()),
                                        priority: None,
                                        annotates: None,
                                    }
                                }).collect();

                                // For mono-lingual: use target if present, otherwise use source
                                let value_text = tu.target.clone().unwrap_or_else(|| tu.source.clone());
                                let value = EntryValue::Simple(value_text);

                                let entry_ext = if slide_id.is_some() || item_id.is_some() || tu.css_style.is_some() {
                                    Some(FormatExtension::CaptivateXml(CaptivateXmlExt {
                                        slide_id,
                                        item_id,
                                        css_style: tu.css_style.clone(),
                                    }))
                                } else {
                                    None
                                };

                                let entry = I18nEntry {
                                    key: tu.id.clone(),
                                    value,
                                    comments,
                                    source: Some(tu.source),
                                    format_ext: entry_ext,
                                    ..Default::default()
                                };

                                entries.insert(tu.id, entry);
                            }
                            state = ParseState::Body;
                        }
                        "body" => {
                            state = ParseState::File;
                        }
                        "header" => {
                            state = ParseState::File;
                        }
                        "file" => {
                            state = ParseState::Root;
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    return Err(ParseError::Xml(format!("XML parse error: {e}")));
                }
                _ => {}
            }
            buf.clear();
        }

        // Set resource-level extension with file original
        metadata.format_ext = Some(FormatExtension::CaptivateXml(CaptivateXmlExt {
            slide_id: None,
            item_id: None,
            css_style: None,
        }));

        // Store file original in metadata properties for round-trip
        if let Some(ref orig) = file_original {
            metadata.properties.insert("original".to_string(), orig.clone());
        }

        Ok(I18nResource { metadata, entries })
    }

    fn capabilities(&self) -> FormatCapabilities {
        FormatCapabilities {
            source_string: true,
            inline_markup: true,
            comments: true,
            context: true,
            ..Default::default()
        }
    }
}

// ---------------------------------------------------------------------------
// Writer implementation
// ---------------------------------------------------------------------------

impl FormatWriter for Writer {
    fn write(&self, resource: &I18nResource) -> Result<Vec<u8>, WriteError> {
        let mut buf = Vec::new();
        let mut writer = XmlWriter::new_with_indent(Cursor::new(&mut buf), b' ', 2);

        // XML declaration
        writer
            .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        // <xliff>
        let mut xliff_start = BytesStart::new("xliff");
        xliff_start.push_attribute(("version", "1.2"));
        xliff_start.push_attribute(("xmlns", "urn:oasis:names:tc:xliff:document:1.2"));
        writer
            .write_event(Event::Start(xliff_start))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        // <file>
        let mut file_start = BytesStart::new("file");

        // Determine original attribute
        let original = resource
            .metadata
            .properties
            .get("original")
            .cloned()
            .unwrap_or_else(|| "captivate_project".to_string());

        file_start.push_attribute(("original", original.as_str()));

        if let Some(ref sl) = resource.metadata.source_locale {
            file_start.push_attribute(("source-language", sl.as_str()));
        }
        if let Some(ref tl) = resource.metadata.locale {
            file_start.push_attribute(("target-language", tl.as_str()));
        }
        file_start.push_attribute(("datatype", "plaintext"));

        writer
            .write_event(Event::Start(file_start))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        // <header>
        writer
            .write_event(Event::Start(BytesStart::new("header")))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        // <tool tool-id="captivate" tool-name="Adobe Captivate"/>
        let mut tool_elem = BytesStart::new("tool");
        tool_elem.push_attribute(("tool-id", "captivate"));
        let tool_name = resource
            .metadata
            .tool_name
            .as_deref()
            .unwrap_or("Adobe Captivate");
        tool_elem.push_attribute(("tool-name", tool_name));
        writer
            .write_event(Event::Empty(tool_elem))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        // </header>
        writer
            .write_event(Event::End(BytesEnd::new("header")))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        // <body>
        writer
            .write_event(Event::Start(BytesStart::new("body")))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        for (_key, entry) in &resource.entries {
            self.write_trans_unit(&mut writer, entry)?;
        }

        // </body>
        writer
            .write_event(Event::End(BytesEnd::new("body")))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        // </file>
        writer
            .write_event(Event::End(BytesEnd::new("file")))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        // </xliff>
        writer
            .write_event(Event::End(BytesEnd::new("xliff")))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        drop(writer);
        Ok(buf)
    }

    fn capabilities(&self) -> FormatCapabilities {
        Parser.capabilities()
    }
}

impl Writer {
    fn write_trans_unit(
        &self,
        writer: &mut XmlWriter<Cursor<&mut Vec<u8>>>,
        entry: &I18nEntry,
    ) -> Result<(), WriteError> {
        let mut tu_start = BytesStart::new("trans-unit");
        tu_start.push_attribute(("id", entry.key.as_str()));

        // Add css-style from format extension
        if let Some(FormatExtension::CaptivateXml(ref ext)) = entry.format_ext {
            if let Some(ref css) = ext.css_style {
                tu_start.push_attribute(("css-style", css.as_str()));
            }
        }

        writer
            .write_event(Event::Start(tu_start))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        // <source> - write raw content (may contain inline markup)
        if let Some(ref source) = entry.source {
            self.write_raw_element(writer, "source", source)?;
        }

        // <target> - write only if value differs from source
        let target_text = match &entry.value {
            EntryValue::Simple(s) => Some(s.clone()),
            _ => None,
        };

        if let Some(ref text) = target_text {
            let source_text = entry.source.as_deref().unwrap_or("");
            if text != source_text {
                self.write_raw_element(writer, "target", text)?;
            }
        }

        // <note> elements
        for comment in &entry.comments {
            let mut note_start = BytesStart::new("note");
            if let Some(from) = role_to_from(&comment.role) {
                note_start.push_attribute(("from", from));
            }
            writer
                .write_event(Event::Start(note_start))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;
            writer
                .write_event(Event::Text(BytesText::new(&comment.text)))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;
            writer
                .write_event(Event::End(BytesEnd::new("note")))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;
        }

        // </trans-unit>
        writer
            .write_event(Event::End(BytesEnd::new("trans-unit")))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        Ok(())
    }

    /// Write an element that may contain raw inline XML markup (like <g> elements).
    /// We parse the raw string back through quick_xml reader and re-emit events.
    fn write_raw_element(
        &self,
        writer: &mut XmlWriter<Cursor<&mut Vec<u8>>>,
        tag: &str,
        content: &str,
    ) -> Result<(), WriteError> {
        // If content contains XML-like elements, wrap and re-emit
        if content.contains('<') {
            // Wrap content in a temporary root to parse
            let wrapped = format!("<{tag}>{content}</{tag}>");
            let mut reader = Reader::from_str(&wrapped);
            reader.config_mut().trim_text(false);
            let mut buf = Vec::new();

            loop {
                match reader.read_event_into(&mut buf) {
                    Ok(Event::Eof) => break,
                    Ok(Event::Start(ref e)) => {
                        let start = e.to_owned();
                        writer
                            .write_event(Event::Start(start))
                            .map_err(|e| WriteError::Serialization(e.to_string()))?;
                    }
                    Ok(Event::End(ref e)) => {
                        let end = e.to_owned();
                        writer
                            .write_event(Event::End(end))
                            .map_err(|e| WriteError::Serialization(e.to_string()))?;
                    }
                    Ok(Event::Empty(ref e)) => {
                        let empty = e.to_owned();
                        writer
                            .write_event(Event::Empty(empty))
                            .map_err(|e| WriteError::Serialization(e.to_string()))?;
                    }
                    Ok(Event::Text(ref e)) => {
                        let text = e.to_owned();
                        writer
                            .write_event(Event::Text(text))
                            .map_err(|e| WriteError::Serialization(e.to_string()))?;
                    }
                    Err(e) => {
                        return Err(WriteError::Serialization(format!(
                            "Error re-parsing inline content: {e}"
                        )));
                    }
                    _ => {}
                }
                buf.clear();
            }
        } else {
            // Plain text, no inline markup
            writer
                .write_event(Event::Start(BytesStart::new(tag)))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;
            writer
                .write_event(Event::Text(BytesText::new(content)))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;
            writer
                .write_event(Event::End(BytesEnd::new(tag)))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;
        }

        Ok(())
    }
}
