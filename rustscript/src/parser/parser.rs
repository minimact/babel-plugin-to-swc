//! Recursive descent parser for RustScript

use crate::lexer::{Token, TokenKind, Span};
use crate::parser::ast::*;

/// Parser for RustScript
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

/// Parse error
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl ParseError {
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}

pub type ParseResult<T> = Result<T, ParseError>;

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    /// Parse a complete program
    pub fn parse(&mut self) -> ParseResult<Program> {
        self.skip_newlines();
        let start_span = self.current_span();

        let decl = if self.check(TokenKind::Plugin) {
            TopLevelDecl::Plugin(self.parse_plugin()?)
        } else if self.check(TokenKind::Writer) {
            TopLevelDecl::Writer(self.parse_writer()?)
        } else {
            return Err(self.error("Expected 'plugin' or 'writer' declaration"));
        };

        Ok(Program {
            decl,
            span: start_span,
        })
    }

    /// Parse plugin declaration
    fn parse_plugin(&mut self) -> ParseResult<PluginDecl> {
        let start_span = self.current_span();
        self.expect(TokenKind::Plugin)?;
        let name = self.expect_ident()?;
        self.expect(TokenKind::LBrace)?;

        let body = self.parse_plugin_body()?;

        self.expect(TokenKind::RBrace)?;

        Ok(PluginDecl {
            name,
            body,
            span: start_span,
        })
    }

    /// Parse writer declaration
    fn parse_writer(&mut self) -> ParseResult<WriterDecl> {
        let start_span = self.current_span();
        self.expect(TokenKind::Writer)?;
        let name = self.expect_ident()?;
        self.expect(TokenKind::LBrace)?;

        let body = self.parse_plugin_body()?;

        self.expect(TokenKind::RBrace)?;

        Ok(WriterDecl {
            name,
            body,
            span: start_span,
        })
    }

    /// Parse plugin/writer body items
    fn parse_plugin_body(&mut self) -> ParseResult<Vec<PluginItem>> {
        let mut items = Vec::new();

        loop {
            self.skip_newlines();

            if self.check(TokenKind::RBrace) || self.is_at_end() {
                break;
            }

            let item = if self.check(TokenKind::Struct) {
                PluginItem::Struct(self.parse_struct()?)
            } else if self.check(TokenKind::Enum) {
                PluginItem::Enum(self.parse_enum()?)
            } else if self.check(TokenKind::Fn) || self.check(TokenKind::Pub) {
                PluginItem::Function(self.parse_function()?)
            } else if self.check(TokenKind::Impl) {
                PluginItem::Impl(self.parse_impl()?)
            } else {
                return Err(self.error("Expected struct, enum, fn, or impl"));
            };

            items.push(item);
        }

        Ok(items)
    }

    /// Parse struct declaration
    fn parse_struct(&mut self) -> ParseResult<StructDecl> {
        let start_span = self.current_span();
        self.expect(TokenKind::Struct)?;
        let name = self.expect_ident()?;
        self.expect(TokenKind::LBrace)?;

        let mut fields = Vec::new();
        loop {
            self.skip_newlines();
            if self.check(TokenKind::RBrace) {
                break;
            }

            let field_span = self.current_span();
            let field_name = self.expect_ident()?;
            self.expect(TokenKind::Colon)?;
            let ty = self.parse_type()?;

            fields.push(StructField {
                name: field_name,
                ty,
                span: field_span,
            });

            self.skip_newlines();
            if !self.check(TokenKind::RBrace) {
                self.expect(TokenKind::Comma)?;
            }
        }

        self.expect(TokenKind::RBrace)?;

        Ok(StructDecl {
            name,
            fields,
            span: start_span,
        })
    }

    /// Parse enum declaration
    fn parse_enum(&mut self) -> ParseResult<EnumDecl> {
        let start_span = self.current_span();
        self.expect(TokenKind::Enum)?;
        let name = self.expect_ident()?;
        self.expect(TokenKind::LBrace)?;

        let mut variants = Vec::new();
        loop {
            self.skip_newlines();
            if self.check(TokenKind::RBrace) {
                break;
            }

            let variant_span = self.current_span();
            let variant_name = self.expect_ident()?;

            let fields = if self.check(TokenKind::LParen) {
                self.advance();
                let mut types = Vec::new();
                if !self.check(TokenKind::RParen) {
                    types.push(self.parse_type()?);
                    while self.match_token(TokenKind::Comma) {
                        types.push(self.parse_type()?);
                    }
                }
                self.expect(TokenKind::RParen)?;
                Some(types)
            } else {
                None
            };

            variants.push(EnumVariant {
                name: variant_name,
                fields,
                span: variant_span,
            });

            self.skip_newlines();
            if !self.check(TokenKind::RBrace) {
                self.expect(TokenKind::Comma)?;
            }
        }

        self.expect(TokenKind::RBrace)?;

        Ok(EnumDecl {
            name,
            variants,
            span: start_span,
        })
    }

    /// Parse function declaration
    fn parse_function(&mut self) -> ParseResult<FnDecl> {
        let start_span = self.current_span();
        let is_pub = self.match_token(TokenKind::Pub);
        self.expect(TokenKind::Fn)?;
        let name = self.expect_ident()?;

        self.expect(TokenKind::LParen)?;
        let params = self.parse_params()?;
        self.expect(TokenKind::RParen)?;

        let return_type = if self.match_token(TokenKind::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };

        let body = self.parse_block()?;

        Ok(FnDecl {
            is_pub,
            name,
            params,
            return_type,
            body,
            span: start_span,
        })
    }

    /// Parse function parameters
    fn parse_params(&mut self) -> ParseResult<Vec<Param>> {
        let mut params = Vec::new();

        if self.check(TokenKind::RParen) {
            return Ok(params);
        }

        loop {
            let param_span = self.current_span();
            let name = self.expect_ident()?;
            self.expect(TokenKind::Colon)?;
            let ty = self.parse_type()?;

            params.push(Param {
                name,
                ty,
                span: param_span,
            });

            if !self.match_token(TokenKind::Comma) {
                break;
            }
        }

        Ok(params)
    }

    /// Parse impl block
    fn parse_impl(&mut self) -> ParseResult<ImplBlock> {
        let start_span = self.current_span();
        self.expect(TokenKind::Impl)?;
        let target = self.expect_ident()?;
        self.expect(TokenKind::LBrace)?;

        let mut items = Vec::new();
        loop {
            self.skip_newlines();
            if self.check(TokenKind::RBrace) {
                break;
            }
            items.push(self.parse_function()?);
        }

        self.expect(TokenKind::RBrace)?;

        Ok(ImplBlock {
            target,
            items,
            span: start_span,
        })
    }

    /// Parse a type
    fn parse_type(&mut self) -> ParseResult<Type> {
        // Check for reference
        if self.match_token(TokenKind::Ampersand) {
            let mutable = self.match_token(TokenKind::Mut);
            let inner = self.parse_type()?;
            return Ok(Type::Reference {
                mutable,
                inner: Box::new(inner),
            });
        }

        // Get the type name
        let name = self.expect_type_name()?;

        // Check for type arguments
        if self.match_token(TokenKind::Lt) {
            let mut type_args = Vec::new();
            type_args.push(self.parse_type()?);
            while self.match_token(TokenKind::Comma) {
                type_args.push(self.parse_type()?);
            }
            self.expect(TokenKind::Gt)?;

            Ok(Type::Container { name, type_args })
        } else {
            // Determine if it's a primitive or named type
            match name.as_str() {
                "Str" | "i32" | "u32" | "f64" | "bool" => Ok(Type::Primitive(name)),
                _ => Ok(Type::Named(name)),
            }
        }
    }

    /// Parse a block
    fn parse_block(&mut self) -> ParseResult<Block> {
        let start_span = self.current_span();
        self.expect(TokenKind::LBrace)?;

        let mut stmts = Vec::new();
        loop {
            self.skip_newlines();
            if self.check(TokenKind::RBrace) || self.is_at_end() {
                break;
            }
            stmts.push(self.parse_statement()?);
        }

        self.expect(TokenKind::RBrace)?;

        Ok(Block {
            stmts,
            span: start_span,
        })
    }

    /// Parse a statement
    fn parse_statement(&mut self) -> ParseResult<Stmt> {
        self.skip_newlines();

        if self.check(TokenKind::Let) {
            self.parse_let_stmt()
        } else if self.check(TokenKind::Const) {
            self.parse_const_stmt()
        } else if self.check(TokenKind::If) {
            self.parse_if_stmt()
        } else if self.check(TokenKind::Match) {
            self.parse_match_stmt()
        } else if self.check(TokenKind::For) {
            self.parse_for_stmt()
        } else if self.check(TokenKind::While) {
            self.parse_while_stmt()
        } else if self.check(TokenKind::Loop) {
            self.parse_loop_stmt()
        } else if self.check(TokenKind::Return) {
            self.parse_return_stmt()
        } else if self.check(TokenKind::Break) {
            self.parse_break_stmt()
        } else if self.check(TokenKind::Continue) {
            self.parse_continue_stmt()
        } else if self.check(TokenKind::Traverse) {
            self.parse_traverse_stmt()
        } else {
            self.parse_expr_stmt()
        }
    }

    /// Parse let statement
    fn parse_let_stmt(&mut self) -> ParseResult<Stmt> {
        let start_span = self.current_span();
        self.expect(TokenKind::Let)?;
        let mutable = self.match_token(TokenKind::Mut);
        let name = self.expect_ident()?;

        let ty = if self.match_token(TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };

        self.expect(TokenKind::Eq)?;
        let init = self.parse_expr()?;
        self.expect(TokenKind::Semicolon)?;

        Ok(Stmt::Let(LetStmt {
            mutable,
            name,
            ty,
            init,
            span: start_span,
        }))
    }

    /// Parse const statement
    fn parse_const_stmt(&mut self) -> ParseResult<Stmt> {
        let start_span = self.current_span();
        self.expect(TokenKind::Const)?;
        let name = self.expect_ident()?;

        let ty = if self.match_token(TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };

        self.expect(TokenKind::Eq)?;
        let init = self.parse_expr()?;
        self.expect(TokenKind::Semicolon)?;

        Ok(Stmt::Const(ConstStmt {
            name,
            ty,
            init,
            span: start_span,
        }))
    }

    /// Parse if statement
    fn parse_if_stmt(&mut self) -> ParseResult<Stmt> {
        let start_span = self.current_span();
        self.expect(TokenKind::If)?;

        // Check for if-let pattern: `if let Pattern = expr`
        let (pattern, condition) = if self.match_token(TokenKind::Let) {
            let pat = self.parse_pattern()?;
            self.expect(TokenKind::Eq)?;
            let expr = self.parse_expr()?;
            (Some(pat), expr)
        } else {
            (None, self.parse_expr()?)
        };

        let then_branch = self.parse_block()?;

        let mut else_if_branches = Vec::new();
        let mut else_branch = None;

        while self.match_token(TokenKind::Else) {
            if self.match_token(TokenKind::If) {
                // Note: else-if with let not supported yet, just regular condition
                let cond = self.parse_expr()?;
                let block = self.parse_block()?;
                else_if_branches.push((cond, block));
            } else {
                else_branch = Some(self.parse_block()?);
                break;
            }
        }

        Ok(Stmt::If(IfStmt {
            condition,
            pattern,
            then_branch,
            else_if_branches,
            else_branch,
            span: start_span,
        }))
    }

    /// Parse match statement
    fn parse_match_stmt(&mut self) -> ParseResult<Stmt> {
        let start_span = self.current_span();
        self.expect(TokenKind::Match)?;
        let scrutinee = self.parse_expr()?;
        self.expect(TokenKind::LBrace)?;

        let mut arms = Vec::new();
        loop {
            self.skip_newlines();
            if self.check(TokenKind::RBrace) {
                break;
            }
            arms.push(self.parse_match_arm()?);
        }

        self.expect(TokenKind::RBrace)?;

        Ok(Stmt::Match(MatchStmt {
            scrutinee,
            arms,
            span: start_span,
        }))
    }

    /// Parse match arm
    fn parse_match_arm(&mut self) -> ParseResult<MatchArm> {
        let start_span = self.current_span();
        let pattern = self.parse_pattern()?;
        self.expect(TokenKind::FatArrow)?;
        let body = self.parse_expr()?;

        // Optional comma
        self.match_token(TokenKind::Comma);
        self.skip_newlines();

        Ok(MatchArm {
            pattern,
            body,
            span: start_span,
        })
    }

    /// Parse pattern
    fn parse_pattern(&mut self) -> ParseResult<Pattern> {
        // Check for wildcard
        if self.check_ident("_") {
            self.advance();
            return Ok(Pattern::Wildcard);
        }

        // Check for literal
        if let Some(lit) = self.try_parse_literal() {
            return Ok(Pattern::Literal(lit));
        }

        // Identifier, struct pattern, or variant pattern
        let name = self.expect_ident()?;

        if self.check(TokenKind::LBrace) {
            // Struct pattern: Name { field: pattern, ... }
            self.advance();
            let mut fields = Vec::new();
            loop {
                self.skip_newlines();
                if self.check(TokenKind::RBrace) {
                    break;
                }
                let field_name = self.expect_ident()?;
                self.expect(TokenKind::Colon)?;
                let field_pattern = self.parse_pattern()?;
                fields.push((field_name, field_pattern));

                if !self.match_token(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RBrace)?;
            Ok(Pattern::Struct { name, fields })
        } else if self.check(TokenKind::LParen) {
            // Variant pattern: Some(x), Ok(value), Err(e)
            self.advance();
            let inner = if self.check(TokenKind::RParen) {
                None
            } else {
                Some(Box::new(self.parse_pattern()?))
            };
            self.expect(TokenKind::RParen)?;
            Ok(Pattern::Variant { name, inner })
        } else if self.match_token(TokenKind::Pipe) {
            // Or pattern
            let mut patterns = vec![Pattern::Ident(name)];
            loop {
                patterns.push(self.parse_pattern()?);
                if !self.match_token(TokenKind::Pipe) {
                    break;
                }
            }
            Ok(Pattern::Or(patterns))
        } else {
            // Check if this is a unit variant like None
            if name == "None" || name == "true" || name == "false" {
                Ok(Pattern::Variant { name, inner: None })
            } else {
                Ok(Pattern::Ident(name))
            }
        }
    }

    /// Parse for statement
    fn parse_for_stmt(&mut self) -> ParseResult<Stmt> {
        let start_span = self.current_span();
        self.expect(TokenKind::For)?;
        let var = self.expect_ident()?;
        self.expect(TokenKind::In)?;
        let iter = self.parse_expr()?;
        let body = self.parse_block()?;

        Ok(Stmt::For(ForStmt {
            var,
            iter,
            body,
            span: start_span,
        }))
    }

    /// Parse while statement
    fn parse_while_stmt(&mut self) -> ParseResult<Stmt> {
        let start_span = self.current_span();
        self.expect(TokenKind::While)?;
        let condition = self.parse_expr()?;
        let body = self.parse_block()?;

        Ok(Stmt::While(WhileStmt {
            condition,
            body,
            span: start_span,
        }))
    }

    /// Parse loop statement
    fn parse_loop_stmt(&mut self) -> ParseResult<Stmt> {
        let start_span = self.current_span();
        self.expect(TokenKind::Loop)?;
        let body = self.parse_block()?;

        Ok(Stmt::Loop(LoopStmt {
            body,
            span: start_span,
        }))
    }

    /// Parse return statement
    fn parse_return_stmt(&mut self) -> ParseResult<Stmt> {
        let start_span = self.current_span();
        self.expect(TokenKind::Return)?;

        let value = if !self.check(TokenKind::Semicolon) && !self.check(TokenKind::Newline) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        self.expect(TokenKind::Semicolon)?;

        Ok(Stmt::Return(ReturnStmt {
            value,
            span: start_span,
        }))
    }

    /// Parse break statement
    fn parse_break_stmt(&mut self) -> ParseResult<Stmt> {
        let start_span = self.current_span();
        self.expect(TokenKind::Break)?;
        self.expect(TokenKind::Semicolon)?;
        Ok(Stmt::Break(BreakStmt { span: start_span }))
    }

    /// Parse continue statement
    fn parse_continue_stmt(&mut self) -> ParseResult<Stmt> {
        let start_span = self.current_span();
        self.expect(TokenKind::Continue)?;
        self.expect(TokenKind::Semicolon)?;
        Ok(Stmt::Continue(ContinueStmt { span: start_span }))
    }

    /// Parse traverse statement
    /// `traverse(node) { ... }` or `traverse(node) capturing [...] { ... }` or `traverse(node) using Visitor;`
    fn parse_traverse_stmt(&mut self) -> ParseResult<Stmt> {
        let start_span = self.current_span();
        self.expect(TokenKind::Traverse)?;
        self.expect(TokenKind::LParen)?;
        let target = self.parse_expr()?;
        self.expect(TokenKind::RParen)?;

        // Parse optional capturing clause
        let captures = if self.match_token(TokenKind::Capturing) {
            self.parse_capture_list()?
        } else {
            Vec::new()
        };

        let kind = if self.match_token(TokenKind::Using) {
            // Delegated traversal: `traverse(node) using OtherVisitor;`
            let visitor_name = self.expect_ident()?;
            self.expect(TokenKind::Semicolon)?;
            TraverseKind::Delegated(visitor_name)
        } else {
            // Inline traversal: `traverse(node) { ... }`
            let inline_span = self.current_span();
            self.expect(TokenKind::LBrace)?;

            let mut state = Vec::new();
            let mut methods = Vec::new();

            loop {
                self.skip_newlines();
                if self.check(TokenKind::RBrace) {
                    break;
                }

                // Parse either let statements (state) or fn declarations (methods)
                if self.check(TokenKind::Let) {
                    // Parse let statement for state
                    let let_span = self.current_span();
                    self.expect(TokenKind::Let)?;
                    let mutable = self.match_token(TokenKind::Mut);
                    let name = self.expect_ident()?;

                    let ty = if self.match_token(TokenKind::Colon) {
                        Some(self.parse_type()?)
                    } else {
                        None
                    };

                    self.expect(TokenKind::Eq)?;
                    let init = self.parse_expr()?;
                    self.expect(TokenKind::Semicolon)?;

                    state.push(LetStmt {
                        mutable,
                        name,
                        ty,
                        init,
                        span: let_span,
                    });
                } else if self.check(TokenKind::Fn) || self.check(TokenKind::Pub) {
                    // Parse visitor method
                    methods.push(self.parse_function()?);
                } else {
                    return Err(self.error("Expected 'let' or 'fn' in traverse block"));
                }
            }

            self.expect(TokenKind::RBrace)?;

            TraverseKind::Inline(InlineVisitor {
                state,
                methods,
                span: inline_span,
            })
        };

        Ok(Stmt::Traverse(TraverseStmt {
            target,
            captures,
            kind,
            span: start_span,
        }))
    }

    /// Parse capture list: `[&mut x, &y, &mut z]`
    fn parse_capture_list(&mut self) -> ParseResult<Vec<Capture>> {
        self.expect(TokenKind::LBracket)?;
        let mut captures = Vec::new();

        loop {
            self.skip_newlines();
            if self.check(TokenKind::RBracket) {
                break;
            }

            let capture_span = self.current_span();

            // Expect & for reference
            self.expect(TokenKind::Ampersand)?;

            // Check for mut
            let mutable = self.match_token(TokenKind::Mut);

            // Get variable name
            let name = self.expect_ident()?;

            captures.push(Capture {
                name,
                mutable,
                span: capture_span,
            });

            // Check for comma or end
            if !self.match_token(TokenKind::Comma) {
                break;
            }
        }

        self.expect(TokenKind::RBracket)?;
        Ok(captures)
    }

    /// Parse expression statement
    fn parse_expr_stmt(&mut self) -> ParseResult<Stmt> {
        let start_span = self.current_span();
        let expr = self.parse_expr()?;
        self.expect(TokenKind::Semicolon)?;
        Ok(Stmt::Expr(ExprStmt {
            expr,
            span: start_span,
        }))
    }

    /// Parse expression (entry point for expression parsing)
    fn parse_expr(&mut self) -> ParseResult<Expr> {
        self.parse_assignment()
    }

    /// Parse assignment expression
    fn parse_assignment(&mut self) -> ParseResult<Expr> {
        let expr = self.parse_or()?;

        if self.match_token(TokenKind::Eq) {
            let value = self.parse_assignment()?;
            let span = self.current_span();
            return Ok(Expr::Assign(AssignExpr {
                target: Box::new(expr),
                value: Box::new(value),
                span,
            }));
        }

        // Compound assignment
        let op = if self.match_token(TokenKind::PlusEq) {
            Some(CompoundAssignOp::AddAssign)
        } else if self.match_token(TokenKind::MinusEq) {
            Some(CompoundAssignOp::SubAssign)
        } else if self.match_token(TokenKind::StarEq) {
            Some(CompoundAssignOp::MulAssign)
        } else if self.match_token(TokenKind::SlashEq) {
            Some(CompoundAssignOp::DivAssign)
        } else {
            None
        };

        if let Some(op) = op {
            let value = self.parse_assignment()?;
            let span = self.current_span();
            return Ok(Expr::CompoundAssign(CompoundAssignExpr {
                op,
                target: Box::new(expr),
                value: Box::new(value),
                span,
            }));
        }

        Ok(expr)
    }

    /// Parse logical OR
    fn parse_or(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_and()?;

        while self.match_token(TokenKind::Or) {
            let right = self.parse_and()?;
            let span = self.current_span();
            expr = Expr::Binary(BinaryExpr {
                op: BinaryOp::Or,
                left: Box::new(expr),
                right: Box::new(right),
                span,
            });
        }

        Ok(expr)
    }

    /// Parse logical AND
    fn parse_and(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_equality()?;

        while self.match_token(TokenKind::And) {
            let right = self.parse_equality()?;
            let span = self.current_span();
            expr = Expr::Binary(BinaryExpr {
                op: BinaryOp::And,
                left: Box::new(expr),
                right: Box::new(right),
                span,
            });
        }

        Ok(expr)
    }

    /// Parse equality
    fn parse_equality(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_comparison()?;

        loop {
            let op = if self.match_token(TokenKind::EqEq) {
                BinaryOp::Eq
            } else if self.match_token(TokenKind::NotEq) {
                BinaryOp::NotEq
            } else {
                break;
            };

            let right = self.parse_comparison()?;
            let span = self.current_span();
            expr = Expr::Binary(BinaryExpr {
                op,
                left: Box::new(expr),
                right: Box::new(right),
                span,
            });
        }

        Ok(expr)
    }

    /// Parse comparison
    fn parse_comparison(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_term()?;

        loop {
            let op = if self.match_token(TokenKind::Lt) {
                BinaryOp::Lt
            } else if self.match_token(TokenKind::Gt) {
                BinaryOp::Gt
            } else if self.match_token(TokenKind::LtEq) {
                BinaryOp::LtEq
            } else if self.match_token(TokenKind::GtEq) {
                BinaryOp::GtEq
            } else {
                break;
            };

            let right = self.parse_term()?;
            let span = self.current_span();
            expr = Expr::Binary(BinaryExpr {
                op,
                left: Box::new(expr),
                right: Box::new(right),
                span,
            });
        }

        Ok(expr)
    }

    /// Parse term (addition/subtraction)
    fn parse_term(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_factor()?;

        loop {
            let op = if self.match_token(TokenKind::Plus) {
                BinaryOp::Add
            } else if self.match_token(TokenKind::Minus) {
                BinaryOp::Sub
            } else {
                break;
            };

            let right = self.parse_factor()?;
            let span = self.current_span();
            expr = Expr::Binary(BinaryExpr {
                op,
                left: Box::new(expr),
                right: Box::new(right),
                span,
            });
        }

        Ok(expr)
    }

    /// Parse factor (multiplication/division)
    fn parse_factor(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_unary()?;

        loop {
            let op = if self.match_token(TokenKind::Star) {
                BinaryOp::Mul
            } else if self.match_token(TokenKind::Slash) {
                BinaryOp::Div
            } else if self.match_token(TokenKind::Percent) {
                BinaryOp::Mod
            } else {
                break;
            };

            let right = self.parse_unary()?;
            let span = self.current_span();
            expr = Expr::Binary(BinaryExpr {
                op,
                left: Box::new(expr),
                right: Box::new(right),
                span,
            });
        }

        Ok(expr)
    }

    /// Parse unary expression
    fn parse_unary(&mut self) -> ParseResult<Expr> {
        let span = self.current_span();

        if self.match_token(TokenKind::Not) {
            let operand = self.parse_unary()?;
            return Ok(Expr::Unary(UnaryExpr {
                op: UnaryOp::Not,
                operand: Box::new(operand),
                span,
            }));
        }

        if self.match_token(TokenKind::Minus) {
            let operand = self.parse_unary()?;
            return Ok(Expr::Unary(UnaryExpr {
                op: UnaryOp::Neg,
                operand: Box::new(operand),
                span,
            }));
        }

        if self.match_token(TokenKind::Star) {
            let operand = self.parse_unary()?;
            return Ok(Expr::Deref(DerefExpr {
                expr: Box::new(operand),
                span,
            }));
        }

        if self.match_token(TokenKind::Ampersand) {
            let mutable = self.match_token(TokenKind::Mut);
            let operand = self.parse_unary()?;
            return Ok(Expr::Ref(RefExpr {
                mutable,
                expr: Box::new(operand),
                span,
            }));
        }

        self.parse_call()
    }

    /// Parse call/member/index expression
    fn parse_call(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.match_token(TokenKind::LParen) {
                // Function call
                let args = self.parse_args()?;
                self.expect(TokenKind::RParen)?;
                let span = self.current_span();
                expr = Expr::Call(CallExpr {
                    callee: Box::new(expr),
                    args,
                    type_args: Vec::new(),
                    optional: false,
                    span,
                });
            } else if self.match_token(TokenKind::Dot) {
                // Member access
                let property = self.expect_ident()?;
                let span = self.current_span();
                expr = Expr::Member(MemberExpr {
                    object: Box::new(expr),
                    property,
                    optional: false,
                    computed: false,
                    span,
                });
            } else if self.match_token(TokenKind::QuestionDot) {
                // Optional member access ?.
                let property = self.expect_ident()?;
                let span = self.current_span();
                expr = Expr::Member(MemberExpr {
                    object: Box::new(expr),
                    property,
                    optional: true,
                    computed: false,
                    span,
                });
            } else if self.match_token(TokenKind::LBracket) {
                // Index access
                let index = self.parse_expr()?;
                self.expect(TokenKind::RBracket)?;
                let span = self.current_span();
                expr = Expr::Index(IndexExpr {
                    object: Box::new(expr),
                    index: Box::new(index),
                    span,
                });
            } else if self.match_token(TokenKind::ColonColon) {
                // Static method call (treat as member for now)
                let method = self.expect_ident()?;
                let span = self.current_span();
                expr = Expr::Member(MemberExpr {
                    object: Box::new(expr),
                    property: method,
                    optional: false,
                    computed: false,
                    span,
                });
            } else {
                break;
            }
        }

        Ok(expr)
    }

    /// Parse function call arguments
    fn parse_args(&mut self) -> ParseResult<Vec<Expr>> {
        let mut args = Vec::new();

        if self.check(TokenKind::RParen) {
            return Ok(args);
        }

        loop {
            args.push(self.parse_expr()?);
            if !self.match_token(TokenKind::Comma) {
                break;
            }
        }

        Ok(args)
    }

    /// Parse primary expression
    fn parse_primary(&mut self) -> ParseResult<Expr> {
        let span = self.current_span();

        // Parenthesized expression
        if self.match_token(TokenKind::LParen) {
            // Check for closure: |params| expr
            if self.check(TokenKind::Pipe) {
                return self.parse_closure(span);
            }
            let expr = self.parse_expr()?;
            self.expect(TokenKind::RParen)?;
            return Ok(Expr::Paren(Box::new(expr)));
        }

        // Closure
        if self.check(TokenKind::Pipe) {
            return self.parse_closure(span);
        }

        // Literal
        if let Some(lit) = self.try_parse_literal() {
            return Ok(Expr::Literal(lit));
        }

        // Vec initialization: vec![...]
        if self.check_ident("vec") {
            self.advance();
            if self.match_token(TokenKind::Not) {
                self.expect(TokenKind::LBracket)?;
                let mut elements = Vec::new();
                if !self.check(TokenKind::RBracket) {
                    loop {
                        elements.push(self.parse_expr()?);
                        if !self.match_token(TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.expect(TokenKind::RBracket)?;
                return Ok(Expr::VecInit(VecInitExpr { elements, span }));
            } else {
                // Just identifier "vec"
                return Ok(Expr::Ident(IdentExpr {
                    name: "vec".to_string(),
                    span,
                }));
            }
        }

        // format! macro (treat as function call)
        if self.check_ident("format") {
            self.advance();
            if self.match_token(TokenKind::Not) {
                self.expect(TokenKind::LParen)?;
                let args = self.parse_args()?;
                self.expect(TokenKind::RParen)?;
                return Ok(Expr::Call(CallExpr {
                    callee: Box::new(Expr::Ident(IdentExpr {
                        name: "format".to_string(),
                        span,
                    })),
                    args,
                    type_args: Vec::new(),
                    optional: false,
                    span,
                }));
            } else {
                return Ok(Expr::Ident(IdentExpr {
                    name: "format".to_string(),
                    span,
                }));
            }
        }

        // matches! macro
        if self.match_token(TokenKind::Matches) {
            self.expect(TokenKind::LParen)?;
            let args = self.parse_args()?;
            self.expect(TokenKind::RParen)?;
            return Ok(Expr::Call(CallExpr {
                callee: Box::new(Expr::Ident(IdentExpr {
                    name: "matches!".to_string(),
                    span,
                })),
                args,
                type_args: Vec::new(),
                optional: false,
                span,
            }));
        }

        // Self
        if self.match_token(TokenKind::Self_) {
            return Ok(Expr::Ident(IdentExpr {
                name: "self".to_string(),
                span,
            }));
        }

        if self.match_token(TokenKind::SelfType) {
            return Ok(Expr::Ident(IdentExpr {
                name: "Self".to_string(),
                span,
            }));
        }

        // Identifier or struct init
        if let Some(name) = self.try_expect_ident() {
            // Check for struct initialization
            if self.check(TokenKind::LBrace) {
                return self.parse_struct_init(name, span);
            }
            return Ok(Expr::Ident(IdentExpr { name, span }));
        }

        // AST node type as identifier
        if let Some(name) = self.try_expect_ast_type() {
            if self.check(TokenKind::LBrace) {
                return self.parse_struct_init(name, span);
            }
            return Ok(Expr::Ident(IdentExpr { name, span }));
        }

        Err(self.error("Expected expression"))
    }

    /// Parse closure expression
    fn parse_closure(&mut self, span: Span) -> ParseResult<Expr> {
        self.expect(TokenKind::Pipe)?;
        let mut params = Vec::new();
        if !self.check(TokenKind::Pipe) {
            loop {
                params.push(self.expect_ident()?);
                if !self.match_token(TokenKind::Comma) {
                    break;
                }
            }
        }
        self.expect(TokenKind::Pipe)?;
        let body = self.parse_expr()?;
        Ok(Expr::Closure(ClosureExpr {
            params,
            body: Box::new(body),
            span,
        }))
    }

    /// Parse struct initialization
    fn parse_struct_init(&mut self, name: String, span: Span) -> ParseResult<Expr> {
        self.expect(TokenKind::LBrace)?;
        let mut fields = Vec::new();

        loop {
            self.skip_newlines();
            if self.check(TokenKind::RBrace) {
                break;
            }

            let field_name = self.expect_ident()?;
            self.expect(TokenKind::Colon)?;
            let value = self.parse_expr()?;
            fields.push((field_name, value));

            self.skip_newlines();
            if !self.match_token(TokenKind::Comma) {
                break;
            }
        }

        self.expect(TokenKind::RBrace)?;
        Ok(Expr::StructInit(StructInitExpr { name, fields, span }))
    }

    /// Try to parse a literal
    fn try_parse_literal(&mut self) -> Option<Literal> {
        match self.peek() {
            Some(Token { kind: TokenKind::StringLit(s), .. }) => {
                let s = s.clone();
                self.advance();
                Some(Literal::String(s))
            }
            Some(Token { kind: TokenKind::IntLit(n), .. }) => {
                let n = *n;
                self.advance();
                Some(Literal::Int(n))
            }
            Some(Token { kind: TokenKind::FloatLit(n), .. }) => {
                let n = *n;
                self.advance();
                Some(Literal::Float(n))
            }
            Some(Token { kind: TokenKind::True, .. }) => {
                self.advance();
                Some(Literal::Bool(true))
            }
            Some(Token { kind: TokenKind::False, .. }) => {
                self.advance();
                Some(Literal::Bool(false))
            }
            Some(Token { kind: TokenKind::Null, .. }) => {
                self.advance();
                Some(Literal::Null)
            }
            _ => None,
        }
    }

    // === Helper methods ===

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<&Token> {
        if !self.is_at_end() {
            self.pos += 1;
        }
        self.tokens.get(self.pos - 1)
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek(), Some(Token { kind: TokenKind::Eof, .. }) | None)
    }

    fn check(&self, kind: TokenKind) -> bool {
        matches!(self.peek(), Some(t) if std::mem::discriminant(&t.kind) == std::mem::discriminant(&kind))
    }

    fn check_ident(&self, name: &str) -> bool {
        matches!(self.peek(), Some(Token { kind: TokenKind::Ident(n), .. }) if n == name)
    }

    fn match_token(&mut self, kind: TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: TokenKind) -> ParseResult<()> {
        if self.check(kind.clone()) {
            self.advance();
            Ok(())
        } else {
            Err(self.error(format!("Expected {:?}", kind)))
        }
    }

    fn expect_ident(&mut self) -> ParseResult<String> {
        match self.peek() {
            Some(Token { kind: TokenKind::Ident(name), .. }) => {
                let name = name.clone();
                self.advance();
                Ok(name)
            }
            _ => Err(self.error("Expected identifier")),
        }
    }

    fn try_expect_ident(&mut self) -> Option<String> {
        match self.peek() {
            Some(Token { kind: TokenKind::Ident(name), .. }) => {
                let name = name.clone();
                self.advance();
                Some(name)
            }
            _ => None,
        }
    }

    fn try_expect_ast_type(&mut self) -> Option<String> {
        let name = match self.peek()?.kind {
            TokenKind::Program => "Program",
            TokenKind::FunctionDeclaration => "FunctionDeclaration",
            TokenKind::VariableDeclaration => "VariableDeclaration",
            TokenKind::ExpressionStatement => "ExpressionStatement",
            TokenKind::ReturnStatement => "ReturnStatement",
            TokenKind::IfStatement => "IfStatement",
            TokenKind::ForStatement => "ForStatement",
            TokenKind::WhileStatement => "WhileStatement",
            TokenKind::BlockStatement => "BlockStatement",
            TokenKind::Identifier => "Identifier",
            TokenKind::Literal => "Literal",
            TokenKind::BinaryExpression => "BinaryExpression",
            TokenKind::UnaryExpression => "UnaryExpression",
            TokenKind::CallExpression => "CallExpression",
            TokenKind::MemberExpression => "MemberExpression",
            TokenKind::ArrayExpression => "ArrayExpression",
            TokenKind::ObjectExpression => "ObjectExpression",
            TokenKind::JSXElement => "JSXElement",
            TokenKind::JSXFragment => "JSXFragment",
            TokenKind::JSXAttribute => "JSXAttribute",
            TokenKind::JSXText => "JSXText",
            TokenKind::JSXExpressionContainer => "JSXExpressionContainer",
            _ => return None,
        };
        self.advance();
        Some(name.to_string())
    }

    fn expect_type_name(&mut self) -> ParseResult<String> {
        // First try AST types
        if let Some(name) = self.try_expect_ast_type() {
            return Ok(name);
        }

        // Try primitive types
        let name = match self.peek() {
            Some(Token { kind: TokenKind::Str, .. }) => "Str",
            Some(Token { kind: TokenKind::Bool, .. }) => "bool",
            Some(Token { kind: TokenKind::I32, .. }) => "i32",
            Some(Token { kind: TokenKind::U32, .. }) => "u32",
            Some(Token { kind: TokenKind::F64, .. }) => "f64",
            Some(Token { kind: TokenKind::Vec, .. }) => "Vec",
            Some(Token { kind: TokenKind::Option, .. }) => "Option",
            Some(Token { kind: TokenKind::Result, .. }) => "Result",
            Some(Token { kind: TokenKind::HashMap, .. }) => "HashMap",
            Some(Token { kind: TokenKind::HashSet, .. }) => "HashSet",
            Some(Token { kind: TokenKind::CodeBuilder, .. }) => "CodeBuilder",
            Some(Token { kind: TokenKind::Ident(n), .. }) => {
                let name = n.clone();
                self.advance();
                return Ok(name);
            }
            _ => return Err(self.error("Expected type name")),
        };
        self.advance();
        Ok(name.to_string())
    }

    fn skip_newlines(&mut self) {
        while matches!(self.peek(), Some(Token { kind: TokenKind::Newline | TokenKind::Comment(_) | TokenKind::DocComment(_), .. })) {
            self.advance();
        }
    }

    fn current_span(&self) -> Span {
        self.peek()
            .map(|t| t.span)
            .unwrap_or(Span::new(0, 0, 0, 0))
    }

    fn error(&self, message: impl Into<String>) -> ParseError {
        ParseError::new(message, self.current_span())
    }
}
