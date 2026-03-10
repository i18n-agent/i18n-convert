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
    Header,
    Body,
    Tu,
    Tuv,
    Seg,
    Note,
    Prop,
}

// Accumulator for a single <tu>
#[derive(Debug, Default)]
struct TuBuilder {
    tuid: String,
    notes: Vec<String>,
    properties: IndexMap<String, String>,
    variants: Vec<TuvData>,
    change_date: Option<String>,
    change_id: Option<String>,
    // Current property type being parsed
    current_prop_type: Option<String>,
}

#[derive(Debug, Default)]
struct TuvData {
    lang: String,
    text: String,
}

// Helper to get an attribute value from a BytesStart event
fn get_attr(e: &BytesStart, name: &[u8]) -> Option<String> {
    e.attributes()
        .filter_map(|a| a.ok())
        .find(|a| a.key.as_ref() == name)
        .and_then(|a| String::from_utf8(a.value.to_vec()).ok())
}

// Helper to get xml:lang attribute (may appear as "xml:lang" or just "lang")
fn get_lang_attr(e: &BytesStart) -> Option<String> {
    e.attributes()
        .filter_map(|a| a.ok())
        .find(|a| {
            let key = a.key.as_ref();
            key == b"xml:lang" || key == b"lang"
        })
        .and_then(|a| String::from_utf8(a.value.to_vec()).ok())
}

