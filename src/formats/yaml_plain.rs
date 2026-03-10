use crate::ir::*;
use super::*;
use indexmap::IndexMap;

pub struct Parser;
pub struct Writer;

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        if extension != ".yml" && extension != ".yaml" {
            return Confidence::None;
        }
        // Distinguish from Rails YAML: Rails has locale root key like "en:", "ja:", etc.
        let text = String::from_utf8_lossy(content);
        let first_line = text.lines().find(|l| !l.trim().is_empty() && !l.starts_with('#'));
        if let Some(line) = first_line {
            // If first meaningful line looks like a 2-3 char locale key, it's probably Rails
            let trimmed = line.trim();
            if trimmed.len() <= 8 && trimmed.ends_with(':') {
                return Confidence::Low;
            }
        }
        Confidence::High
    }

    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError> {
        let _ = content;
        Ok(I18nResource {
            metadata: ResourceMetadata {
                source_format: FormatId::YamlPlain,
                ..Default::default()
            },
            entries: IndexMap::new(),
        })
    }

    fn capabilities(&self) -> FormatCapabilities {
        FormatCapabilities {
            comments: true,
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
            comments: true,
            ..Default::default()
        }
    }
}
