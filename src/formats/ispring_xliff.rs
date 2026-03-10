use super::*;
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

/// Accumulator for a single `<trans-unit>`.
#[derive(Debug, Default)]
struct TransUnitBuilder {
    id: String,
    source: String,
    target: Option<String>,
    target_state: Option<String>,
    notes: Vec<NoteBuilder>,
}

#[derive(Debug, Default)]
struct NoteBuilder {
    text: String,
    from: Option<String>,
}

// ---------------------------------------------------------------------------
// Mapping helpers
// ---------------------------------------------------------------------------

fn map_xliff_state(state: &str) -> Option<TranslationState> {
    match state {
        "new" => Some(TranslationState::New),
        "translated" => Some(TranslationState::Translated),
        "needs-translation" => Some(TranslationState::NeedsTranslation),
        "needs-adaptation" => Some(TranslationState::NeedsAdaptation),
        "needs-l10n" => Some(TranslationState::NeedsL10n),
        "needs-review-translation" => Some(TranslationState::NeedsReview),
        "needs-review-adaptation" => Some(TranslationState::NeedsReviewAdaptation),
        "needs-review-l10n" => Some(TranslationState::NeedsReviewL10n),
        "signed-off" => Some(TranslationState::Reviewed),
        "final" => Some(TranslationState::Final),
        _ => None,
    }
}

fn state_to_xliff(state: &TranslationState) -> &'static str {
    match state {
        TranslationState::New => "new",
        TranslationState::Translated => "translated",
        TranslationState::NeedsTranslation => "needs-translation",
        TranslationState::NeedsAdaptation => "needs-adaptation",
        TranslationState::NeedsL10n => "needs-l10n",
        TranslationState::NeedsReview => "needs-review-translation",
        TranslationState::NeedsReviewAdaptation => "needs-review-adaptation",
        TranslationState::NeedsReviewL10n => "needs-review-l10n",
        TranslationState::Reviewed => "signed-off",
        TranslationState::Final => "final",
        TranslationState::Stale => "needs-review-translation",
        TranslationState::Vanished => "needs-translation",
        TranslationState::Obsolete => "needs-translation",
    }
}

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

/// Check whether the given content contains iSpring markers.
/// Looks for "ispring" (case-insensitive) in the document text.
fn content_has_ispring_markers(content: &[u8]) -> bool {
    if let Ok(s) = std::str::from_utf8(content) {
        let lower = s.to_ascii_lowercase();
        lower.contains("ispring")
    } else {
        false
    }
}

