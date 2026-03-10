use crate::ir::*;
use super::*;
use quick_xml::events::Event;
use quick_xml::reader::Reader;

pub struct Parser;
pub struct Writer;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Get an attribute value by name from a BytesStart tag.
fn get_attr(tag: &quick_xml::events::BytesStart, name: &[u8]) -> Option<String> {
    for attr in tag.attributes().flatten() {
        if attr.key.as_ref() == name {
            return String::from_utf8(attr.value.to_vec()).ok();
        }
    }
    None
}

/// Read the full inner text/XML content of an element, including any inline
/// child elements like `<xliff:g>`. Returns the raw text with inline tags
/// preserved as-is, plus a list of extracted Placeholder entries.
fn read_element_content(
    reader: &mut Reader<&[u8]>,
) -> Result<(String, Vec<Placeholder>), ParseError> {
    let mut text = String::new();
    let mut placeholders = Vec::new();
    let mut depth: u32 = 1;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Text(e)) => {
                let decoded = e.unescape().map_err(|e| ParseError::Xml(e.to_string()))?;
                text.push_str(&decoded);
            }
            Ok(Event::CData(e)) => {
                let decoded = String::from_utf8_lossy(e.as_ref());
                text.push_str(&decoded);
            }
            Ok(Event::Start(ref e)) => {
                depth += 1;
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                if tag_name == "xliff:g" || tag_name.ends_with(":g") {
                    // Extract placeholder metadata from attributes
                    let id = get_attr(e, b"id");
                    let example = get_attr(e, b"example");

                    // Read the inner content of the xliff:g tag
                    let (inner, _) = read_element_content(reader)?;
                    depth -= 1; // read_element_content consumed the end tag

                    // Build the original syntax representation
                    let mut original_parts = format!("<{}", tag_name);
                    for attr in e.attributes().flatten() {
                        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                        let val = String::from_utf8_lossy(&attr.value).to_string();
                        original_parts.push_str(&format!(" {}=\"{}\"", key, val));
                    }
                    original_parts.push('>');
                    original_parts.push_str(&inner);
                    original_parts.push_str(&format!("</{}>", tag_name));

                    let placeholder_name = id.clone().unwrap_or_else(|| inner.clone());

                    placeholders.push(Placeholder {
                        name: placeholder_name,
                        original_syntax: inner.clone(),
                        placeholder_type: None,
                        position: None,
                        example,
                        description: None,
                        format: None,
                        optional_parameters: None,
                    });

                    // Append the full xliff:g markup to text for lossless output
                    text.push_str(&original_parts);
                } else {
                    // Other inline tags: reconstruct and include literally
                    let mut tag_str = format!("<{}", tag_name);
                    for attr in e.attributes().flatten() {
                        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                        let val = String::from_utf8_lossy(&attr.value).to_string();
                        tag_str.push_str(&format!(" {}=\"{}\"", key, val));
                    }
                    tag_str.push('>');
                    text.push_str(&tag_str);
                }
            }
            Ok(Event::End(ref e)) => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                text.push_str(&format!("</{}>", tag_name));
            }
            Ok(Event::Empty(ref e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let mut tag_str = format!("<{}", tag_name);
                for attr in e.attributes().flatten() {
                    let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                    let val = String::from_utf8_lossy(&attr.value).to_string();
                    tag_str.push_str(&format!(" {}=\"{}\"", key, val));
                }
                tag_str.push_str(" />");
                text.push_str(&tag_str);
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(ParseError::Xml(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    Ok((text, placeholders))
}

/// Read just the text content of a simple element (like <item>).
fn read_text_content(reader: &mut Reader<&[u8]>) -> Result<String, ParseError> {
    let (text, _) = read_element_content(reader)?;
    Ok(text)
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        if extension == ".xml" {
            if let Ok(s) = std::str::from_utf8(content) {
                if s.contains("<resources") {
                    return Confidence::Definite;
                }
            }
            return Confidence::Low;
        }
        Confidence::None
    }

    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError> {
        let mut reader = Reader::from_reader(content);
        reader.config_mut().trim_text(false);

        let mut entries = IndexMap::new();
        let mut buf = Vec::new();
        let mut pending_comment: Option<String> = None;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Comment(e)) => {
                    let comment_text = String::from_utf8_lossy(e.as_ref()).trim().to_string();
                    pending_comment = Some(comment_text);
                }

                Ok(Event::Start(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                    match tag_name.as_str() {
                        "string" => {
                            let name = get_attr(e, b"name")
                                .ok_or_else(|| ParseError::Xml("string missing name attr".into()))?;
                            let translatable = get_attr(e, b"translatable");
                            let formatted = get_attr(e, b"formatted");
                            let product = get_attr(e, b"product");

                            let (text, placeholders) = read_element_content(&mut reader)?;

                            let mut comments = Vec::new();
                            if let Some(c) = pending_comment.take() {
                                comments.push(Comment {
                                    text: c,
                                    role: CommentRole::General,
                                    priority: None,
                                    annotates: None,
                                });
                            }

                            let translatable_bool = translatable.as_deref().map(|v| parse_translatable(v, &name));

                            // Build format extension if we have formatted or product
                            let format_ext = if formatted.is_some() || product.is_some() {
                                Some(FormatExtension::AndroidXml(AndroidXmlExt {
                                    formatted: formatted.as_deref().map(|v| v == "true"),
                                    product,
                                    xml_comments: Vec::new(),
                                }))
                            } else {
                                None
                            };

                            let entry = I18nEntry {
                                key: name.clone(),
                                value: EntryValue::Simple(text),
                                comments,
                                placeholders,
                                translatable: translatable_bool,
                                format_ext,
                                ..Default::default()
                            };

                            entries.insert(name, entry);
                        }

                        "plurals" => {
                            let name = get_attr(e, b"name")
                                .ok_or_else(|| ParseError::Xml("plurals missing name attr".into()))?;

                            let mut comments = Vec::new();
                            if let Some(c) = pending_comment.take() {
                                comments.push(Comment {
                                    text: c,
                                    role: CommentRole::General,
                                    priority: None,
                                    annotates: None,
                                });
                            }

                            let mut plural_set = PluralSet::default();
                            let mut inner_buf = Vec::new();

                            // Read inner <item> elements
                            loop {
                                match reader.read_event_into(&mut inner_buf) {
                                    Ok(Event::Start(ref inner_e)) => {
                                        let inner_tag = String::from_utf8_lossy(inner_e.name().as_ref()).to_string();
                                        if inner_tag == "item" {
                                            let quantity = get_attr(inner_e, b"quantity")
                                                .unwrap_or_default();
                                            let text = read_text_content(&mut reader)?;

                                            match quantity.as_str() {
                                                "zero" => plural_set.zero = Some(text),
                                                "one" => plural_set.one = Some(text),
                                                "two" => plural_set.two = Some(text),
                                                "few" => plural_set.few = Some(text),
                                                "many" => plural_set.many = Some(text),
                                                "other" => plural_set.other = text,
                                                _ => {} // ignore unknown quantities
                                            }
                                        }
                                    }
                                    Ok(Event::End(ref inner_e)) => {
                                        let inner_tag = String::from_utf8_lossy(inner_e.name().as_ref()).to_string();
                                        if inner_tag == "plurals" {
                                            break;
                                        }
                                    }
                                    Ok(Event::Eof) => break,
                                    Err(e) => return Err(ParseError::Xml(e.to_string())),
                                    _ => {}
                                }
                                inner_buf.clear();
                            }

                            let entry = I18nEntry {
                                key: name.clone(),
                                value: EntryValue::Plural(plural_set),
                                comments,
                                ..Default::default()
                            };

                            entries.insert(name, entry);
                        }

                        "string-array" => {
                            let name = get_attr(e, b"name")
                                .ok_or_else(|| ParseError::Xml("string-array missing name attr".into()))?;

                            let mut comments = Vec::new();
                            if let Some(c) = pending_comment.take() {
                                comments.push(Comment {
                                    text: c,
                                    role: CommentRole::General,
                                    priority: None,
                                    annotates: None,
                                });
                            }

                            let mut items = Vec::new();
                            let mut inner_buf = Vec::new();

                            loop {
                                match reader.read_event_into(&mut inner_buf) {
                                    Ok(Event::Start(ref inner_e)) => {
                                        let inner_tag = String::from_utf8_lossy(inner_e.name().as_ref()).to_string();
                                        if inner_tag == "item" {
                                            let text = read_text_content(&mut reader)?;
                                            items.push(text);
                                        }
                                    }
                                    Ok(Event::End(ref inner_e)) => {
                                        let inner_tag = String::from_utf8_lossy(inner_e.name().as_ref()).to_string();
                                        if inner_tag == "string-array" {
                                            break;
                                        }
                                    }
                                    Ok(Event::Eof) => break,
                                    Err(e) => return Err(ParseError::Xml(e.to_string())),
                                    _ => {}
                                }
                                inner_buf.clear();
                            }

                            let entry = I18nEntry {
                                key: name.clone(),
                                value: EntryValue::Array(items),
                                comments,
                                ..Default::default()
                            };

                            entries.insert(name, entry);
                        }

                        // resources tag, or anything else - skip
                        _ => {
                            // Don't consume the pending comment for non-entry tags
                        }
                    }
                }

                Ok(Event::Empty(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    if tag_name == "string" {
                        // Self-closing <string name="key" /> means empty string
                        let name = get_attr(e, b"name")
                            .ok_or_else(|| ParseError::Xml("string missing name attr".into()))?;
                        let translatable = get_attr(e, b"translatable");

                        let mut comments = Vec::new();
                        if let Some(c) = pending_comment.take() {
                            comments.push(Comment {
                                text: c,
                                role: CommentRole::General,
                                priority: None,
                                annotates: None,
                            });
                        }

                        let entry = I18nEntry {
                            key: name.clone(),
                            value: EntryValue::Simple(String::new()),
                            comments,
                            translatable: translatable.as_deref().map(|v| parse_translatable(v, &name)),
                            ..Default::default()
                        };

                        entries.insert(name, entry);
                    }
                }

                Ok(Event::Eof) => break,
                Err(e) => return Err(ParseError::Xml(e.to_string())),

                // Whitespace text, decl, PI, etc.
                _ => {}
            }
            buf.clear();
        }

        Ok(I18nResource {
            metadata: ResourceMetadata {
                source_format: FormatId::AndroidXml,
                ..Default::default()
            },
            entries,
        })
    }

    fn capabilities(&self) -> FormatCapabilities {
        FormatCapabilities {
            plurals: true,
            arrays: true,
            comments: true,
            context: false,
            source_string: false,
            translatable_flag: true,
            translation_state: false,
            max_width: false,
            device_variants: true,
            select_gender: false,
            nested_keys: false,
            inline_markup: true,
            alternatives: false,
            source_references: false,
            custom_properties: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Writer — builds output as a string for full control over inline XML
// ---------------------------------------------------------------------------

/// Escape special XML characters in text content.
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Check whether a string value contains inline XML (like xliff:g tags).
fn contains_inline_xml(s: &str) -> bool {
    s.contains("<xliff:g") || s.contains("</xliff:g>")
}

/// Parse a translatable attribute value. Android XML recognizes "true" and "false".
fn parse_translatable(value: &str, key: &str) -> bool {
    match value {
        "true" => true,
        "false" => false,
        other => {
            eprintln!(
                "Warning: unrecognized translatable value '{}' for key '{}', treating as true",
                other, key
            );
            true
        }
    }
}

/// Escape a string for use as an XML attribute value.
fn xml_attr_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

impl FormatWriter for Writer {
    fn write(&self, resource: &I18nResource) -> Result<Vec<u8>, WriteError> {
        let mut out = String::new();

        // XML declaration
        out.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");

        // Check if any entry has xliff:g placeholders -> need namespace
        let needs_xliff_ns = resource.entries.values().any(|entry| match &entry.value {
            EntryValue::Simple(s) => contains_inline_xml(s),
            EntryValue::Plural(ps) => {
                contains_inline_xml(&ps.other)
                    || ps.zero.as_deref().is_some_and(contains_inline_xml)
                    || ps.one.as_deref().is_some_and(contains_inline_xml)
                    || ps.two.as_deref().is_some_and(contains_inline_xml)
                    || ps.few.as_deref().is_some_and(contains_inline_xml)
                    || ps.many.as_deref().is_some_and(contains_inline_xml)
            }
            EntryValue::Array(items) => items.iter().any(|s| contains_inline_xml(s)),
            _ => false,
        });

        // <resources>
        if needs_xliff_ns {
            out.push_str("<resources xmlns:xliff=\"urn:oasis:names:tc:xliff:document:1.2\">\n");
        } else {
            out.push_str("<resources>\n");
        }

        for (_key, entry) in &resource.entries {
            // Write preceding comment if any
            for comment in &entry.comments {
                out.push_str(&format!("    <!-- {} -->\n", comment.text));
            }

            match &entry.value {
                EntryValue::Simple(text) => {
                    out.push_str("    <string name=\"");
                    out.push_str(&xml_attr_escape(&entry.key));
                    out.push('"');

                    if entry.translatable == Some(false) {
                        out.push_str(" translatable=\"false\"");
                    }

                    // Check for formatted attr in format_ext
                    if let Some(FormatExtension::AndroidXml(ext)) = &entry.format_ext {
                        if let Some(formatted) = ext.formatted {
                            out.push_str(&format!(
                                " formatted=\"{}\"",
                                if formatted { "true" } else { "false" }
                            ));
                        }
                        if let Some(ref product) = ext.product {
                            out.push_str(&format!(" product=\"{}\"", xml_attr_escape(product)));
                        }
                    }

                    if text.is_empty() {
                        out.push_str(" />\n");
                    } else if contains_inline_xml(text) {
                        // Inline XML: write raw content (already properly formatted)
                        out.push('>');
                        out.push_str(text);
                        out.push_str("</string>\n");
                    } else {
                        out.push('>');
                        out.push_str(&xml_escape(text));
                        out.push_str("</string>\n");
                    }
                }

                EntryValue::Plural(plural_set) => {
                    out.push_str("    <plurals name=\"");
                    out.push_str(&xml_attr_escape(&entry.key));
                    out.push_str("\">\n");

                    // Write each quantity in CLDR order
                    let quantities: &[(&str, &Option<String>)] = &[
                        ("zero", &plural_set.zero),
                        ("one", &plural_set.one),
                        ("two", &plural_set.two),
                        ("few", &plural_set.few),
                        ("many", &plural_set.many),
                    ];

                    for (quantity, value) in quantities {
                        if let Some(text) = value {
                            out.push_str(&format!(
                                "        <item quantity=\"{}\">{}</item>\n",
                                quantity,
                                xml_escape(text)
                            ));
                        }
                    }

                    // Always write "other"
                    out.push_str(&format!(
                        "        <item quantity=\"other\">{}</item>\n",
                        xml_escape(&plural_set.other)
                    ));

                    out.push_str("    </plurals>\n");
                }

                EntryValue::Array(items) => {
                    out.push_str("    <string-array name=\"");
                    out.push_str(&xml_attr_escape(&entry.key));
                    out.push_str("\">\n");

                    for item in items {
                        out.push_str(&format!("        <item>{}</item>\n", xml_escape(item)));
                    }

                    out.push_str("    </string-array>\n");
                }

                _ => {
                    eprintln!(
                        "Warning: skipping entry '{}' with unsupported value type for Android XML",
                        entry.key
                    );
                }
            }
        }

        out.push_str("</resources>\n");

        Ok(out.into_bytes())
    }

    fn capabilities(&self) -> FormatCapabilities {
        Parser.capabilities()
    }
}
