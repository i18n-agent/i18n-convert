use crate::ir::*;
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
    Body,
    TransUnit,
    Source,
    Target,
    Note,
    ContextGroup,
    Context,
    AltTrans,
}

// Accumulator for a single <trans-unit>
#[derive(Debug, Default)]
struct TransUnitBuilder {
    id: String,
    source: String,
    target: Option<String>,
    target_state: Option<String>,
    target_state_qualifier: Option<String>,
    approved: Option<bool>,
    translatable: Option<bool>,
    max_width: Option<u32>,
    size_unit: Option<String>,
    restype: Option<String>,
    resname: Option<String>,
    notes: Vec<NoteBuilder>,
    contexts: Vec<ContextEntry>,
    alternatives: Vec<AltTransBuilder>,
    // Current context-group purpose
    current_context_group_purpose: Option<String>,
}

#[derive(Debug, Default)]
struct NoteBuilder {
    text: String,
    from: Option<String>,
    priority: Option<u8>,
    annotates: Option<String>,
}

#[derive(Debug, Default)]
struct AltTransBuilder {
    match_quality: Option<f32>,
    origin: Option<String>,
    source: Option<String>,
    target: Option<String>,
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
        // States that don't have a direct XLIFF 1.2 mapping
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

fn map_annotates(annotates: Option<&str>) -> Option<AnnotationTarget> {
    match annotates {
        Some("source") => Some(AnnotationTarget::Source),
        Some("target") => Some(AnnotationTarget::Target),
        Some("general") => Some(AnnotationTarget::General),
        _ => None,
    }
}

fn annotates_to_str(target: &AnnotationTarget) -> &'static str {
    match target {
        AnnotationTarget::Source => "source",
        AnnotationTarget::Target => "target",
        AnnotationTarget::General => "general",
    }
}

fn map_context_type(ct: &str) -> ContextType {
    match ct {
        "sourcefile" => ContextType::SourceFile,
        "linenumber" => ContextType::LineNumber,
        "element" => ContextType::Element,
        other => ContextType::Custom(other.to_string()),
    }
}

fn context_type_to_str(ct: &ContextType) -> String {
    match ct {
        ContextType::SourceFile => "sourcefile".to_string(),
        ContextType::LineNumber => "linenumber".to_string(),
        ContextType::Element => "element".to_string(),
        ContextType::Disambiguation => "x-disambiguation".to_string(),
        ContextType::Description => "x-description".to_string(),
        ContextType::Custom(s) => s.clone(),
    }
}

// Helper to get an attribute value from a BytesStart event
fn get_attr(e: &BytesStart, name: &[u8]) -> Option<String> {
    e.attributes()
        .filter_map(|a| a.ok())
        .find(|a| a.key.as_ref() == name)
        .and_then(|a| {
            String::from_utf8(a.value.to_vec()).ok()
        })
}

