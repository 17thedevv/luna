use crate::Parser;
use mellis_ast::{Decl, DeclId};

impl<'a> Parser<'a> {
    pub fn parse_decl(&mut self) -> Result<DeclId, ()> {
        let span = self.peek().span;
        self.error_at_current("Parsing declarations is not yet implemented", span);
        Err(())
    }
}
