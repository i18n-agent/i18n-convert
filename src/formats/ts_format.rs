use crate::ir::*;
use super::*;
use indexmap::IndexMap;

pub struct Parser;
pub struct Writer;

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        if extension != ".ts" {
            return Confidence::None;
        }
        // .ts is shared with Qt Linguist (XML). Check if content looks like JS/TS, not XML.
        let text = String::from_utf8_lossy(content);
        if text.contains("export default") || text.contains("export const") {
            Confidence::High
        } else if text.trim_start().starts_with('<') {
            // Looks like XML (Qt Linguist), not TypeScript
            Confidence::None
        } else {
            Confidence::Low
        }
    }

    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError> {
        let _ = content;
        Ok(I18nResource {
            metadata: ResourceMetadata {
                source_format: FormatId::TypeScript,
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
