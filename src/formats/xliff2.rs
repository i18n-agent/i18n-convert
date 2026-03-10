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
    Group,
    Unit,
    Notes,
    Note,
    Segment,
    Source,
    Target,
}

// Accumulator for a single <unit>
#[derive(Debug, Default)]
struct UnitBuilder {
    id: String,
    source: String,
    target: Option<String>,
    segment_state: Option<String>,
    notes: Vec<NoteBuilder>,
    properties: IndexMap<String, String>,
}

#[derive(Debug, Default)]
struct NoteBuilder {
    text: String,
    category: Option<String>,
    priority: Option<u8>,
}

// ---------------------------------------------------------------------------
// Mapping helpers
// ---------------------------------------------------------------------------

fn map_xliff2_state(state: &str) -> Option<TranslationState> {
    match state {
        "initial" => Some(TranslationState::New),
        "translated" => Some(TranslationState::Translated),
        "reviewed" => Some(TranslationState::Reviewed),
        "final" => Some(TranslationState::Final),
        "needs-review" => Some(TranslationState::NeedsReview),
        _ => None,
    }
}

fn state_to_xliff2(state: &TranslationState) -> &'static str {
    match state {
        TranslationState::New => "initial",
        TranslationState::Translated => "translated",
        TranslationState::Reviewed => "reviewed",
        TranslationState::Final => "final",
        TranslationState::NeedsReview => "needs-review",
        TranslationState::NeedsTranslation => "initial",
        TranslationState::NeedsAdaptation => "initial",
        TranslationState::NeedsL10n => "initial",
        TranslationState::NeedsReviewAdaptation => "needs-review",
        TranslationState::NeedsReviewL10n => "needs-review",
        TranslationState::Stale => "needs-review",
        TranslationState::Vanished => "initial",
        TranslationState::Obsolete => "initial",
    }
}

fn map_note_role(category: Option<&str>) -> CommentRole {
    match category {
        Some("description") => CommentRole::Developer,
        Some("translator") => CommentRole::Translator,
        Some("extracted") => CommentRole::Extracted,
        _ => CommentRole::General,
    }
}

fn role_to_category(role: &CommentRole) -> Option<&'static str> {
    match role {
        CommentRole::Developer => Some("description"),
        CommentRole::Translator => Some("translator"),
        CommentRole::Extracted => Some("extracted"),
        CommentRole::General => Some("general"),
    }
}

