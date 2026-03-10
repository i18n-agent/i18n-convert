use crate::ir::*;
use super::*;

pub struct Parser;
pub struct Writer;

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        if extension == ".json" {
            if let Ok(s) = std::str::from_utf8(content) {
                // i18next uses _one, _other, _zero, _few, _many, _two suffixes
                if s.contains("_one\"") && s.contains("_other\"") {
                    return Confidence::High;
                }
                if s.contains("_other\"") {
                    return Confidence::Low;
                }
            }
        }
        Confidence::None
    }

    fn parse(&self, _content: &[u8]) -> Result<I18nResource, ParseError> {
        todo!("i18next parser")
    }

    fn capabilities(&self) -> FormatCapabilities {
        FormatCapabilities {
            plurals: true,
            arrays: false,
            comments: false,
            context: true,
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
        todo!("i18next writer")
    }

    fn capabilities(&self) -> FormatCapabilities {
        Parser.capabilities()
    }
}
