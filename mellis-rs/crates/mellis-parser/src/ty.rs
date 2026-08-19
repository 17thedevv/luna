use crate::Parser;
use mellis_ast::{Type, TypeId};

impl<'a> Parser<'a> {
    pub fn parse_type(&mut self) -> Result<TypeId, ()> {
        let span = self.peek().span;
        self.error_at_current("Parsing types is not yet implemented", span);
        Err(())
    }
}
