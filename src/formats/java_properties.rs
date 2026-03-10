use super::*;

pub struct Parser;
pub struct Writer;

impl FormatParser for Parser {
    fn detect(&self, _extension: &str, _content: &[u8]) -> Confidence {
        Confidence::None
    }

    fn parse(&self, _content: &[u8]) -> Result<I18nResource, ParseError> {
        Err(ParseError::InvalidFormat("not yet implemented".into()))
    }

    fn capabilities(&self) -> FormatCapabilities {
        FormatCapabilities::default()
    }
}

impl FormatWriter for Writer {
    fn write(&self, _resource: &I18nResource) -> Result<Vec<u8>, WriteError> {
        Err(WriteError::Serialization("not yet implemented".into()))
    }

    fn capabilities(&self) -> FormatCapabilities {
        FormatCapabilities::default()
    }
}
