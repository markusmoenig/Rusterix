use crate::parser::{
    scanner::{Scanner, Token},
    ParseError,
};
use rusterix::Map;

pub struct MapParser {
    scanner: Scanner,

    current: Token,
    previous: Token,

    error: Option<ParseError>,
}

impl Default for MapParser {
    fn default() -> Self {
        MapParser::new()
    }
}

impl MapParser {
    pub fn new() -> Self {
        Self {
            scanner: Scanner::new("".to_string()),
            current: Token::synthetic("".to_owned()),
            previous: Token::synthetic("".to_owned()),
            error: None,
        }
    }

    /// Parse the source and return a valid map.
    pub fn parse(&mut self, source: String) -> Result<Map, ParseError> {
        self.error = None;
        self.scanner = Scanner::new(source);

        let mut map = Map::default();

        Ok(map)
    }
}
