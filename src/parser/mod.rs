pub mod map;
pub mod scanner;

#[derive(Clone, Debug)]
pub struct ParseError {
    pub file_name: String,
    pub description: String,
    pub line: u32,
}

impl ParseError {
    pub fn new(file_name: String, description: String, line: u32) -> Self {
        Self {
            file_name,
            description,
            line,
        }
    }
}
