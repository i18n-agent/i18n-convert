use crate::ir::*;
use super::*;

pub struct Parser;
pub struct Writer;

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        if extension == ".strings" {
            return Confidence::Definite;
        }
        if let Ok(s) = std::str::from_utf8(content) {
            // Pattern: "key" = "value";
            if s.contains("\" = \"") && s.contains(';') {
                return Confidence::High;
            }
        }
        Confidence::None
    }

    fn parse(&self, _content: &[u8]) -> Result<I18nResource, ParseError> {
        todo!("ios_strings parser")
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
            custom_properties: false,
        }
    }
}

impl FormatWriter for Writer {
    fn write(&self, _resource: &I18nResource) -> Result<Vec<u8>, WriteError> {
        todo!("ios_strings writer")
    }

    fn capabilities(&self) -> FormatCapabilities {
        Parser.capabilities()
    }
}
