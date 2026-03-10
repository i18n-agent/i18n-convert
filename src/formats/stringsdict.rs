use crate::ir::*;
use super::*;

pub struct Parser;
pub struct Writer;

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        if extension == ".stringsdict" {
            return Confidence::Definite;
        }
        if extension == ".plist" || extension == ".xml" {
            if let Ok(s) = std::str::from_utf8(content) {
                if s.contains("NSStringLocalizedFormatKey") {
                    return Confidence::Definite;
                }
            }
        }
        Confidence::None
    }

    fn parse(&self, _content: &[u8]) -> Result<I18nResource, ParseError> {
        todo!("stringsdict parser")
    }

    fn capabilities(&self) -> FormatCapabilities {
        FormatCapabilities {
            plurals: true,
            arrays: false,
            comments: false,
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
        todo!("stringsdict writer")
    }

    fn capabilities(&self) -> FormatCapabilities {
        Parser.capabilities()
    }
}
