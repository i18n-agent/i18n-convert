use crate::ir::*;
use super::*;

pub struct Parser;
pub struct Writer;

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        if extension == ".json" {
            if let Ok(s) = std::str::from_utf8(content) {
                // Exclude ARB (has @@locale) and xcstrings (has sourceLanguage+strings)
                if s.contains("\"@@locale\"") {
                    return Confidence::None;
                }
                if s.contains("\"sourceLanguage\"") && s.contains("\"strings\"") {
                    return Confidence::None;
                }
                // Check for i18next plural suffixes
                if s.contains("_one\"") || s.contains("_other\"") {
                    return Confidence::None;
                }
                // Generic JSON with string values
                if s.trim_start().starts_with('{') {
                    return Confidence::Low;
                }
            }
        }
        Confidence::None
    }

    fn parse(&self, _content: &[u8]) -> Result<I18nResource, ParseError> {
        todo!("json_structured parser")
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
            inline_markup: false,
            alternatives: false,
            source_references: false,
            custom_properties: false,
        }
    }
}

impl FormatWriter for Writer {
    fn write(&self, _resource: &I18nResource) -> Result<Vec<u8>, WriteError> {
        todo!("json_structured writer")
    }

    fn capabilities(&self) -> FormatCapabilities {
        Parser.capabilities()
    }
}
