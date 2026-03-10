use crate::ir::*;
use super::*;
use indexmap::IndexMap;

pub struct Parser;
pub struct Writer;

impl FormatParser for Parser {
    fn detect(&self, extension: &str, _content: &[u8]) -> Confidence {
        if extension == ".xliff" || extension == ".xlf" {
            Confidence::Low
        } else {
            Confidence::None
        }
    }

    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError> {
        let _ = content;
        Ok(I18nResource {
            metadata: ResourceMetadata {
                source_format: FormatId::IspringXliff,
                ..Default::default()
            },
            entries: IndexMap::new(),
        })
    }

    fn capabilities(&self) -> FormatCapabilities {
        FormatCapabilities {
            source_string: true,
            translation_state: true,
            context: true,
            ..Default::default()
        }
    }
}

impl FormatWriter for Writer {
    fn write(&self, resource: &I18nResource) -> Result<Vec<u8>, WriteError> {
        let _ = resource;
        Ok(Vec::new())
    }

    fn capabilities(&self) -> FormatCapabilities {
        FormatCapabilities {
            source_string: true,
            translation_state: true,
            context: true,
            ..Default::default()
        }
    }
}
