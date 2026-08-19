use crate::Parser;
use mellis_ast::{Expr, ExprId};
use mellis_lexer::TokenKind;

impl<'a> Parser<'a> {
    pub fn parse_expr(&mut self) -> Result<ExprId, ()> {
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<ExprId, ()> {
        if self.match_token(TokenKind::IntegerLiteral)
            || self.match_token(TokenKind::FloatLiteral)
            || self.match_token(TokenKind::StringLiteral)
            || self.match_token(TokenKind::CharLiteral)
            || self.match_token(TokenKind::KwTrue)
            || self.match_token(TokenKind::KwFalse)
        {
            let token = self.previous();
            let expr = Expr::Literal(token);
            Ok(self.arena.alloc_expr(expr))
        } else {
            let span = self.peek().span;
            self.error_at_current("Expected expression.", span);
            Err(())
        }
    }
}