// ---------------------------------------------------------------------------
// Parser implementation
// ---------------------------------------------------------------------------

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        if extension != ".xliff" && extension != ".xlf" {
            return Confidence::None;
        }
        if content_has_ispring_markers(content) {
            Confidence::Definite
        } else {
            Confidence::None
        }
    }

    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError> {
        let mut reader = Reader::from_reader(content);
        reader.config_mut().trim_text(true);

        let mut metadata = ResourceMetadata {
            source_format: FormatId::IspringXliff,
            ..Default::default()
        };

        let mut entries = IndexMap::new();
        let mut state = ParseState::Root;
        let mut buf = Vec::new();
        let mut text_buf = String::new();

        let mut current_tu: Option<TransUnitBuilder> = None;
        let mut current_note: Option<NoteBuilder> = None;

        // Extension data
        let mut xliff_version: Option<String> = None;
        let mut source_language: Option<String> = None;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Eof) => break,
                Ok(Event::Start(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let local_name = tag_name.split(':').next_back().unwrap_or(&tag_name);

                    match local_name {
                        "xliff" => {
                            xliff_version = get_attr(e, b"version");
                        }
                        "file" => {
                            state = ParseState::File;
                            let src_lang = get_attr(e, b"source-language");
                            metadata.source_locale = src_lang.clone();
                            source_language = src_lang;
                            metadata.locale = get_attr(e, b"target-language");
                        }
                        "header" => {
                            state = ParseState::Header;
                        }
                        "tool" => {
                            if let Some(tool_name) = get_attr(e, b"tool-name") {
                                metadata.tool_name = Some(tool_name);
                            }
                            if let Some(tool_version) = get_attr(e, b"tool-version") {
                                metadata.tool_version = Some(tool_version);
                            }
                        }
                        "body" => {
                            state = ParseState::Body;
                        }
                        "trans-unit" => {
                            state = ParseState::TransUnit;
                            let id = get_attr(e, b"id").unwrap_or_default();
                            current_tu = Some(TransUnitBuilder {
                                id,
                                ..Default::default()
                            });
                        }
                        "source" => {
                            if state == ParseState::TransUnit {
                                state = ParseState::Source;
                                text_buf.clear();
                            }
                        }
                        "target" => {
                            if state == ParseState::TransUnit {
                                state = ParseState::Target;
                                if let Some(ref mut tu) = current_tu {
                                    tu.target_state = get_attr(e, b"state");
                                }
                                text_buf.clear();
                            }
                        }
                        "note" => {
                            if state == ParseState::TransUnit {
                                state = ParseState::Note;
                                current_note = Some(NoteBuilder {
                                    from: get_attr(e, b"from"),
                                    ..Default::default()
                                });
                                text_buf.clear();
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::Empty(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let local_name = tag_name.split(':').next_back().unwrap_or(&tag_name);

                    if local_name == "tool" {
                        if let Some(tool_name) = get_attr(e, b"tool-name") {
                            metadata.tool_name = Some(tool_name);
                        }
                        if let Some(tool_version) = get_attr(e, b"tool-version") {
                            metadata.tool_version = Some(tool_version);
                        }
                    }
                }
                Ok(Event::Text(ref e)) => {
                    let t = e.unescape().unwrap_or_default().to_string();
                    text_buf.push_str(&t);
                }
                Ok(Event::End(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let local_name = tag_name.split(':').next_back().unwrap_or(&tag_name);

                    match local_name {
                        "source" => {
                            if state == ParseState::Source {
                                if let Some(ref mut tu) = current_tu {
                                    tu.source = text_buf.clone();
                                }
                                state = ParseState::TransUnit;
                                text_buf.clear();
                            }
                        }
                        "target" => {
                            if state == ParseState::Target {
                                if let Some(ref mut tu) = current_tu {
                                    tu.target = Some(text_buf.clone());
                                }
                                state = ParseState::TransUnit;
                                text_buf.clear();
                            }
                        }
                        "note" => {
                            if state == ParseState::Note {
                                if let Some(mut note) = current_note.take() {
                                    note.text = text_buf.clone();
                                    if let Some(ref mut tu) = current_tu {
                                        tu.notes.push(note);
                                    }
                                }
                                state = ParseState::TransUnit;
                                text_buf.clear();
                            }
                        }
                        "trans-unit" => {
                            if let Some(tu) = current_tu.take() {
                                let comments: Vec<Comment> = tu
                                    .notes
                                    .iter()
                                    .map(|n| Comment {
                                        text: n.text.clone(),
                                        role: map_note_role(n.from.as_deref()),
                                        priority: None,
                                        annotates: None,
                                    })
                                    .collect();

                                let target_state =
                                    tu.target_state.as_deref().and_then(map_xliff_state);

                                // If target exists, use it; otherwise fall back to source
                                let value = match tu.target {
                                    Some(ref t) => EntryValue::Simple(t.clone()),
                                    None => EntryValue::Simple(tu.source.clone()),
                                };

                                let entry = I18nEntry {
                                    key: tu.id.clone(),
                                    value,
                                    comments,
                                    source: Some(tu.source),
                                    state: target_state,
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

        // Store format extension
        metadata.format_ext = Some(FormatExtension::IspringXliff(IspringXliffExt {
            xliff_version,
            source_language,
        }));

        Ok(I18nResource { metadata, entries })
    }

    fn capabilities(&self) -> FormatCapabilities {
        FormatCapabilities {
            source_string: true,
            translation_state: true,
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
        file_start.push_attribute(("original", "ispring_course"));
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

        // <tool tool-id="ispring" tool-name="iSpring Suite"/>
        let mut tool_elem = BytesStart::new("tool");
        tool_elem.push_attribute(("tool-id", "ispring"));
        tool_elem.push_attribute((
            "tool-name",
            resource
                .metadata
                .tool_name
                .as_deref()
                .unwrap_or("iSpring Suite"),
        ));
        if let Some(ref tv) = resource.metadata.tool_version {
            tool_elem.push_attribute(("tool-version", tv.as_str()));
        }
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
        writer
            .write_event(Event::Start(tu_start))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        // <source>
        if let Some(ref source) = entry.source {
            writer
                .write_event(Event::Start(BytesStart::new("source")))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;
            writer
                .write_event(Event::Text(BytesText::new(source)))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;
            writer
                .write_event(Event::End(BytesEnd::new("source")))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;
        }

        // <target>
        let target_text = match &entry.value {
            EntryValue::Simple(s) => Some(s.clone()),
            _ => None,
        };

        if let Some(ref text) = target_text {
            // Only write target if there's content or a state
            if !text.is_empty() || entry.state.is_some() {
                let mut target_start = BytesStart::new("target");
                if let Some(ref st) = entry.state {
                    target_start.push_attribute(("state", state_to_xliff(st)));
                }
                writer
                    .write_event(Event::Start(target_start))
                    .map_err(|e| WriteError::Serialization(e.to_string()))?;
                writer
                    .write_event(Event::Text(BytesText::new(text)))
                    .map_err(|e| WriteError::Serialization(e.to_string()))?;
                writer
                    .write_event(Event::End(BytesEnd::new("target")))
                    .map_err(|e| WriteError::Serialization(e.to_string()))?;
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
}
