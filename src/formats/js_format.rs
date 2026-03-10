use crate::ir::*;
use super::*;
use indexmap::IndexMap;

pub struct Parser;
pub struct Writer;

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        if extension != ".js" {
            return Confidence::None;
        }
        let text = String::from_utf8_lossy(content);
        if text.contains("module.exports") || text.contains("export default") || text.contains("exports.") {
            Confidence::High
        } else {
            Confidence::Low
        }
    }

    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError> {
        let _ = content;
        Ok(I18nResource {
            metadata: ResourceMetadata {
                source_format: FormatId::JavaScript,
                ..Default::default()
            },
            entries: IndexMap::new(),
        })
    }

    fn capabilities(&self) -> FormatCapabilities {
        FormatCapabilities {
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
            ..Default::default()
        }
    }
}
