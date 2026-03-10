use crate::ir::*;
use super::*;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::reader::Reader;
use quick_xml::writer::Writer as XmlWriter;
use std::io::Cursor;

pub struct Parser;
pub struct Writer;

/// Separator used to join context name and source text into a unique key.
/// Same as PO convention (\x04 = EOT).
const CONTEXT_SEPARATOR: &str = "\x04";

// ---------------------------------------------------------------------------
// State machine for SAX-style parsing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum ParseState {
    Root,
    Ts,
    Context,
    ContextName,
    Message,
    Source,
    OldSource,
    Translation,
    NumerusForm,
    Comment,
    TranslatorComment,
    OldComment,
    ExtraElement,
}

/// Accumulator for a single <message>
#[derive(Debug, Default)]
struct MessageBuilder {
    source: String,
    translation: Option<String>,
    translation_type: Option<String>,
    numerus: bool,
    numerus_forms: Vec<String>,
    comment: Option<String>,
    translator_comment: Option<String>,
    old_source: Option<String>,
    old_comment: Option<String>,
    locations: Vec<SourceRef>,
    extra_elements: IndexMap<String, String>,
}

// Helper to get an attribute value from a BytesStart event
fn get_attr(e: &BytesStart, name: &[u8]) -> Option<String> {
    e.attributes()
        .filter_map(|a| a.ok())
        .find(|a| a.key.as_ref() == name)
        .and_then(|a| String::from_utf8(a.value.to_vec()).ok())
}

/// Map the Qt Linguist translation type attribute to IR TranslationState
fn map_qt_state(type_attr: Option<&str>) -> Option<TranslationState> {
    match type_attr {
        None => Some(TranslationState::Translated),
        Some("unfinished") => Some(TranslationState::NeedsReview),
        Some("obsolete") => Some(TranslationState::Obsolete),
        Some("vanished") => Some(TranslationState::Vanished),
        _ => None,
    }
}

/// Map IR TranslationState back to the Qt Linguist type attribute value
fn state_to_qt_type(state: &TranslationState) -> Option<&'static str> {
    match state {
        TranslationState::NeedsReview
        | TranslationState::New
        | TranslationState::NeedsTranslation => Some("unfinished"),
        TranslationState::Obsolete => Some("obsolete"),
        TranslationState::Vanished => Some("vanished"),
        TranslationState::Translated
        | TranslationState::Reviewed
        | TranslationState::Final => None,
        _ => Some("unfinished"),
    }
}

/// Build plural forms from numerus_forms vec into a PluralSet.
/// Qt mapping: index 0 -> one, index 1 -> other (for most European languages).
/// For languages with more plural forms, map: 0->one, 1->few, 2->many, last->other.
fn build_plural_set(forms: &[String]) -> PluralSet {
    match forms.len() {
        0 => PluralSet {
            other: String::new(),
            ..Default::default()
        },
        1 => PluralSet {
            other: forms[0].clone(),
            ..Default::default()
        },
        2 => PluralSet {
            one: Some(forms[0].clone()),
            other: forms[1].clone(),
            ..Default::default()
        },
        3 => PluralSet {
            one: Some(forms[0].clone()),
            few: Some(forms[1].clone()),
            other: forms[2].clone(),
            ..Default::default()
        },
        _ => {
            // 4+ forms: one, two, few, many, ..., other
            let mut set = PluralSet {
                one: Some(forms[0].clone()),
                other: forms[forms.len() - 1].clone(),
                ..Default::default()
            };
            if forms.len() > 2 {
                set.two = Some(forms[1].clone());
            }
            if forms.len() > 3 {
                set.few = Some(forms[2].clone());
            }
            if forms.len() > 4 {
                set.many = Some(forms[3].clone());
            }
            set
        }
    }
}

