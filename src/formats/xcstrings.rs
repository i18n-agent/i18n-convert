use crate::ir::*;
use super::*;

pub struct Parser;
pub struct Writer;

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        if extension == ".xcstrings" {
            return Confidence::Definite;
        }
        if extension == ".json" {
            if let Ok(s) = std::str::from_utf8(content) {
                if s.contains("\"sourceLanguage\"") && s.contains("\"strings\"") {
                    return Confidence::High;
                }
            }
        }
        Confidence::None
    }

    fn parse(&self, _content: &[u8]) -> Result<I18nResource, ParseError> {
        todo!("xcstrings parser")
    }

    fn capabilities(&self) -> FormatCapabilities {
        FormatCapabilities {
            plurals: true,
            arrays: false,
            comments: true,
            context: false,
            source_string: false,
            translatable_flag: true,
            translation_state: true,
            max_width: false,
            device_variants: true,
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
        todo!("xcstrings writer")
    }

    fn capabilities(&self) -> FormatCapabilities {
        Parser.capabilities()
    }
}