// ---------------------------------------------------------------------------
// Parser implementation
// ---------------------------------------------------------------------------

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        let is_xliff_ext = extension == ".xliff" || extension == ".xlf";

        if let Ok(s) = std::str::from_utf8(content) {
            if s.contains("version=\"2.0\"") && s.contains("<xliff") {
                return Confidence::Definite;
            }
            if s.contains("urn:oasis:names:tc:xliff:document:2.0") {
                return Confidence::Definite;
            }
        }

        if is_xliff_ext {
            // Could be XLIFF 1.2 or 2.0, but we can't tell without content — return Low
            // (XLIFF 1.2 parser returns High for the same extension, so it takes priority)
            return Confidence::Low;
        }

        Confidence::None
    }

    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError> {
        let mut reader = Reader::from_reader(content);
        reader.config_mut().trim_text(true);

        let mut metadata = ResourceMetadata {
            source_format: FormatId::Xliff2,
            ..Default::default()
        };

        let mut entries = IndexMap::new();
        let mut state = ParseState::Root;
        let mut state_stack: Vec<ParseState> = Vec::new();
        let mut buf = Vec::new();
        let mut text_buf = String::new();

        let mut current_unit: Option<UnitBuilder> = None;
        let mut current_note: Option<NoteBuilder> = None;

        // File-level data
        let mut file_original: Option<String> = None;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Eof) => break,
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let local_name = tag_name.split(':').next_back().unwrap_or(&tag_name);

                    match local_name {
                        "xliff" => {
                            metadata.source_locale = get_attr(e, b"srcLang");
                            metadata.locale = get_attr(e, b"trgLang");
                        }
                        "file" => {
                            state = ParseState::File;
                            file_original = get_attr(e, b"original");
                        }
                        "group" => {
                            state_stack.push(state.clone());
                            state = ParseState::Group;
                            // We don't push group IDs into key prefixes since XLIFF 2.0
                            // unit IDs are already fully qualified
                        }
                        "unit" => {
                            state_stack.push(state.clone());
                            state = ParseState::Unit;
                            let id = get_attr(e, b"id").unwrap_or_default();
                            current_unit = Some(UnitBuilder {
                                id,
                                ..Default::default()
                            });
                        }
                        "notes" => {
                            state_stack.push(state.clone());
                            state = ParseState::Notes;
                        }
                        "note" => {
                            state_stack.push(state.clone());
                            state = ParseState::Note;
                            current_note = Some(NoteBuilder {
                                category: get_attr(e, b"category"),
                                priority: get_attr(e, b"priority")
                                    .and_then(|v| v.parse::<u8>().ok()),
                                ..Default::default()
                            });
                            text_buf.clear();
                        }
                        "segment" => {
                            state_stack.push(state.clone());
                            state = ParseState::Segment;
                            if let Some(ref mut unit) = current_unit {
                                unit.segment_state = get_attr(e, b"state");
                            }
                        }
                        "source" => {
                            state_stack.push(state.clone());
                            state = ParseState::Source;
                            text_buf.clear();
                        }
                        "target" => {
                            state_stack.push(state.clone());
                            state = ParseState::Target;
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
                    let local_name = tag_name.split(':').next_back().unwrap_or(&tag_name);

                    match local_name {
                        "source" => {
                            if let Some(ref mut unit) = current_unit {
                                unit.source = text_buf.clone();
                            }
                            text_buf.clear();
                            state = state_stack.pop().unwrap_or(ParseState::Segment);
                        }
                        "target" => {
                            if let Some(ref mut unit) = current_unit {
                                unit.target = Some(text_buf.clone());
                            }
                            text_buf.clear();
                            state = state_stack.pop().unwrap_or(ParseState::Segment);
                        }
                        "note" => {
                            if let Some(mut note) = current_note.take() {
                                note.text = text_buf.clone();
                                if let Some(ref mut unit) = current_unit {
                                    unit.notes.push(note);
                                }
                            }
                            text_buf.clear();
                            state = state_stack.pop().unwrap_or(ParseState::Notes);
                        }
                        "notes" => {
                            state = state_stack.pop().unwrap_or(ParseState::Unit);
                        }
                        "segment" => {
                            state = state_stack.pop().unwrap_or(ParseState::Unit);
                        }
                        "unit" => {
                            if let Some(unit) = current_unit.take() {
                                let comments: Vec<Comment> = unit
                                    .notes
                                    .iter()
                                    .map(|n| Comment {
                                        text: n.text.clone(),
                                        role: map_note_role(n.category.as_deref()),
                                        priority: n.priority,
                                        annotates: None,
                                    })
                                    .collect();

                                let target_state =
                                    unit.segment_state.as_deref().and_then(map_xliff2_state);

                                let value = unit
                                    .target
                                    .clone()
                                    .map(EntryValue::Simple)
                                    .unwrap_or_else(|| EntryValue::Simple(String::new()));

                                let entry = I18nEntry {
                                    key: unit.id.clone(),
                                    value,
                                    comments,
                                    source: Some(unit.source),
                                    state: target_state,
                                    properties: unit.properties,
                                    ..Default::default()
                                };

                                entries.insert(unit.id, entry);
                            }
                            state = state_stack.pop().unwrap_or(ParseState::File);
                        }
                        "group" => {
                            state = state_stack.pop().unwrap_or(ParseState::File);
                        }
                        "file" => {
                            state = ParseState::Root;
                        }
                        "xliff" => {
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

        // Set file-level extension
        metadata.format_ext = Some(FormatExtension::Xliff2(Xliff2Ext {
            can_resegment: None,
            original_data: {
                let mut m = IndexMap::new();
                if let Some(orig) = file_original {
                    m.insert("original".to_string(), orig);
                }
                m
            },
        }));

        Ok(I18nResource { metadata, entries })
    }

    fn capabilities(&self) -> FormatCapabilities {
        FormatCapabilities {
            plurals: false,
            arrays: false,
            comments: true,
            context: true,
            source_string: true,
            translatable_flag: false,
            translation_state: true,
            max_width: true,
            device_variants: false,
            select_gender: false,
            nested_keys: false,
            inline_markup: true,
            alternatives: true,
            source_references: false,
            custom_properties: true,
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
        xliff_start.push_attribute(("xmlns", "urn:oasis:names:tc:xliff:document:2.0"));
        xliff_start.push_attribute(("version", "2.0"));
        if let Some(ref sl) = resource.metadata.source_locale {
            xliff_start.push_attribute(("srcLang", sl.as_str()));
        }
        if let Some(ref tl) = resource.metadata.locale {
            xliff_start.push_attribute(("trgLang", tl.as_str()));
        }
        writer
            .write_event(Event::Start(xliff_start))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        // <file>
        let mut file_start = BytesStart::new("file");
        file_start.push_attribute(("id", "f1"));

        // Extract original from extension
        let original = match &resource.metadata.format_ext {
            Some(FormatExtension::Xliff2(ext)) => ext.original_data.get("original").cloned(),
            _ => None,
        };

        if let Some(ref orig) = original {
            file_start.push_attribute(("original", orig.as_str()));
        }

        writer
            .write_event(Event::Start(file_start))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        for (_key, entry) in &resource.entries {
            self.write_unit(&mut writer, entry)?;
        }

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
    fn write_unit(
        &self,
        writer: &mut XmlWriter<Cursor<&mut Vec<u8>>>,
        entry: &I18nEntry,
    ) -> Result<(), WriteError> {
        let mut unit_start = BytesStart::new("unit");
        unit_start.push_attribute(("id", entry.key.as_str()));
        writer
            .write_event(Event::Start(unit_start))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        // <notes> with <note> elements
        if !entry.comments.is_empty() {
            writer
                .write_event(Event::Start(BytesStart::new("notes")))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;

            for comment in &entry.comments {
                let mut note_start = BytesStart::new("note");
                if let Some(category) = role_to_category(&comment.role) {
                    note_start.push_attribute(("category", category));
                }
                if let Some(priority) = comment.priority {
                    let p_str = priority.to_string();
                    note_start.push_attribute(("priority", p_str.as_str()));
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

            writer
                .write_event(Event::End(BytesEnd::new("notes")))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;
        }

        // <segment>
        let mut segment_start = BytesStart::new("segment");
        if let Some(ref state) = entry.state {
            segment_start.push_attribute(("state", state_to_xliff2(state)));
        }
        writer
            .write_event(Event::Start(segment_start))
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
            EntryValue::Plural(ps) => Some(ps.other.clone()),
            EntryValue::Array(arr) => Some(arr.join(", ")),
            EntryValue::Select(ss) => Some(ss.cases.get("other").cloned().unwrap_or_default()),
            EntryValue::MultiVariablePlural(mvp) => Some(mvp.pattern.clone()),
        };

        if let Some(ref text) = target_text {
            if !text.is_empty() {
                writer
                    .write_event(Event::Start(BytesStart::new("target")))
                    .map_err(|e| WriteError::Serialization(e.to_string()))?;
                writer
                    .write_event(Event::Text(BytesText::new(text)))
                    .map_err(|e| WriteError::Serialization(e.to_string()))?;
                writer
                    .write_event(Event::End(BytesEnd::new("target")))
                    .map_err(|e| WriteError::Serialization(e.to_string()))?;
            }
        }

        // </segment>
        writer
            .write_event(Event::End(BytesEnd::new("segment")))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        // </unit>
        writer
            .write_event(Event::End(BytesEnd::new("unit")))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        Ok(())
    }
}