// ---------------------------------------------------------------------------
// Parser implementation
// ---------------------------------------------------------------------------

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        if extension == ".tmx" {
            return Confidence::Definite;
        }
        if let Ok(s) = std::str::from_utf8(content) {
            if s.contains("<tmx") {
                return Confidence::High;
            }
        }
        Confidence::None
    }

    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError> {
        let mut reader = Reader::from_reader(content);
        reader.config_mut().trim_text(true);

        let mut metadata = ResourceMetadata {
            source_format: FormatId::Tmx,
            ..Default::default()
        };

        let mut entries = IndexMap::new();
        let mut state = ParseState::Root;
        let mut buf = Vec::new();
        let mut text_buf = String::new();

        let mut current_tu: Option<TuBuilder> = None;
        let mut current_tuv_lang: Option<String> = None;

        // Header-level data
        let mut seg_type: Option<String> = None;
        let mut o_tmf: Option<String> = None;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Eof) => break,
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let local_name = tag_name.split(':').last().unwrap_or(&tag_name);

                    match local_name {
                        "tmx" => {
                            // Root element
                        }
                        "header" => {
                            state = ParseState::Header;
                            metadata.source_locale = get_attr(e, b"srclang");
                            metadata.tool_name = get_attr(e, b"creationtool");
                            metadata.tool_version = get_attr(e, b"creationtoolversion");
                            seg_type = get_attr(e, b"segtype");
                            o_tmf = get_attr(e, b"o-tmf");

                            if let Some(ref admin_lang) = get_attr(e, b"adminlang") {
                                metadata.headers.insert("adminlang".to_string(), admin_lang.clone());
                            }
                            if let Some(ref datatype) = get_attr(e, b"datatype") {
                                metadata.headers.insert("datatype".to_string(), datatype.clone());
                            }
                        }
                        "body" => {
                            state = ParseState::Body;
                        }
                        "tu" => {
                            state = ParseState::Tu;
                            let tuid = get_attr(e, b"tuid").unwrap_or_default();
                            let change_date = get_attr(e, b"changedate");
                            let change_id = get_attr(e, b"changeid");
                            current_tu = Some(TuBuilder {
                                tuid,
                                change_date,
                                change_id,
                                ..Default::default()
                            });
                        }
                        "tuv" => {
                            state = ParseState::Tuv;
                            current_tuv_lang = get_lang_attr(e);
                        }
                        "seg" => {
                            state = ParseState::Seg;
                            text_buf.clear();
                        }
                        "note" if state == ParseState::Tu => {
                            state = ParseState::Note;
                            text_buf.clear();
                        }
                        "prop" if state == ParseState::Tu => {
                            state = ParseState::Prop;
                            if let Some(ref mut tu) = current_tu {
                                tu.current_prop_type = get_attr(e, b"type");
                            }
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
                        "seg" => {
                            if let Some(ref mut tu) = current_tu {
                                if let Some(ref lang) = current_tuv_lang {
                                    tu.variants.push(TuvData {
                                        lang: lang.clone(),
                                        text: text_buf.clone(),
                                    });
                                }
                            }
                            text_buf.clear();
                            state = ParseState::Tuv;
                        }
                        "tuv" => {
                            current_tuv_lang = None;
                            state = ParseState::Tu;
                        }
                        "note" if matches!(state, ParseState::Note) => {
                            if let Some(ref mut tu) = current_tu {
                                tu.notes.push(text_buf.clone());
                            }
                            text_buf.clear();
                            state = ParseState::Tu;
                        }
                        "prop" if matches!(state, ParseState::Prop) => {
                            if let Some(ref mut tu) = current_tu {
                                if let Some(prop_type) = tu.current_prop_type.take() {
                                    tu.properties.insert(prop_type, text_buf.clone());
                                }
                            }
                            text_buf.clear();
                            state = ParseState::Tu;
                        }
                        "tu" => {
                            if let Some(tu) = current_tu.take() {
                                let source_lang = metadata.source_locale.as_deref().unwrap_or("en");

                                // Find source and target variants
                                let source_text = tu.variants.iter()
                                    .find(|v| v.lang == source_lang)
                                    .map(|v| v.text.clone());

                                // Find the first non-source language as target
                                let target_variant = tu.variants.iter()
                                    .find(|v| v.lang != source_lang);

                                let target_text = target_variant.map(|v| v.text.clone());
                                let target_lang = target_variant.map(|v| v.lang.clone());

                                // Set locale from first target we encounter (if not set)
                                if metadata.locale.is_none() {
                                    if let Some(ref tl) = target_lang {
                                        metadata.locale = Some(tl.clone());
                                    }
                                }

                                let comments: Vec<Comment> = tu.notes.iter().map(|n| {
                                    Comment {
                                        text: n.clone(),
                                        role: CommentRole::General,
                                        priority: None,
                                        annotates: None,
                                    }
                                }).collect();

                                let value = target_text
                                    .map(EntryValue::Simple)
                                    .unwrap_or_else(|| EntryValue::Simple(String::new()));

                                // Build properties from tu-level props + change metadata
                                let mut properties = tu.properties.clone();
                                if let Some(ref cd) = tu.change_date {
                                    properties.insert("changedate".to_string(), cd.clone());
                                }
                                if let Some(ref ci) = tu.change_id {
                                    properties.insert("changeid".to_string(), ci.clone());
                                }

                                let key = if tu.tuid.is_empty() {
                                    // Generate a key from source text if no tuid
                                    source_text.clone().unwrap_or_else(|| format!("entry_{}", entries.len()))
                                } else {
                                    tu.tuid.clone()
                                };

                                let entry = I18nEntry {
                                    key: key.clone(),
                                    value,
                                    comments,
                                    source: source_text,
                                    properties,
                                    ..Default::default()
                                };

                                entries.insert(key, entry);
                            }
                            state = ParseState::Body;
                        }
                        "body" => {
                            state = ParseState::Root;
                        }
                        "header" => {
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

        // Set format extension
        metadata.format_ext = Some(FormatExtension::Tmx(TmxExt {
            seg_type,
            o_tmf,
        }));

        Ok(I18nResource { metadata, entries })
    }

    fn capabilities(&self) -> FormatCapabilities {
        FormatCapabilities {
            plurals: false,
            arrays: false,
            comments: true,
            context: false,
            source_string: true,
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

        // <tmx>
        let mut tmx_start = BytesStart::new("tmx");
        tmx_start.push_attribute(("version", "1.4"));
        writer.write_event(Event::Start(tmx_start))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        // <header>
        let mut header = BytesStart::new("header");

        let tool_name = resource.metadata.tool_name.as_deref().unwrap_or("i18n-convert");
        let tool_version = resource.metadata.tool_version.as_deref().unwrap_or("0.1.0");
        header.push_attribute(("creationtool", tool_name));
        header.push_attribute(("creationtoolversion", tool_version));

        let (seg_type, o_tmf) = match &resource.metadata.format_ext {
            Some(FormatExtension::Tmx(ext)) => (
                ext.seg_type.as_deref(),
                ext.o_tmf.as_deref(),
            ),
            _ => (None, None),
        };

        header.push_attribute(("segtype", seg_type.unwrap_or("sentence")));
        header.push_attribute(("o-tmf", o_tmf.unwrap_or("undefined")));

        let admin_lang = resource.metadata.headers.get("adminlang")
            .map(|s| s.as_str())
            .unwrap_or("en");
        header.push_attribute(("adminlang", admin_lang));

        let src_lang = resource.metadata.source_locale.as_deref().unwrap_or("en");
        header.push_attribute(("srclang", src_lang));

        let datatype = resource.metadata.headers.get("datatype")
            .map(|s| s.as_str())
            .unwrap_or("plaintext");
        header.push_attribute(("datatype", datatype));

        // Write as empty element
        writer.write_event(Event::Empty(header))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        // <body>
        writer.write_event(Event::Start(BytesStart::new("body")))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        let trg_lang = resource.metadata.locale.as_deref().unwrap_or("");

        for (_key, entry) in &resource.entries {
            self.write_tu(&mut writer, entry, src_lang, trg_lang)?;
        }

        // </body>
        writer.write_event(Event::End(BytesEnd::new("body")))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        // </tmx>
        writer.write_event(Event::End(BytesEnd::new("tmx")))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        drop(writer);
        Ok(buf)
    }

    fn capabilities(&self) -> FormatCapabilities {
        Parser.capabilities()
    }
}

impl Writer {
    fn write_tu(
        &self,
        writer: &mut XmlWriter<Cursor<&mut Vec<u8>>>,
        entry: &I18nEntry,
        src_lang: &str,
        trg_lang: &str,
    ) -> Result<(), WriteError> {
        let mut tu_start = BytesStart::new("tu");
        tu_start.push_attribute(("tuid", entry.key.as_str()));

        // Write changedate and changeid from properties
        if let Some(cd) = entry.properties.get("changedate") {
            tu_start.push_attribute(("changedate", cd.as_str()));
        }
        if let Some(ci) = entry.properties.get("changeid") {
            tu_start.push_attribute(("changeid", ci.as_str()));
        }

        writer.write_event(Event::Start(tu_start))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        // <note> elements
        for comment in &entry.comments {
            writer.write_event(Event::Start(BytesStart::new("note")))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;
            writer.write_event(Event::Text(BytesText::new(&comment.text)))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;
            writer.write_event(Event::End(BytesEnd::new("note")))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;
        }

        // <prop> elements (excluding changedate/changeid which are on <tu>)
        for (prop_type, prop_value) in &entry.properties {
            if prop_type == "changedate" || prop_type == "changeid" {
                continue;
            }
            let mut prop_start = BytesStart::new("prop");
            prop_start.push_attribute(("type", prop_type.as_str()));
            writer.write_event(Event::Start(prop_start))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;
            writer.write_event(Event::Text(BytesText::new(prop_value)))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;
            writer.write_event(Event::End(BytesEnd::new("prop")))
                .map_err(|e| WriteError::Serialization(e.to_string()))?;
        }

        // Source <tuv>
        if let Some(ref source) = entry.source {
            self.write_tuv(writer, src_lang, source)?;
        }

        // Target <tuv>
        let target_text = match &entry.value {
            EntryValue::Simple(s) => Some(s.clone()),
            other => {
                eprintln!("TMX writer: skipping unsupported value type: {:?}", other);
                None
            }
        };

        if let Some(ref text) = target_text {
            if !text.is_empty() {
                self.write_tuv(writer, trg_lang, text)?;
            }
        }

        // </tu>
        writer.write_event(Event::End(BytesEnd::new("tu")))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        Ok(())
    }

    fn write_tuv(
        &self,
        writer: &mut XmlWriter<Cursor<&mut Vec<u8>>>,
        lang: &str,
        text: &str,
    ) -> Result<(), WriteError> {
        let mut tuv_start = BytesStart::new("tuv");
        tuv_start.push_attribute(("xml:lang", lang));
        writer.write_event(Event::Start(tuv_start))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        writer.write_event(Event::Start(BytesStart::new("seg")))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;
        writer.write_event(Event::Text(BytesText::new(text)))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;
        writer.write_event(Event::End(BytesEnd::new("seg")))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        writer.write_event(Event::End(BytesEnd::new("tuv")))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        Ok(())
    }
}
