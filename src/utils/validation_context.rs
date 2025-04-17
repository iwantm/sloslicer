use super::errors::ParserError;

pub struct ValidationContext {
    errors: Vec<ParserError>,
}

impl ValidationContext {
    pub fn new() -> Self {
        Self { errors: vec![] }
    }

    pub fn push(&mut self, error: ParserError) {
        self.errors.push(error);
    }

    pub fn result(self) -> Result<(), Vec<ParserError>> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors)
        }
    }
}
