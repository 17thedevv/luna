use crate::Parser;
use mellis_ast::{Stmt, StmtId};

impl<'a> Parser<'a> {
    pub fn parse_stmt(&mut self) -> Result<StmtId, ()> {
        let span = self.peek().span;
        self.error_at_current("Parsing statements is not yet implemented", span);
        Err(())
    }
}