// ---------------------------------------------------------------------------
// Parser implementation
// ---------------------------------------------------------------------------

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        if extension == ".xliff" || extension == ".xlf" {
            return Confidence::High;
        }
        if extension == ".xml" {
            if let Ok(s) = std::str::from_utf8(content) {
                if s.contains("<xliff") {
                    return Confidence::Definite;
                }
            }
        }
        Confidence::None
    }

    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError> {
        let mut reader = Reader::from_reader(content);
        reader.config_mut().trim_text(true);

        let mut metadata = ResourceMetadata {
            source_format: FormatId::Xliff1,
            ..Default::default()
        };

        let mut entries = IndexMap::new();
        let mut state = ParseState::Root;
        let mut buf = Vec::new();
        let mut text_buf = String::new();

        let mut current_tu: Option<TransUnitBuilder> = None;
        let mut current_note: Option<NoteBuilder> = None;
        let mut current_context_type: Option<String> = None;
        // Track if we're in alt-trans source or target
        let mut in_alt_source = false;
        let mut in_alt_target = false;

        // File-level extension data
        let mut file_datatype: Option<String> = None;
        let mut file_original: Option<String> = None;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Eof) => break,
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    // Strip namespace prefix
                    let local_name = tag_name.split(':').last().unwrap_or(&tag_name);

                    match local_name {
                        "xliff" => {
                            // Root element — nothing to extract beyond version
                        }
                        "file" => {
                            state = ParseState::File;
                            metadata.source_locale = get_attr(e, b"source-language");
                            metadata.locale = get_attr(e, b"target-language");
                            file_datatype = get_attr(e, b"datatype");
                            file_original = get_attr(e, b"original");
                        }
                        "body" => {
                            state = ParseState::Body;
                        }
                        "trans-unit" => {
                            state = ParseState::TransUnit;
                            let id = get_attr(e, b"id").unwrap_or_default();
                            let approved = get_attr(e, b"approved").map(|v| v == "yes");
                            let translatable = get_attr(e, b"translate").map(|v| v == "yes");
                            let max_width = get_attr(e, b"maxwidth")
                                .and_then(|v| v.parse::<u32>().ok());
                            let size_unit = get_attr(e, b"size-unit");
                            let restype = get_attr(e, b"restype");
                            let resname = get_attr(e, b"resname");

                            current_tu = Some(TransUnitBuilder {
                                id,
                                approved,
                                translatable,
                                max_width,
                                size_unit,
                                restype,
                                resname,
                                ..Default::default()
                            });
                        }
                        "source" => {
                            match state {
                                ParseState::AltTrans => {
                                    in_alt_source = true;
                                }
                                _ => {
                                    state = ParseState::Source;
                                }
                            }
                            text_buf.clear();
                        }
                        "target" => {
                            match state {
                                ParseState::AltTrans => {
                                    in_alt_target = true;
                                }
                                _ => {
                                    state = ParseState::Target;
                                    if let Some(ref mut tu) = current_tu {
                                        tu.target_state = get_attr(e, b"state");
                                        tu.target_state_qualifier = get_attr(e, b"state-qualifier");
                                    }
                                }
                            }
                            text_buf.clear();
                        }
                        "note" => {
                            let prev_state = state.clone();
                            state = ParseState::Note;
                            current_note = Some(NoteBuilder {
                                from: get_attr(e, b"from"),
                                priority: get_attr(e, b"priority")
                                    .and_then(|v| v.parse::<u8>().ok()),
                                annotates: get_attr(e, b"annotates"),
                                ..Default::default()
                            });
                            text_buf.clear();
                            // We'll restore the state on End
                            let _ = prev_state;
                        }
                        "context-group" => {
                            state = ParseState::ContextGroup;
                            if let Some(ref mut tu) = current_tu {
                                tu.current_context_group_purpose = get_attr(e, b"purpose");
                            }
                        }
                        "context" => {
                            state = ParseState::Context;
                            current_context_type = get_attr(e, b"context-type");
                            text_buf.clear();
                        }
                        "alt-trans" => {
                            state = ParseState::AltTrans;
                            let match_quality = get_attr(e, b"match-quality")
                                .and_then(|v| v.parse::<f32>().ok());
                            let origin = get_attr(e, b"origin");
                            if let Some(ref mut tu) = current_tu {
                                tu.alternatives.push(AltTransBuilder {
                                    match_quality,
                                    origin,
                                    ..Default::default()
                                });
                            }
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
                        "source" => {
                            if in_alt_source {
                                if let Some(ref mut tu) = current_tu {
                                    if let Some(alt) = tu.alternatives.last_mut() {
                                        alt.source = Some(text_buf.clone());
                                    }
                                }
                                in_alt_source = false;
                            } else {
                                if let Some(ref mut tu) = current_tu {
                                    tu.source = text_buf.clone();
                                }
                                state = ParseState::TransUnit;
                            }
                            text_buf.clear();
                        }
                        "target" => {
                            if in_alt_target {
                                if let Some(ref mut tu) = current_tu {
                                    if let Some(alt) = tu.alternatives.last_mut() {
                                        alt.target = Some(text_buf.clone());
                                    }
                                }
                                in_alt_target = false;
                            } else {
                                if let Some(ref mut tu) = current_tu {
                                    tu.target = Some(text_buf.clone());
                                }
                                state = ParseState::TransUnit;
                            }
                            text_buf.clear();
                        }
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
                        "context" => {
                            if let Some(ref mut tu) = current_tu {
                                let ct = current_context_type.take().unwrap_or_default();
                                tu.contexts.push(ContextEntry {
                                    context_type: map_context_type(&ct),
                                    value: text_buf.clone(),
                                    purpose: tu.current_context_group_purpose.clone(),
                                });
                            }
                            state = ParseState::ContextGroup;
                            text_buf.clear();
                        }
                        "context-group" => {
                            if let Some(ref mut tu) = current_tu {
                                tu.current_context_group_purpose = None;
                            }
                            state = ParseState::TransUnit;
                        }
                        "alt-trans" => {
                            state = ParseState::TransUnit;
                        }
                        "trans-unit" => {
                            if let Some(tu) = current_tu.take() {
                                let comments: Vec<Comment> = tu.notes.iter().map(|n| {
                                    Comment {
                                        text: n.text.clone(),
                                        role: map_note_role(n.from.as_deref()),
                                        priority: n.priority,
                                        annotates: map_annotates(n.annotates.as_deref()),
                                    }
                                }).collect();

                                let alternatives: Vec<AlternativeTranslation> = tu.alternatives.iter().map(|a| {
                                    AlternativeTranslation {
                                        value: a.target.clone().unwrap_or_default(),
                                        source: a.source.clone(),
                                        match_quality: a.match_quality,
                                        origin: a.origin.clone(),
                                        alt_type: None,
                                    }
                                }).collect();

                                let target_state = tu.target_state.as_deref()
                                    .and_then(map_xliff_state);

                                let value = tu.target.clone()
                                    .map(|t| EntryValue::Simple(t))
                                    .unwrap_or_else(|| EntryValue::Simple(String::new()));

                                let entry = I18nEntry {
                                    key: tu.id.clone(),
                                    value,
                                    comments,
                                    contexts: tu.contexts,
                                    source: Some(tu.source),
                                    translatable: tu.translatable,
                                    state: target_state,
                                    state_qualifier: tu.target_state_qualifier,
                                    approved: tu.approved,
                                    max_width: tu.max_width,
                                    size_unit: tu.size_unit,
                                    alternatives,
                                    resource_type: tu.restype,
                                    resource_name: tu.resname,
                                    ..Default::default()
                                };

                                entries.insert(tu.id, entry);
                            }
                            state = ParseState::Body;
                        }
                        "body" => {
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

        // Set file-level extension
        metadata.format_ext = Some(FormatExtension::Xliff1(Xliff1Ext {
            datatype: file_datatype,
            original: file_original,
            inline_elements: Vec::new(),
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
            translatable_flag: true,
            translation_state: true,
            max_width: true,
            device_variants: false,
            select_gender: false,
            nested_keys: false,
            inline_markup: true,
            alternatives: true,
            source_references: true,
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
        writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        // <xliff>
        let mut xliff_start = BytesStart::new("xliff");
        xliff_start.push_attribute(("version", "1.2"));
        xliff_start.push_attribute(("xmlns", "urn:oasis:names:tc:xliff:document:1.2"));
        writer.write_event(Event::Start(xliff_start))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        // <file>
        let mut file_start = BytesStart::new("file");
        if let Some(ref sl) = resource.metadata.source_locale {
            file_start.push_attribute(("source-language", sl.as_str()));
        }
        if let Some(ref tl) = resource.metadata.locale {
            file_start.push_attribute(("target-language", tl.as_str()));
        }

        // Extract datatype and original from extension
        let (datatype, original) = match &resource.metadata.format_ext {
            Some(FormatExtension::Xliff1(ext)) => (
                ext.datatype.as_deref(),
                ext.original.as_deref(),
            ),
            _ => (None, None),
        };

        file_start.push_attribute(("datatype", datatype.unwrap_or("plaintext")));
        if let Some(orig) = original {
            file_start.push_attribute(("original", orig));
        }

        writer.write_event(Event::Start(file_start))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        // <body>
        writer.write_event(Event::Start(BytesStart::new("body")))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        for (_key, entry) in &resource.entries {
            self.write_trans_unit(&mut writer, entry)?;
        }

        // </body>
        writer.write_event(Event::End(BytesEnd::new("body")))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        // </file>
        writer.write_event(Event::End(BytesEnd::new("file")))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        // </xliff>
        writer.write_event(Event::End(BytesEnd::new("xliff")))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        // Flush and get the bytes
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

        if let Some(ref rn) = entry.resource_name {
            tu_start.push_attribute(("resname", rn.as_str()));
        }
        if let Some(ref rt) = entry.resource_type {
            tu_start.push_attribute(("restype", rt.as_str()));
        }
        if let Some(mw) = entry.max_width {
            let mw_str = mw.to_string();
            tu_start.push_attribute(("maxwidth", mw_str.as_str()));
        }
        if let Some(ref su) = entry.size_unit {
            tu_start.push_attribute(("size-unit", su.as_str()));
        }
        if let Some(translatable) = entry.translatable {
            tu_start.push_attribute(("translate", if translatable { "yes" } else { "no" }));
        }
        if let Some(approved) = entry.approved {
            tu_start.push_attribute(("approved", if approved { "yes" } else { "no" }));
        }

        writer.write_event(Event::Start(tu_start))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        // <source>
        if let Some(ref source) = entry.source {
            writer.write_event(Event::Start(BytesStart::new("source")))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;
            writer.write_event(Event::Text(BytesText::new(source)))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;
            writer.write_event(Event::End(BytesEnd::new("source")))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;
        }

        // <target>
        let target_text = match &entry.value {
            EntryValue::Simple(s) => Some(s.clone()),
            _ => None,
        };

        if let Some(ref text) = target_text {
            // Only write target if there's actual content or a state
            if !text.is_empty() || entry.state.is_some() {
                let mut target_start = BytesStart::new("target");
                if let Some(ref state) = entry.state {
                    target_start.push_attribute(("state", state_to_xliff(state)));
                }
                if let Some(ref sq) = entry.state_qualifier {
                    target_start.push_attribute(("state-qualifier", sq.as_str()));
                }
                writer.write_event(Event::Start(target_start))
                    .map_err(|e| WriteError::Serialization(e.to_string()))?;
                writer.write_event(Event::Text(BytesText::new(text)))
                    .map_err(|e| WriteError::Serialization(e.to_string()))?;
                writer.write_event(Event::End(BytesEnd::new("target")))
                    .map_err(|e| WriteError::Serialization(e.to_string()))?;
            }
        }

        // <note> elements
        for comment in &entry.comments {
            let mut note_start = BytesStart::new("note");
            if let Some(from) = role_to_from(&comment.role) {
                note_start.push_attribute(("from", from));
            }
            if let Some(priority) = comment.priority {
                let p_str = priority.to_string();
                note_start.push_attribute(("priority", p_str.as_str()));
            }
            if let Some(ref annotates) = comment.annotates {
                note_start.push_attribute(("annotates", annotates_to_str(annotates)));
            }
            writer.write_event(Event::Start(note_start))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;
            writer.write_event(Event::Text(BytesText::new(&comment.text)))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;
            writer.write_event(Event::End(BytesEnd::new("note")))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;
        }

        // <context-group> elements
        // Group contexts by purpose
        if !entry.contexts.is_empty() {
            // Group by purpose
            let mut groups: IndexMap<Option<String>, Vec<&ContextEntry>> = IndexMap::new();
            for ctx in &entry.contexts {
                groups.entry(ctx.purpose.clone()).or_default().push(ctx);
            }

            for (purpose, ctxs) in &groups {
                let mut cg_start = BytesStart::new("context-group");
                if let Some(ref p) = purpose {
                    cg_start.push_attribute(("purpose", p.as_str()));
                }
                writer.write_event(Event::Start(cg_start))
                    .map_err(|e| WriteError::Serialization(e.to_string()))?;

                for ctx in ctxs {
                    let ct_str = context_type_to_str(&ctx.context_type);
                    let mut ctx_start = BytesStart::new("context");
                    ctx_start.push_attribute(("context-type", ct_str.as_str()));
                    writer.write_event(Event::Start(ctx_start))
                        .map_err(|e| WriteError::Serialization(e.to_string()))?;
                    writer.write_event(Event::Text(BytesText::new(&ctx.value)))
                        .map_err(|e| WriteError::Serialization(e.to_string()))?;
                    writer.write_event(Event::End(BytesEnd::new("context")))
                        .map_err(|e| WriteError::Serialization(e.to_string()))?;
                }

                writer.write_event(Event::End(BytesEnd::new("context-group")))
                    .map_err(|e| WriteError::Serialization(e.to_string()))?;
            }
        }

        // <alt-trans> elements
        for alt in &entry.alternatives {
            let mut alt_start = BytesStart::new("alt-trans");
            if let Some(mq) = alt.match_quality {
                // Write as integer if it's a whole number, else as float
                let mq_str = if mq == mq.floor() {
                    format!("{}", mq as i32)
                } else {
                    format!("{}", mq)
                };
                alt_start.push_attribute(("match-quality", mq_str.as_str()));
            }
            if let Some(ref origin) = alt.origin {
                alt_start.push_attribute(("origin", origin.as_str()));
            }
            writer.write_event(Event::Start(alt_start))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;

            if let Some(ref source) = alt.source {
                writer.write_event(Event::Start(BytesStart::new("source")))
                    .map_err(|e| WriteError::Serialization(e.to_string()))?;
                writer.write_event(Event::Text(BytesText::new(source)))
                    .map_err(|e| WriteError::Serialization(e.to_string()))?;
                writer.write_event(Event::End(BytesEnd::new("source")))
                    .map_err(|e| WriteError::Serialization(e.to_string()))?;
            }

            writer.write_event(Event::Start(BytesStart::new("target")))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;
            writer.write_event(Event::Text(BytesText::new(&alt.value)))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;
            writer.write_event(Event::End(BytesEnd::new("target")))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;

            writer.write_event(Event::End(BytesEnd::new("alt-trans")))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;
        }

        // </trans-unit>
        writer.write_event(Event::End(BytesEnd::new("trans-unit")))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        Ok(())
    }
}
