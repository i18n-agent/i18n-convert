use crate::ir::*;
use super::*;

pub struct Parser;
pub struct Writer;

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        if extension == ".yml" || extension == ".yaml" {
            if let Ok(s) = std::str::from_utf8(content) {
                // Rails convention: top-level key is a locale code like "en:", "ja:", "de:", etc.
                let trimmed = s.trim_start();
                if trimmed.starts_with("en:")
                    || trimmed.starts_with("ja:")
                    || trimmed.starts_with("de:")
                    || trimmed.starts_with("fr:")
                    || trimmed.starts_with("es:")
                    || trimmed.starts_with("zh:")
                    || trimmed.starts_with("ko:")
                    || trimmed.starts_with("pt:")
                    || trimmed.starts_with("it:")
                    || trimmed.starts_with("ru:")
                {
                    return Confidence::High;
                }
            }
            return Confidence::Low;
        }
        Confidence::None
    }

    fn parse(&self, _content: &[u8]) -> Result<I18nResource, ParseError> {
        todo!("yaml_rails parser")
    }

    fn capabilities(&self) -> FormatCapabilities {
        FormatCapabilities {
            plurals: true,
            arrays: true,
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
        todo!("yaml_rails writer")
    }

    fn capabilities(&self) -> FormatCapabilities {
        Parser.capabilities()
    }
}
