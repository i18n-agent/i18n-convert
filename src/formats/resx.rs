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
    ResHeader,
    ResHeaderValue,
    Data,
    DataValue,
    DataComment,
    Schema,
}

// Accumulator for a single <data> element
#[derive(Debug, Default)]
struct DataBuilder {
    name: String,
    value: String,
    comment: Option<String>,
    type_name: Option<String>,
    mimetype: Option<String>,
    #[allow(dead_code)]
    xml_space: Option<String>,
}

/// Check for xml:space attribute (the local name is "space" under the xml namespace)
fn get_xml_space(e: &BytesStart) -> Option<String> {
    e.attributes()
        .filter_map(|a| a.ok())
        .find(|a| {
            let key = a.key.as_ref();
            key == b"xml:space"
        })
        .and_then(|a| String::from_utf8(a.value.to_vec()).ok())
}

// ---------------------------------------------------------------------------
// Parser implementation
// ---------------------------------------------------------------------------

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        if extension == ".resx" {
            return Confidence::Definite;
        }
        if extension == ".xml" {
            if let Ok(s) = std::str::from_utf8(content) {
                if s.contains("<root>") && s.contains("<resheader") {
                    return Confidence::High;
                }
            }
        }
        Confidence::None
    }

    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError> {
        let mut reader = Reader::from_reader(content);
        reader.config_mut().trim_text(false);

        let mut metadata = ResourceMetadata {
            source_format: FormatId::Resx,
            ..Default::default()
        };

        let mut entries = IndexMap::new();
        let mut state = ParseState::Root;
        let mut buf = Vec::new();
        let mut text_buf = String::new();

        let mut current_data: Option<DataBuilder> = None;
        let mut current_resheader_name: Option<String> = None;
        let mut schema_text = String::new();
        let mut schema_depth: u32 = 0;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Eof) => break,
                Ok(Event::Start(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let local_name = tag_name.split(':').next_back().unwrap_or(&tag_name);

                    match state {
                        ParseState::Schema => {
                            schema_depth += 1;
                        }
                        _ => match local_name {
                            "root" => {
                                state = ParseState::Root;
                            }
                            "schema" if state == ParseState::Root => {
                                state = ParseState::Schema;
                                schema_depth = 1;
                                schema_text.clear();
                            }
                            "resheader" if state == ParseState::Root => {
                                state = ParseState::ResHeader;
                                current_resheader_name = get_attr(e, b"name");
                            }
                            "value" if state == ParseState::ResHeader => {
                                state = ParseState::ResHeaderValue;
                                text_buf.clear();
                            }
                            "data" if state == ParseState::Root => {
                                state = ParseState::Data;
                                let name = get_attr(e, b"name").unwrap_or_default();
                                let type_name = get_attr(e, b"type");
                                let mimetype = get_attr(e, b"mimetype");
                                let xml_space = get_xml_space(e);
                                current_data = Some(DataBuilder {
                                    name,
                                    type_name,
                                    mimetype,
                                    xml_space,
                                    ..Default::default()
                                });
                            }
                            "value" if state == ParseState::Data => {
                                state = ParseState::DataValue;
                                text_buf.clear();
                            }
                            "comment" if state == ParseState::Data => {
                                state = ParseState::DataComment;
                                text_buf.clear();
                            }
                            _ => {}
                        },
                    }
                }
                Ok(Event::Empty(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let local_name = tag_name.split(':').next_back().unwrap_or(&tag_name);

                    match local_name {
                        "data" if state == ParseState::Root => {
                            // Self-closing data element (unusual but possible)
                            let name = get_attr(e, b"name").unwrap_or_default();
                            let entry = I18nEntry {
                                key: name.clone(),
                                value: EntryValue::Simple(String::new()),
                                ..Default::default()
                            };
                            entries.insert(name, entry);
                        }
                        _ => {}
                    }
                }
                Ok(Event::Text(ref e)) => {
                    match state {
                        ParseState::Schema => {
                            // Accumulate schema text if needed
                        }
                        _ => {
                            let t = e.unescape().unwrap_or_default().to_string();
                            text_buf.push_str(&t);
                        }
                    }
                }
                Ok(Event::End(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let local_name = tag_name.split(':').next_back().unwrap_or(&tag_name);

                    match state {
                        ParseState::Schema => {
                            if local_name == "schema" {
                                schema_depth -= 1;
                                if schema_depth == 0 {
                                    state = ParseState::Root;
                                }
                            } else {
                                schema_depth = schema_depth.saturating_sub(1);
                            }
                        }
                        _ => match local_name {
                            "value" if state == ParseState::ResHeaderValue => {
                                if let Some(ref name) = current_resheader_name {
                                    metadata
                                        .headers
                                        .insert(name.clone(), text_buf.trim().to_string());
                                }
                                text_buf.clear();
                                state = ParseState::ResHeader;
                            }
                            "resheader" => {
                                current_resheader_name = None;
                                state = ParseState::Root;
                            }
                            "value" if state == ParseState::DataValue => {
                                if let Some(ref mut data) = current_data {
                                    data.value = text_buf.clone();
                                }
                                text_buf.clear();
                                state = ParseState::Data;
                            }
                            "comment" if state == ParseState::DataComment => {
                                if let Some(ref mut data) = current_data {
                                    data.comment = Some(text_buf.trim().to_string());
                                }
                                text_buf.clear();
                                state = ParseState::Data;
                            }
                            "data" if state == ParseState::Data => {
                                if let Some(data) = current_data.take() {
                                    let mut comments = Vec::new();
                                    if let Some(ref comment_text) = data.comment {
                                        if !comment_text.is_empty() {
                                            comments.push(Comment {
                                                text: comment_text.clone(),
                                                role: CommentRole::Developer,
                                                priority: None,
                                                annotates: None,
                                            });
                                        }
                                    }

                                    let format_ext =
                                        if data.type_name.is_some() || data.mimetype.is_some() {
                                            Some(FormatExtension::Resx(ResxExt {
                                                type_name: data.type_name,
                                                mimetype: data.mimetype,
                                                schema: None,
                                            }))
                                        } else {
                                            None
                                        };

                                    let entry = I18nEntry {
                                        key: data.name.clone(),
                                        value: EntryValue::Simple(data.value),
                                        comments,
                                        format_ext,
                                        ..Default::default()
                                    };

                                    entries.insert(data.name, entry);
                                }
                                state = ParseState::Root;
                            }
                            "root" => {
                                state = ParseState::Root;
                            }
                            _ => {}
                        },
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
        writer
            .write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        // <root>
        writer
            .write_event(Event::Start(BytesStart::new("root")))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        // Write resheaders from metadata
        let default_headers = vec![("resmimetype", "text/microsoft-resx"), ("version", "2.0")];

        // Merge: use metadata headers if present, otherwise use defaults
        let mut written_headers = std::collections::HashSet::new();
        for (name, default_value) in &default_headers {
            let value = resource
                .metadata
                .headers
                .get(*name)
                .map(|s| s.as_str())
                .unwrap_or(default_value);

            self.write_resheader(&mut writer, name, value)?;
            written_headers.insert(name.to_string());
        }

        // Write any additional headers from metadata that are not defaults
        for (name, value) in &resource.metadata.headers {
            if !written_headers.contains(name.as_str()) {
                self.write_resheader(&mut writer, name, value)?;
            }
        }

        // Write data entries
        for (_key, entry) in &resource.entries {
            self.write_data_element(&mut writer, entry)?;
        }

        // </root>
        writer
            .write_event(Event::End(BytesEnd::new("root")))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        drop(writer);
        Ok(buf)
    }

    fn capabilities(&self) -> FormatCapabilities {
        Parser.capabilities()
    }
}

impl Writer {
    fn write_resheader(
        &self,
        writer: &mut XmlWriter<Cursor<&mut Vec<u8>>>,
        name: &str,
        value: &str,
    ) -> Result<(), WriteError> {
        let mut start = BytesStart::new("resheader");
        start.push_attribute(("name", name));
        writer
            .write_event(Event::Start(start))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        writer
            .write_event(Event::Start(BytesStart::new("value")))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;
        writer
            .write_event(Event::Text(BytesText::new(value)))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;
        writer
            .write_event(Event::End(BytesEnd::new("value")))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        writer
            .write_event(Event::End(BytesEnd::new("resheader")))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        Ok(())
    }

    fn write_data_element(
        &self,
        writer: &mut XmlWriter<Cursor<&mut Vec<u8>>>,
        entry: &I18nEntry,
    ) -> Result<(), WriteError> {
        let mut data_start = BytesStart::new("data");
        data_start.push_attribute(("name", entry.key.as_str()));
        data_start.push_attribute(("xml:space", "preserve"));

        // Add type and mimetype from entry extension
        if let Some(FormatExtension::Resx(ref ext)) = entry.format_ext {
            if let Some(ref type_name) = ext.type_name {
                data_start.push_attribute(("type", type_name.as_str()));
            }
            if let Some(ref mimetype) = ext.mimetype {
                data_start.push_attribute(("mimetype", mimetype.as_str()));
            }
        }

        writer
            .write_event(Event::Start(data_start))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        // <value>
        let text = match &entry.value {
            EntryValue::Simple(s) => s.clone(),
            _ => String::new(),
        };

        writer
            .write_event(Event::Start(BytesStart::new("value")))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;
        writer
            .write_event(Event::Text(BytesText::new(&text)))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;
        writer
            .write_event(Event::End(BytesEnd::new("value")))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        // <comment> (first developer comment)
        for comment in &entry.comments {
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
                // RESX only supports one comment per data element
                break;
            }
        }

        // </data>
        writer
            .write_event(Event::End(BytesEnd::new("data")))
            .map_err(|e| WriteError::Serialization(e.to_string()))?;

        Ok(())
    }
}
