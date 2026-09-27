//! Parser: tokens → abstract syntax tree.
//!
//! Spec: `docs/specs/parser.md` (the grammar is there too). Tests:
//! `compiler/tests/parser.rs` and `parser_depth.rs`. Tree types: `ast.rs`.
//!
//! Recursive descent, one method per grammar rule. Precedence falls out of
//! the call order: looser rules call tighter ones (`or` → `and` →
//! `comparison` → `additive` → `term` → `unary` → `primary`).
//!
//! Errors never stop the parse. A method that hits one reports it and
//! returns `None`; the `?`s carry that up to the statement, which skips to a
//! likely statement boundary ("panic mode") and carries on, so one mistake
//! gives one error. The missing `;` is special: at the end of a line it is
//! reported and then assumed, since skipping would throw away the next line.

use crate::ast::*;
use crate::diagnostic::Diagnostic;
use crate::lexer::{Token, TokenKind as K, lex};
use crate::span::Span;

/// Deeper than this is reported instead of recursed into: the browser gives
/// the compiler a small stack, and overflowing it crashes rather than erring.
const MAX_DEPTH: u32 = 200;

/// Lexes and parses `source`.
///
/// Returns every diagnostic from both the lexer and the parser. Always
/// returns a `Program`, even with errors (containing whatever parsed
/// successfully). Never panics, whatever the input.
pub fn parse(source: &str) -> (Program, Vec<Diagnostic>) {
    let (tokens, diagnostics) = lex(source);
    let mut p = Parser {
        source,
        tokens,
        pos: 0,
        diagnostics,
        depth: 0,
        too_deep: false,
    };
    let functions = p.program();
    (Program { functions }, p.diagnostics)
}

struct Parser<'a> {
    source: &'a str,
    /// Always ends with `Eof`, and `advance` never moves past it, so the
    /// current token always exists.
    tokens: Vec<Token>,
    pos: usize,
    diagnostics: Vec<Diagnostic>,
    depth: u32,
    /// Set once the depth limit is hit: the rest of the file is abandoned.
    too_deep: bool,
}

