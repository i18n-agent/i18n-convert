use crate::ir::*;
use super::*;

pub struct Parser;
pub struct Writer;

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

    fn parse(&self, _content: &[u8]) -> Result<I18nResource, ParseError> {
        todo!("xliff1 parser")
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

impl FormatWriter for Writer {
    fn write(&self, _resource: &I18nResource) -> Result<Vec<u8>, WriteError> {
        todo!("xliff1 writer")
    }

    fn capabilities(&self) -> FormatCapabilities {
        Parser.capabilities()
    }
}
