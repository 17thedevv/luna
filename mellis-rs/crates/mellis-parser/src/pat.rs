use crate::Parser;
use mellis_ast::{PatId, Pattern};

impl<'a> Parser<'a> {
    pub fn parse_pattern(&mut self) -> Result<PatId, ()> {
        let span = self.peek().span;
        self.error_at_current("Parsing patterns is not yet implemented", span);
        Err(())
    }
}
