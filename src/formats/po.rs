use crate::ir::*;
use super::*;

pub struct Parser;
pub struct Writer;

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        if extension == ".po" || extension == ".pot" {
            return Confidence::Definite;
        }
        if let Ok(s) = std::str::from_utf8(content) {
            if s.contains("msgid") && s.contains("msgstr") {
                return Confidence::Definite;
            }
        }
        Confidence::None
    }

    fn parse(&self, _content: &[u8]) -> Result<I18nResource, ParseError> {
        todo!("po parser")
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
            custom_properties: false,
        }
    }
}

impl FormatWriter for Writer {
    fn write(&self, _resource: &I18nResource) -> Result<Vec<u8>, WriteError> {
        todo!("po writer")
    }

    fn capabilities(&self) -> FormatCapabilities {
        Parser.capabilities()
    }
}