impl Parser<'_> {
    // ------------------------------------------------------------ helpers

    fn peek(&self) -> Token {
        self.tokens[self.pos]
    }

    fn peek_kind(&self) -> K {
        self.peek().kind
    }

    fn peek_ahead(&self, n: usize) -> K {
        self.tokens[(self.pos + n).min(self.tokens.len() - 1)].kind
    }

    fn advance(&mut self) -> Token {
        let token = self.peek();
        if token.kind != K::Eof {
            self.pos += 1;
        }
        token
    }

    fn at(&self, kind: K) -> bool {
        self.peek_kind() == kind
    }

    /// Consumes the current token if it is `kind`.
    fn eat(&mut self, kind: K) -> bool {
        let yes = self.at(kind);
        if yes {
            self.advance();
        }
        yes
    }

    /// End of the previous token: where a missing `;` belongs.
    fn prev_end(&self) -> u32 {
        self.pos
            .checked_sub(1)
            .map_or(0, |i| self.tokens[i].span.end)
    }

    fn error(&mut self, message: impl Into<String>, span: Span) {
        self.diagnostics.push(Diagnostic::error(message, span));
    }

    /// Consumes a `kind` token, or reports `message` at the current token.
    fn expect(&mut self, kind: K, message: &str) -> Option<Token> {
        if self.at(kind) {
            Some(self.advance())
        } else {
            let span = self.peek().span;
            self.error(message, span);
            None
        }
    }

    fn ident(&mut self, message: &str) -> Option<Ident> {
        let token = self.expect(K::Ident, message)?;
        Some(Ident {
            name: token.span.text(self.source).to_string(),
            span: token.span,
        })
    }

    fn type_expr(&mut self) -> Option<TypeExpr> {
        let name = self.ident("expected a type here, like `int` or `bool`")?;
        Some(TypeExpr { name })
    }

    /// Enters one level of nesting. `false` means the limit was hit (and
    /// reported once); the caller must return `None` without calling `leave`.
    fn enter(&mut self) -> bool {
        if self.too_deep {
            return false;
        }
        if self.depth >= MAX_DEPTH {
            let span = self.peek().span;
            self.diagnostics.push(
                Diagnostic::error("this is nested too deeply for Stepwise to follow", span)
                    .with_note("split it into smaller pieces, for example with a helper function"),
            );
            self.too_deep = true;
            self.pos = self.tokens.len() - 1; // abandon the rest: skip to Eof
            return false;
        }
        self.depth += 1;
        true
    }

    fn leave(&mut self) {
        self.depth -= 1;
    }

    // ----------------------------------------------------------- recovery

    /// At the top level, skips to the next `fn`.
    fn skip_to_function(&mut self) {
        while !matches!(self.peek_kind(), K::Fn | K::Eof) {
            self.advance();
        }
    }

    /// Inside a block, skips to a likely statement boundary: just past a
    /// `;`, or just before a `}` or a keyword that starts a statement.
    fn skip_to_statement(&mut self) {
        loop {
            match self.peek_kind() {
                K::Semicolon => {
                    self.advance();
                    return;
                }
                K::RBrace | K::Eof | K::Let | K::If | K::While | K::Return | K::Fn => return,
                _ => {
                    self.advance();
                }
            }
        }
    }

    // ------------------------------------------------------ program level

    fn program(&mut self) -> Vec<Function> {
        let mut functions = Vec::new();
        while !self.at(K::Eof) {
            if self.at(K::Fn) {
                match self.function() {
                    Some(f) => functions.push(f),
                    None => self.skip_to_function(),
                }
            } else {
                let span = self.peek().span;
                self.diagnostics.push(
                    Diagnostic::error("expected `fn` here", span)
                        .with_note("all code must be inside a function, like `fn main() { ... }`"),
                );
                self.advance();
                self.skip_to_function();
            }
        }
        functions
    }

    /// `fn name(params) -> type { ... }`
    fn function(&mut self) -> Option<Function> {
        let start = self.advance().span; // `fn`
        let name = self.ident("expected a function name after `fn`")?;
        self.expect(K::LParen, "expected `(` after the function name")?;
        let mut params = Vec::new();
        if !self.at(K::RParen) {
            loop {
                let name = self.ident("expected a parameter name")?;
                self.expect(
                    K::Colon,
                    "expected `:` and a type after the parameter name, like `n: int`",
                )?;
                let ty = self.type_expr()?;
                params.push(Param { name, ty });
                if !self.eat(K::Comma) {
                    break;
                }
            }
        }
        self.expect(K::RParen, "expected `)` after the parameters")?;
        let return_type = if self.eat(K::Arrow) {
            Some(self.type_expr()?)
        } else {
            None
        };
        let body = self.block()?;
        Some(Function {
            name,
            params,
            return_type,
            span: start.to(body.span),
            body,
        })
    }

    // -------------------------------------------------------- statements

    /// `{ statements }`
    fn block(&mut self) -> Option<Block> {
        if !self.enter() {
            return None;
        }
        let block = self.block_body();
        self.leave();
        block
    }

    fn block_body(&mut self) -> Option<Block> {
        let open = self
            .expect(K::LBrace, "expected `{` to start a block")?
            .span;
        let mut stmts = Vec::new();
        loop {
            match self.peek_kind() {
                K::RBrace => {
                    let close = self.advance().span;
                    return Some(Block {
                        stmts,
                        span: open.to(close),
                    });
                }
                K::Eof => {
                    if !self.too_deep {
                        let span = self.peek().span;
                        self.diagnostics.push(
                            Diagnostic::error("expected `}` before the end of the file", span)
                                .with_label(open, "this `{` is never closed"),
                        );
                    }
                    return None;
                }
                _ => {
                    let before = self.pos;
                    match self.statement() {
                        Some(stmt) => stmts.push(stmt),
                        None if self.too_deep => return None,
                        None => {
                            self.skip_to_statement();
                            // Always make progress, or a token that no
                            // statement can start (like a stray `fn`) would
                            // loop forever.
                            if self.pos == before {
                                self.advance();
                            }
                        }
                    }
                }
            }
        }
    }

    fn statement(&mut self) -> Option<Stmt> {
        let start = self.peek().span;
        let (kind, end) = match self.peek_kind() {
            K::Let => self.let_statement()?,
            K::If => return self.if_statement(),
            K::While => {
                self.advance();
                let cond = self.expression()?;
                let body = self.block()?;
                let end = body.span;
                (StmtKind::While { cond, body }, end)
            }
            K::Return => {
                self.advance();
                let value = if self.at(K::Semicolon) {
                    None
                } else {
                    Some(self.expression()?)
                };
                let end = self.semicolon()?;
                (StmtKind::Return(value), end)
            }
            // One token of lookahead tells an assignment from an expression.
            K::Ident if self.peek_ahead(1) == K::Eq => {
                let target = self.ident("expected a variable name")?;
                self.advance(); // `=`
                let value = self.expression()?;
                let end = self.semicolon()?;
                (StmtKind::Assign { target, value }, end)
            }
            _ => {
                let expr = self.expression()?;
                let end = self.semicolon()?;
                (StmtKind::Expr(expr), end)
            }
        };
        Some(Stmt {
            kind,
            span: start.to(end),
        })
    }

    /// `let mut name: type = value;`
    fn let_statement(&mut self) -> Option<(StmtKind, Span)> {
        self.advance(); // `let`
        let mutable = self.eat(K::Mut);
        let name = self.ident("expected a variable name after `let`")?;
        let ty = if self.eat(K::Colon) {
            Some(self.type_expr()?)
        } else {
            None
        };
        self.expect(K::Eq, "expected `=` and a value after the variable name")?;
        let value = self.expression()?;
        let end = self.semicolon()?;
        Some((
            StmtKind::Let {
                mutable,
                name,
                ty,
                value,
            },
            end,
        ))
    }

    /// `if cond { ... } else if ... else { ... }`. An `else if` becomes an
    /// else block holding just the inner `if`.
    fn if_statement(&mut self) -> Option<Stmt> {
        let start = self.advance().span; // `if`
        let cond = self.expression()?;
        let then_block = self.block()?;
        let mut end = then_block.span;
        let else_block = if self.eat(K::Else) {
            let block = if self.at(K::If) {
                let inner = self.if_statement()?;
                Block {
                    span: inner.span,
                    stmts: vec![inner],
                }
            } else {
                self.block()?
            };
            end = block.span;
            Some(block)
        } else {
            None
        };
        Some(Stmt {
            kind: StmtKind::If {
                cond,
                then_block,
                else_block,
            },
            span: start.to(end),
        })
    }

    /// The `;` that ends a statement. If it's missing, the error points just
    /// after the previous token, where it belongs. At the end of a line (or
    /// before a `}`) the `;` is then assumed, so the next line still parses
    /// and one missing `;` is one error.
    fn semicolon(&mut self) -> Option<Span> {
        if self.at(K::Semicolon) {
            return Some(self.advance().span);
        }
        let end = self.prev_end();
        let here = Span::new(end, end);
        self.diagnostics.push(
            Diagnostic::error("expected `;` at the end of this statement", here)
                .with_note("every statement ends with a semicolon"),
        );
        let next = self.peek();
        let gap = self
            .source
            .get(end as usize..next.span.start as usize)
            .unwrap_or("");
        (gap.contains('\n') || next.kind == K::RBrace).then_some(here)
    }

    // ------------------------------------------------------- expressions

    fn expression(&mut self) -> Option<Expr> {
        if !self.enter() {
            return None;
        }
        let expr = self.or();
        self.leave();
        expr
    }

    fn binary(op: BinaryOp, lhs: Expr, rhs: Expr) -> Expr {
        let span = lhs.span.to(rhs.span);
        Expr {
            kind: ExprKind::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            },
            span,
        }
    }

    /// `a || b || ...`
    fn or(&mut self) -> Option<Expr> {
        let mut lhs = self.and()?;
        while self.eat(K::OrOr) {
            let rhs = self.and()?;
            lhs = Self::binary(BinaryOp::Or, lhs, rhs);
        }
        Some(lhs)
    }

    /// `a && b && ...`
    fn and(&mut self) -> Option<Expr> {
        let mut lhs = self.comparison()?;
        while self.eat(K::AndAnd) {
            let rhs = self.comparison()?;
            lhs = Self::binary(BinaryOp::And, lhs, rhs);
        }
        Some(lhs)
    }

    fn comparison_op(&self) -> Option<BinaryOp> {
        Some(match self.peek_kind() {
            K::EqEq => BinaryOp::Eq,
            K::BangEq => BinaryOp::Ne,
            K::Lt => BinaryOp::Lt,
            K::LtEq => BinaryOp::Le,
            K::Gt => BinaryOp::Gt,
            K::GtEq => BinaryOp::Ge,
            _ => return None,
        })
    }

    /// `a < b`, at most once: `1 < x < 5` is an error, because in most
    /// languages it compiles and silently means something else. The rest of
    /// the chain is still parsed, so it doesn't cause more errors.
    fn comparison(&mut self) -> Option<Expr> {
        let lhs = self.additive()?;
        let Some(op) = self.comparison_op() else {
            return Some(lhs);
        };
        self.advance();
        let rhs = self.additive()?;
        let mut expr = Self::binary(op, lhs, rhs);
        while let Some(op) = self.comparison_op() {
            let token = self.advance();
            self.diagnostics.push(
                Diagnostic::error("comparisons can't be chained", token.span)
                    .with_note("to check both, write them separately: `a < b && b < c`"),
            );
            let rhs = self.additive()?;
            expr = Self::binary(op, expr, rhs);
        }
        Some(expr)
    }

    /// `a + b - c ...`
    fn additive(&mut self) -> Option<Expr> {
        let mut lhs = self.term()?;
        loop {
            let op = match self.peek_kind() {
                K::Plus => BinaryOp::Add,
                K::Minus => BinaryOp::Sub,
                _ => return Some(lhs),
            };
            self.advance();
            let rhs = self.term()?;
            lhs = Self::binary(op, lhs, rhs);
        }
    }

    /// `a * b / c % d ...`
    fn term(&mut self) -> Option<Expr> {
        let mut lhs = self.unary()?;
        loop {
            let op = match self.peek_kind() {
                K::Star => BinaryOp::Mul,
                K::Slash => BinaryOp::Div,
                K::Percent => BinaryOp::Rem,
                _ => return Some(lhs),
            };
            self.advance();
            let rhs = self.unary()?;
            lhs = Self::binary(op, lhs, rhs);
        }
    }

    /// `-e` and `!e`. A `-` directly before a number literal is folded into
    /// it, which is the only way to write -2147483648.
    fn unary(&mut self) -> Option<Expr> {
        let op = match self.peek_kind() {
            K::Minus => UnaryOp::Neg,
            K::Bang => UnaryOp::Not,
            _ => return self.primary(),
        };
        let start = self.advance().span;
        if op == UnaryOp::Neg
            && let K::Int(n) = self.peek_kind()
        {
            let literal = self.advance();
            return Some(Expr {
                kind: ExprKind::Int((-i64::from(n)) as i32),
                span: start.to(literal.span),
            });
        }
        if !self.enter() {
            return None;
        }
        let operand = self.unary();
        self.leave();
        let operand = operand?;
        let span = start.to(operand.span);
        Some(Expr {
            kind: ExprKind::Unary {
                op,
                operand: Box::new(operand),
            },
            span,
        })
    }

    /// Numbers, `true`/`false`, variables, calls, and `( ... )`.
    fn primary(&mut self) -> Option<Expr> {
        let token = self.peek();
        let kind = match token.kind {
            K::Int(n) => {
                self.advance();
                match i32::try_from(n) {
                    Ok(n) => ExprKind::Int(n),
                    Err(_) => {
                        // 2147483648 is only allowed right after a `-`.
                        self.diagnostics.push(
                            Diagnostic::error("this number is too large for an int", token.span)
                                .with_note("the largest int is 2147483647"),
                        );
                        ExprKind::Int(0)
                    }
                }
            }
            K::True => {
                self.advance();
                ExprKind::Bool(true)
            }
            K::False => {
                self.advance();
                ExprKind::Bool(false)
            }
            K::Ident => return self.name_or_call(),
            K::LParen => {
                let open = self.advance().span;
                let inner = self.expression()?;
                let close = self
                    .expect(K::RParen, "expected `)` to close the `(`")?
                    .span;
                // Keep the parentheses in the span, so `a * (b + c)` covers the `)`.
                return Some(Expr {
                    kind: inner.kind,
                    span: open.to(close),
                });
            }
            _ => {
                self.error(
                    "expected a value here, like a number, a variable, or a function call",
                    token.span,
                );
                return None;
            }
        };
        Some(Expr {
            kind,
            span: token.span,
        })
    }

    /// `name` or `name(args)`.
    fn name_or_call(&mut self) -> Option<Expr> {
        let name = self.ident("expected a name")?;
        if !self.eat(K::LParen) {
            return Some(Expr {
                span: name.span,
                kind: ExprKind::Var(name),
            });
        }
        let mut args = Vec::new();
        if !self.at(K::RParen) {
            loop {
                args.push(self.expression()?);
                if !self.eat(K::Comma) {
                    break;
                }
            }
        }
        let close = self
            .expect(K::RParen, "expected `)` to close the function call")?
            .span;
        let span = name.span.to(close);
        Some(Expr {
            kind: ExprKind::Call { callee: name, args },
            span,
        })
    }
}