/// Convert a PluralSet back to a Vec of numerus forms for Qt output
fn plural_set_to_forms(plural: &PluralSet) -> Vec<String> {
    let mut forms = Vec::new();
    if let Some(ref one) = plural.one {
        forms.push(one.clone());
    }
    if let Some(ref two) = plural.two {
        forms.push(two.clone());
    }
    if let Some(ref few) = plural.few {
        forms.push(few.clone());
    }
    if let Some(ref many) = plural.many {
        forms.push(many.clone());
    }
    forms.push(plural.other.clone());
    forms
}

// ---------------------------------------------------------------------------
// Parser implementation
// ---------------------------------------------------------------------------

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        if extension == ".ts" {
            if let Ok(s) = std::str::from_utf8(content) {
                if s.contains("<!DOCTYPE TS>") || s.contains("<TS version=") || s.contains("<TS>") {
                    return Confidence::Definite;
                }
            }
            // .ts extension alone is ambiguous (could be TypeScript)
            return Confidence::None;
        }
        // Check content for any extension (e.g. .xml)
        if let Ok(s) = std::str::from_utf8(content) {
            if s.contains("<!DOCTYPE TS>") || s.contains("<TS version=") {
                return Confidence::High;
            }
        }
        Confidence::None
    }

    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError> {
        let mut reader = Reader::from_reader(content);
        reader.config_mut().trim_text(false);

        let mut metadata = ResourceMetadata {
            source_format: FormatId::QtLinguist,
            ..Default::default()
        };

        let mut entries = IndexMap::new();
        let mut state = ParseState::Root;
        let mut buf = Vec::new();
        let mut text_buf = String::new();

        let mut current_context_name = String::new();
        let mut current_message: Option<MessageBuilder> = None;
        let mut current_extra_name: Option<String> = None;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Eof) => break,
                Ok(Event::Start(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                    match tag_name.as_str() {
                        "TS" => {
                            state = ParseState::Ts;
                            metadata.locale = get_attr(e, b"language");
                            metadata.source_locale = get_attr(e, b"sourcelanguage");
                            if let Some(ref version) = get_attr(e, b"version") {
                                metadata
                                    .properties
                                    .insert("ts_version".to_string(), version.clone());
                            }
                        }
                        "context" if state == ParseState::Ts => {
                            state = ParseState::Context;
                            current_context_name.clear();
                        }
                        "name" if state == ParseState::Context => {
                            state = ParseState::ContextName;
                            text_buf.clear();
                        }
                        "message" if state == ParseState::Context => {
                            state = ParseState::Message;
                            let numerus = get_attr(e, b"numerus")
                                .map(|v| v == "yes")
                                .unwrap_or(false);
                            current_message = Some(MessageBuilder {
                                numerus,
                                ..Default::default()
                            });
                        }
                        "source" if state == ParseState::Message => {
                            state = ParseState::Source;
                            text_buf.clear();
                        }
                        "oldsource" if state == ParseState::Message => {
                            state = ParseState::OldSource;
                            text_buf.clear();
                        }
                        "translation" if state == ParseState::Message => {
                            state = ParseState::Translation;
                            if let Some(ref mut msg) = current_message {
                                msg.translation_type = get_attr(e, b"type");
                            }
                            text_buf.clear();
                        }
                        "numerusform" if state == ParseState::Translation => {
                            state = ParseState::NumerusForm;
                            text_buf.clear();
                        }
                        "comment" if state == ParseState::Message => {
                            state = ParseState::Comment;
                            text_buf.clear();
                        }
                        "translatorcomment" if state == ParseState::Message => {
                            state = ParseState::TranslatorComment;
                            text_buf.clear();
                        }
                        "oldcomment" if state == ParseState::Message => {
                            state = ParseState::OldComment;
                            text_buf.clear();
                        }
                        "location" if state == ParseState::Message => {
                            // <location> can be a start tag (will get an End event)
                            if let Some(ref mut msg) = current_message {
                                let file = get_attr(e, b"filename").unwrap_or_default();
                                let line = get_attr(e, b"line")
                                    .and_then(|v| v.parse::<u32>().ok());
                                msg.locations.push(SourceRef { file, line });
                            }
                        }
                        other if state == ParseState::Message
                            && other.starts_with("extra-") =>
                        {
                            state = ParseState::ExtraElement;
                            current_extra_name = Some(other.to_string());
                            text_buf.clear();
                        }
                        _ => {}
                    }
                }
                Ok(Event::Empty(ref e)) => {
                    // Handle self-closing elements like <location/>
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                    match tag_name.as_str() {
                        "location" if state == ParseState::Message => {
                            if let Some(ref mut msg) = current_message {
                                let file = get_attr(e, b"filename").unwrap_or_default();
                                let line = get_attr(e, b"line")
                                    .and_then(|v| v.parse::<u32>().ok());
                                msg.locations.push(SourceRef { file, line });
                            }
                        }
                        _ => {}
                    }
                    // No state change for Empty events since no End event follows
                }
                Ok(Event::Text(ref e)) => {
                    let t = e.unescape().unwrap_or_default().to_string();
                    text_buf.push_str(&t);
                }
                Ok(Event::End(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                    match tag_name.as_str() {
                        "name" if state == ParseState::ContextName => {
                            current_context_name = text_buf.trim().to_string();
                            text_buf.clear();
                            state = ParseState::Context;
                        }
                        "source" if state == ParseState::Source => {
                            if let Some(ref mut msg) = current_message {
                                msg.source = text_buf.clone();
                            }
                            text_buf.clear();
                            state = ParseState::Message;
                        }
                        "oldsource" if state == ParseState::OldSource => {
                            if let Some(ref mut msg) = current_message {
                                msg.old_source = Some(text_buf.clone());
                            }
                            text_buf.clear();
                            state = ParseState::Message;
                        }
                        "numerusform" if state == ParseState::NumerusForm => {
                            if let Some(ref mut msg) = current_message {
                                msg.numerus_forms.push(text_buf.clone());
                            }
                            text_buf.clear();
                            state = ParseState::Translation;
                        }
                        "translation" if state == ParseState::Translation => {
                            if let Some(ref mut msg) = current_message {
                                // If not numerus (no numerusform children), use text_buf
                                if !msg.numerus {
                                    msg.translation = Some(text_buf.clone());
                                }
                            }
                            text_buf.clear();
                            state = ParseState::Message;
                        }
                        "comment" if state == ParseState::Comment => {
                            if let Some(ref mut msg) = current_message {
                                msg.comment = Some(text_buf.trim().to_string());
                            }
                            text_buf.clear();
                            state = ParseState::Message;
                        }
                        "translatorcomment" if state == ParseState::TranslatorComment => {
                            if let Some(ref mut msg) = current_message {
                                msg.translator_comment = Some(text_buf.trim().to_string());
                            }
                            text_buf.clear();
                            state = ParseState::Message;
                        }
                        "oldcomment" if state == ParseState::OldComment => {
                            if let Some(ref mut msg) = current_message {
                                msg.old_comment = Some(text_buf.trim().to_string());
                            }
                            text_buf.clear();
                            state = ParseState::Message;
                        }
                        other if state == ParseState::ExtraElement
                            && current_extra_name.as_deref() == Some(other) =>
                        {
                            if let Some(ref mut msg) = current_message {
                                if let Some(ref extra_name) = current_extra_name {
                                    msg.extra_elements
                                        .insert(extra_name.clone(), text_buf.trim().to_string());
                                }
                            }
                            current_extra_name = None;
                            text_buf.clear();
                            state = ParseState::Message;
                        }
                        "message" if state == ParseState::Message => {
                            if let Some(msg) = current_message.take() {
                                // Build the key: context + separator + source
                                let key = if current_context_name.is_empty() {
                                    msg.source.clone()
                                } else {
                                    format!(
                                        "{}{}{}",
                                        current_context_name, CONTEXT_SEPARATOR, msg.source
                                    )
                                };

                                // Determine translation state
                                let is_obsolete = matches!(
                                    msg.translation_type.as_deref(),
                                    Some("obsolete") | Some("vanished")
                                );
                                let ir_state =
                                    map_qt_state(msg.translation_type.as_deref());

                                // Build value
                                let value = if msg.numerus && !msg.numerus_forms.is_empty() {
                                    EntryValue::Plural(build_plural_set(&msg.numerus_forms))
                                } else {
                                    EntryValue::Simple(
                                        msg.translation.clone().unwrap_or_default(),
                                    )
                                };

                                // Build comments
                                let mut comments = Vec::new();
                                if let Some(ref c) = msg.comment {
                                    if !c.is_empty() {
                                        comments.push(Comment {
                                            text: c.clone(),
                                            role: CommentRole::Developer,
                                            priority: None,
                                            annotates: None,
                                        });
                                    }
                                }
                                if let Some(ref tc) = msg.translator_comment {
                                    if !tc.is_empty() {
                                        comments.push(Comment {
                                            text: tc.clone(),
                                            role: CommentRole::Translator,
                                            priority: None,
                                            annotates: None,
                                        });
                                    }
                                }

                                // Build contexts
                                let mut contexts = Vec::new();
                                if !current_context_name.is_empty() {
                                    contexts.push(ContextEntry {
                                        value: current_context_name.clone(),
                                        context_type: ContextType::Disambiguation,
                                        purpose: None,
                                    });
                                }

                                // Build format extension
                                let format_ext = {
                                    let ext = QtLinguistExt {
                                        numerus: if msg.numerus { Some(true) } else { None },
                                        extra_elements: msg.extra_elements,
                                    };
                                    if ext.numerus.is_some() || !ext.extra_elements.is_empty()
                                    {
                                        Some(FormatExtension::QtLinguist(ext))
                                    } else {
                                        None
                                    }
                                };

                                let entry = I18nEntry {
                                    key: key.clone(),
                                    value,
                                    comments,
                                    contexts,
                                    source: Some(msg.source),
                                    previous_source: msg.old_source,
                                    previous_comment: msg.old_comment,
                                    state: ir_state,
                                    obsolete: is_obsolete,
                                    source_references: msg.locations,
                                    format_ext,
                                    ..Default::default()
                                };

                                entries.insert(key, entry);
                            }
                            state = ParseState::Context;
                        }
                        "context" if state == ParseState::Context => {
                            state = ParseState::Ts;
                        }
                        "TS" => {
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

        Ok(I18nResource { metadata, entries })
    }

    fn capabilities(&self) -> FormatCapabilities {
        FormatCapabilities {
            plurals: true,
            arrays: false,
            comments: true,
            context: true,
            source_string: true,
            translatable_flag: false,
            translation_state: true,
            max_width: false,
            device_variants: false,
            select_gender: false,
            nested_keys: false,
            inline_markup: false,
            alternatives: false,
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
        writer
            .write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        // <TS>
        let mut ts_start = BytesStart::new("TS");

        // Version from properties
        let version = resource
            .metadata
            .properties
            .get("ts_version")
            .map(|s| s.clone())
            .unwrap_or_else(|| "2.1".to_string());
        ts_start.push_attribute(("version", version.as_str()));

        if let Some(ref lang) = resource.metadata.locale {
            ts_start.push_attribute(("language", lang.as_str()));
        }
        if let Some(ref src_lang) = resource.metadata.source_locale {
            ts_start.push_attribute(("sourcelanguage", src_lang.as_str()));
        }

        writer
            .write_event(Event::Start(ts_start))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        // Group entries by context name
        let mut context_groups: IndexMap<String, Vec<&I18nEntry>> = IndexMap::new();
        for (_key, entry) in &resource.entries {
            let context_name = entry
                .contexts
                .iter()
                .find(|c| c.context_type == ContextType::Disambiguation)
                .map(|c| c.value.clone())
                .unwrap_or_default();
            context_groups
                .entry(context_name)
                .or_default()
                .push(entry);
        }

        // Write each context group
        for (context_name, messages) in &context_groups {
            writer
                .write_event(Event::Start(BytesStart::new("context")))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;

            // <name>
            writer
                .write_event(Event::Start(BytesStart::new("name")))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;
            writer
                .write_event(Event::Text(BytesText::new(context_name)))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;
            writer
                .write_event(Event::End(BytesEnd::new("name")))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;

            for entry in messages {
                self.write_message(&mut writer, entry)?;
            }

            writer
                .write_event(Event::End(BytesEnd::new("context")))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;
        }

        // </TS>
        writer
            .write_event(Event::End(BytesEnd::new("TS")))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        drop(writer);

        // Insert <!DOCTYPE TS> after the XML declaration.
        // Find the end of the XML declaration (?>), then insert the DOCTYPE line.
        let xml_str = String::from_utf8(buf)
            .map_err(|e| WriteError::Serialization(e.to_string()))?;
        let result = if let Some(pos) = xml_str.find("?>") {
            let insert_pos = pos + 2;
            let mut output = String::with_capacity(xml_str.len() + 16);
            output.push_str(&xml_str[..insert_pos]);
            output.push_str("\n<!DOCTYPE TS>");
            output.push_str(&xml_str[insert_pos..]);
            output.into_bytes()
        } else {
            // No XML declaration found, prepend DOCTYPE
            let mut output = b"<!DOCTYPE TS>\n".to_vec();
            output.extend_from_slice(xml_str.as_bytes());
            output
        };

        Ok(result)
    }

    fn capabilities(&self) -> FormatCapabilities {
        Parser.capabilities()
    }
}

impl Writer {
    fn write_message(
        &self,
        writer: &mut XmlWriter<Cursor<&mut Vec<u8>>>,
        entry: &I18nEntry,
    ) -> Result<(), WriteError> {
        // Determine if this is a numerus message
        let is_numerus = matches!(&entry.value, EntryValue::Plural(_))
            || entry
                .format_ext
                .as_ref()
                .and_then(|ext| match ext {
                    FormatExtension::QtLinguist(qt) => qt.numerus,
                    _ => None,
                })
                .unwrap_or(false);

        let mut msg_start = BytesStart::new("message");
        if is_numerus {
            msg_start.push_attribute(("numerus", "yes"));
        }
        writer
            .write_event(Event::Start(msg_start))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        // <location> elements
        for src_ref in &entry.source_references {
            let mut loc = BytesStart::new("location");
            loc.push_attribute(("filename", src_ref.file.as_str()));
            if let Some(line) = src_ref.line {
                let line_str = line.to_string();
                loc.push_attribute(("line", line_str.as_str()));
            }
            writer
                .write_event(Event::Empty(loc))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;
        }

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

        // <oldsource>
        if let Some(ref old_source) = entry.previous_source {
            writer
                .write_event(Event::Start(BytesStart::new("oldsource")))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;
            writer
                .write_event(Event::Text(BytesText::new(old_source)))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;
            writer
                .write_event(Event::End(BytesEnd::new("oldsource")))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;
        }

        // <translation>
        {
            let mut trans_start = BytesStart::new("translation");
            if let Some(ref state) = entry.state {
                if let Some(type_str) = state_to_qt_type(state) {
                    trans_start.push_attribute(("type", type_str));
                }
            }

            match &entry.value {
                EntryValue::Plural(plural_set) => {
                    writer
                        .write_event(Event::Start(trans_start))
                        .map_err(|e| WriteError::Serialization(e.to_string()))?;

                    let forms = plural_set_to_forms(plural_set);
                    for form in &forms {
                        writer
                            .write_event(Event::Start(BytesStart::new("numerusform")))
                            .map_err(|e| WriteError::Serialization(e.to_string()))?;
                        writer
                            .write_event(Event::Text(BytesText::new(form)))
                            .map_err(|e| WriteError::Serialization(e.to_string()))?;
                        writer
                            .write_event(Event::End(BytesEnd::new("numerusform")))
                            .map_err(|e| WriteError::Serialization(e.to_string()))?;
                    }

                    writer
                        .write_event(Event::End(BytesEnd::new("translation")))
                        .map_err(|e| WriteError::Serialization(e.to_string()))?;
                }
                EntryValue::Simple(text) => {
                    writer
                        .write_event(Event::Start(trans_start))
                        .map_err(|e| WriteError::Serialization(e.to_string()))?;
                    writer
                        .write_event(Event::Text(BytesText::new(text)))
                        .map_err(|e| WriteError::Serialization(e.to_string()))?;
                    writer
                        .write_event(Event::End(BytesEnd::new("translation")))
                        .map_err(|e| WriteError::Serialization(e.to_string()))?;
                }
                _ => {
                    // For unsupported value types, write empty translation
                    writer
                        .write_event(Event::Start(trans_start))
                        .map_err(|e| WriteError::Serialization(e.to_string()))?;
                    writer
                        .write_event(Event::End(BytesEnd::new("translation")))
                        .map_err(|e| WriteError::Serialization(e.to_string()))?;
                }
            }
        }

        // <comment> (developer)
        for comment in &entry.comments {
            match comment.role {
                CommentRole::Developer | CommentRole::Extracted | CommentRole::General => {
                    if !comment.text.is_empty() {
                        writer
                            .write_event(Event::Start(BytesStart::new("comment")))
                            .map_err(|e| WriteError::Serialization(e.to_string()))?;
                        writer
                            .write_event(Event::Text(BytesText::new(&comment.text)))
                            .map_err(|e| WriteError::Serialization(e.to_string()))?;
                        writer
                            .write_event(Event::End(BytesEnd::new("comment")))
                            .map_err(|e| WriteError::Serialization(e.to_string()))?;
                        break; // Qt only supports one developer comment
                    }
                }
                _ => {}
            }
        }

        // <translatorcomment>
        for comment in &entry.comments {
            if comment.role == CommentRole::Translator && !comment.text.is_empty() {
                writer
                    .write_event(Event::Start(BytesStart::new("translatorcomment")))
                    .map_err(|e| WriteError::Serialization(e.to_string()))?;
                writer
                    .write_event(Event::Text(BytesText::new(&comment.text)))
                    .map_err(|e| WriteError::Serialization(e.to_string()))?;
                writer
                    .write_event(Event::End(BytesEnd::new("translatorcomment")))
                    .map_err(|e| WriteError::Serialization(e.to_string()))?;
                break; // Only one translator comment
            }
        }

        // <oldcomment>
        if let Some(ref old_comment) = entry.previous_comment {
            writer
                .write_event(Event::Start(BytesStart::new("oldcomment")))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;
            writer
                .write_event(Event::Text(BytesText::new(old_comment)))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;
            writer
                .write_event(Event::End(BytesEnd::new("oldcomment")))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;
        }

        // <extra-*> elements
        if let Some(FormatExtension::QtLinguist(ref ext)) = entry.format_ext {
            for (name, value) in &ext.extra_elements {
                writer
                    .write_event(Event::Start(BytesStart::new(name.as_str())))
                    .map_err(|e| WriteError::Serialization(e.to_string()))?;
                writer
                    .write_event(Event::Text(BytesText::new(value)))
                    .map_err(|e| WriteError::Serialization(e.to_string()))?;
                writer
                    .write_event(Event::End(BytesEnd::new(name.as_str())))
                    .map_err(|e| WriteError::Serialization(e.to_string()))?;
            }
        }

        // </message>
        writer
            .write_event(Event::End(BytesEnd::new("message")))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        Ok(())
    }
}
