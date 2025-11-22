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

        // Parse use statements
        let mut uses = Vec::new();
        while self.check(TokenKind::Use) {
            uses.push(self.parse_use_stmt()?);
            self.skip_newlines();
        }

        let decl = if self.check(TokenKind::Plugin) {
            TopLevelDecl::Plugin(self.parse_plugin()?)
        } else if self.check(TokenKind::Writer) {
            TopLevelDecl::Writer(self.parse_writer()?)
        } else {
            // No plugin/writer keyword - treat as standalone module
            TopLevelDecl::Module(self.parse_module()?)
        };

        Ok(Program {
            uses,
            decl,
            span: start_span,
        })
    }

    /// Parse use statement: `use fs;` or `use "./helpers.rsc";` or `use "./helpers.rsc" as h { foo, bar };`
    fn parse_use_stmt(&mut self) -> ParseResult<UseStmt> {
        let start_span = self.current_span();
        self.expect(TokenKind::Use)?;

        // Parse module path (string literal for files, identifier for built-ins)
        let path = if self.check(TokenKind::StringLit(String::new())) {
            // File path: use "./helpers.rsc";
            if let Some(Token { kind: TokenKind::StringLit(s), .. }) = self.peek() {
                let path = s.clone();
                self.advance();
                path
            } else {
                return Err(self.error("Expected string literal"));
            }
        } else {
            // Built-in module: use fs;
            self.expect_ident()?
        };

        // Optional: as alias
        let alias = if self.check(TokenKind::As) {
            self.advance();
            Some(self.expect_ident()?)
        } else {
            None
        };

        // Optional: { imports }
        let imports = if self.check(TokenKind::LBrace) {
            self.parse_import_list()?
        } else {
            vec![]
        };

        self.expect(TokenKind::Semicolon)?;

        Ok(UseStmt {
            path,
            alias,
            imports,
            span: start_span,
        })
    }

    /// Parse import list: `{ foo, bar, baz }`
    fn parse_import_list(&mut self) -> ParseResult<Vec<String>> {
        self.expect(TokenKind::LBrace)?;
        self.skip_newlines();  // Allow newlines after {
        let mut imports = Vec::new();

        loop {
            // Allow trailing commas and empty lists
            if self.check(TokenKind::RBrace) {
                break;
            }

            imports.push(self.expect_ident()?);
            self.skip_newlines();  // Allow newlines after identifier

            if !self.check(TokenKind::Comma) {
                break;
            }
            self.advance(); // consume comma
            self.skip_newlines();  // Allow newlines after comma
        }

        self.expect(TokenKind::RBrace)?;
        Ok(imports)
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

    /// Parse module declaration (standalone module without plugin/writer keyword)
    fn parse_module(&mut self) -> ParseResult<ModuleDecl> {
        let start_span = self.current_span();

        // Parse module items (functions, structs, enums) until EOF
        let items = self.parse_plugin_body()?;

        Ok(ModuleDecl {
            items,
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

            // Check for 'mut self' parameter
            if self.check(TokenKind::Mut) {
                let peek_ahead = self.tokens.get(self.pos + 1);
                if matches!(peek_ahead, Some(Token { kind: TokenKind::Self_, .. })) {
                    self.advance(); // consume 'mut'
                    self.advance(); // consume 'self'
                    params.push(Param {
                        name: "self".to_string(),
                        ty: Type::Named("Self".to_string()), // mut self consumes Self
                        span: param_span,
                    });

                    if !self.match_token(TokenKind::Comma) {
                        break;
                    }
                    continue;
                }
            }

            // Check for self parameters (self, &self, &mut self)
            if self.check(TokenKind::Self_) {
                // Just 'self' - consuming parameter
                self.advance();
                params.push(Param {
                    name: "self".to_string(),
                    ty: Type::Named("Self".to_string()),
                    span: param_span,
                });

                if !self.match_token(TokenKind::Comma) {
                    break;
                }
                continue;
            }

            // Check for &self or &mut self
            if self.check(TokenKind::Ampersand) {
                self.advance(); // consume &

                let is_mut = self.match_token(TokenKind::Mut);

                if self.check(TokenKind::Self_) {
                    self.advance(); // consume 'self'
                    params.push(Param {
                        name: "self".to_string(),
                        ty: Type::Reference {
                            mutable: is_mut,
                            inner: Box::new(Type::Named("Self".to_string())),
                        },
                        span: param_span,
                    });

                    if !self.match_token(TokenKind::Comma) {
                        break;
                    }
                    continue;
                }

                // Not a self parameter - this is an error
                // RustScript doesn't support &param syntax, only param: &Type
                return Err(self.error("Unexpected '&' - use 'param: &Type' syntax instead"));
            }

            // Regular parameter: name: Type
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

        // Check for tuple type: (T1, T2, ...)
        if self.check(TokenKind::LParen) {
            return self.parse_tuple_type();
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

    /// Parse tuple type: (T1, T2, ...)
    fn parse_tuple_type(&mut self) -> ParseResult<Type> {
        self.expect(TokenKind::LParen)?;

        // Empty tuple () is the unit type
        if self.check(TokenKind::RParen) {
            self.advance();
            return Ok(Type::Unit);
        }

        // Parse tuple element types
        let mut types = Vec::new();
        loop {
            types.push(self.parse_type()?);
            if !self.match_token(TokenKind::Comma) {
                break;
            }
            // Allow trailing comma
            if self.check(TokenKind::RParen) {
                break;
            }
        }
        self.expect(TokenKind::RParen)?;

        // Single element with no trailing comma is just a parenthesized type, not a tuple
        if types.len() == 1 {
            return Ok(types.into_iter().next().unwrap());
        }

        Ok(Type::Tuple(types))
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

        // Parse pattern (supports simple identifiers and tuple destructuring)
        let pattern = self.parse_pattern()?;

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
            pattern,
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
            let expr = self.parse_expr_no_struct()?;
            (Some(pat), expr)
        } else {
            // Use parse_expr_no_struct to avoid ambiguity with block
            (None, self.parse_expr_no_struct()?)
        };

        let then_branch = self.parse_block()?;

        let mut else_if_branches = Vec::new();
        let mut else_branch = None;

        while self.match_token(TokenKind::Else) {
            if self.match_token(TokenKind::If) {
                // Note: else-if with let not supported yet, just regular condition
                let cond = self.parse_expr_no_struct()?;
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

        // Parse scrutinee - must not consume the { that starts match arms
        // We parse a restricted expression that stops at {
        let scrutinee = self.parse_match_scrutinee()?;

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

    /// Parse match scrutinee (expression that doesn't consume {)
    fn parse_match_scrutinee(&mut self) -> ParseResult<Expr> {
        // Use parse_or_no_struct to avoid consuming { as struct init
        self.parse_or_no_struct()
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

        // Check for tuple pattern: (a, b, c)
        if self.check(TokenKind::LParen) {
            self.advance();
            let mut elements = Vec::new();

            // Empty tuple ()
            if self.check(TokenKind::RParen) {
                self.advance();
                return Ok(Pattern::Tuple(elements));
            }

            // Parse tuple elements
            loop {
                elements.push(self.parse_pattern()?);
                if !self.match_token(TokenKind::Comma) {
                    break;
                }
                // Allow trailing comma
                if self.check(TokenKind::RParen) {
                    break;
                }
            }

            self.expect(TokenKind::RParen)?;

            // Single element is not a tuple, just a parenthesized pattern
            if elements.len() == 1 {
                return Ok(elements.into_iter().next().unwrap());
            }

            return Ok(Pattern::Tuple(elements));
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

        // Parse pattern (identifier or tuple destructuring)
        let pattern = self.parse_pattern()?;

        self.expect(TokenKind::In)?;
        // Use parse_expr_no_struct to avoid ambiguity with block
        let iter = self.parse_expr_no_struct()?;
        let body = self.parse_block()?;

        Ok(Stmt::For(ForStmt {
            pattern,
            iter,
            body,
            span: start_span,
        }))
    }

    /// Parse expression without allowing struct initialization
    /// This is used in contexts where `{` starts a block, not a struct
    fn parse_expr_no_struct(&mut self) -> ParseResult<Expr> {
        // Parse the expression but stop if we see an identifier followed by {
        // We need to support binary operators like ==, &&, ||
        self.parse_or_no_struct()
    }

    fn parse_or_no_struct(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_and_no_struct()?;

        while self.match_token(TokenKind::Or) {
            let right = self.parse_and_no_struct()?;
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

    fn parse_and_no_struct(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_equality_no_struct()?;

        while self.match_token(TokenKind::And) {
            let right = self.parse_equality_no_struct()?;
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

    fn parse_equality_no_struct(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_comparison_no_struct()?;

        loop {
            let op = if self.match_token(TokenKind::EqEq) {
                BinaryOp::Eq
            } else if self.match_token(TokenKind::NotEq) {
                BinaryOp::NotEq
            } else {
                break;
            };

            let right = self.parse_comparison_no_struct()?;
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

    fn parse_comparison_no_struct(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_term_no_struct()?;

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

            let right = self.parse_term_no_struct()?;
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

    fn parse_term_no_struct(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_factor_no_struct()?;

        loop {
            let op = if self.match_token(TokenKind::Plus) {
                BinaryOp::Add
            } else if self.match_token(TokenKind::Minus) {
                BinaryOp::Sub
            } else {
                break;
            };

            let right = self.parse_factor_no_struct()?;
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

    fn parse_factor_no_struct(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_unary_no_struct()?;

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

            let right = self.parse_unary_no_struct()?;
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

    fn parse_unary_no_struct(&mut self) -> ParseResult<Expr> {
        let span = self.current_span();

        // Handle unary operators
        if self.match_token(TokenKind::Not) {
            let operand = self.parse_unary_no_struct()?;
            return Ok(Expr::Unary(UnaryExpr {
                op: UnaryOp::Not,
                operand: Box::new(operand),
                span,
            }));
        }
        if self.match_token(TokenKind::Minus) {
            let operand = self.parse_unary_no_struct()?;
            return Ok(Expr::Unary(UnaryExpr {
                op: UnaryOp::Neg,
                operand: Box::new(operand),
                span,
            }));
        }
        if self.match_token(TokenKind::Star) {
            let operand = self.parse_unary_no_struct()?;
            return Ok(Expr::Unary(UnaryExpr {
                op: UnaryOp::Deref,
                operand: Box::new(operand),
                span,
            }));
        }
        if self.match_token(TokenKind::Ampersand) {
            let is_mut = self.match_token(TokenKind::Mut);
            let operand = self.parse_unary_no_struct()?;
            return Ok(Expr::Unary(UnaryExpr {
                op: if is_mut { UnaryOp::RefMut } else { UnaryOp::Ref },
                operand: Box::new(operand),
                span,
            }));
        }

        self.parse_primary_no_struct()
    }

    fn parse_primary_no_struct(&mut self) -> ParseResult<Expr> {
        let span = self.current_span();

        // Handle 'self' keyword
        if self.match_token(TokenKind::Self_) {
            let mut expr = Expr::Ident(IdentExpr {
                name: "self".to_string(),
                span,
            });

            // Handle member access on self
            loop {
                if self.match_token(TokenKind::Dot) {
                    let property = self.expect_ident()?;
                    let span = self.current_span();
                    expr = Expr::Member(MemberExpr {
                        object: Box::new(expr),
                        property,
                        optional: false,
                        computed: false,
                        is_path: false,
                        span,
                    });
                } else {
                    break;
                }
            }

            return Ok(expr);
        }

        // Identifier (no struct init)
        if let Some(name) = self.try_expect_ident() {
            // Don't check for LBrace here - just return the identifier
            let mut expr = Expr::Ident(IdentExpr { name, span });

            // Handle postfix operators (member access, calls, etc.) but not struct init
            loop {
                if self.match_token(TokenKind::Dot) {
                    let property = self.expect_ident()?;
                    let span = self.current_span();
                    expr = Expr::Member(MemberExpr {
                        object: Box::new(expr),
                        property,
                        optional: false,
                        computed: false,
                        is_path: false,
                        span,
                    });
                } else if self.match_token(TokenKind::LParen) {
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
                } else if self.match_token(TokenKind::LBracket) {
                    let index = self.parse_expr()?;
                    self.expect(TokenKind::RBracket)?;
                    let span = self.current_span();
                    expr = Expr::Index(IndexExpr {
                        object: Box::new(expr),
                        index: Box::new(index),
                        span,
                    });
                } else if self.match_token(TokenKind::ColonColon) {
                    // Path expression like fs::write or HashMap::new
                    let method = self.expect_ident()?;
                    let span = self.current_span();
                    expr = Expr::Member(MemberExpr {
                        object: Box::new(expr),
                        property: method,
                        optional: false,
                        computed: false,
                        is_path: true,
                        span,
                    });
                } else {
                    // Don't handle LBrace here - that would be struct init
                    break;
                }
            }

            return Ok(expr);
        }

        // For other cases, delegate to normal parse_primary
        self.parse_primary()
    }

    /// Parse while statement
    fn parse_while_stmt(&mut self) -> ParseResult<Stmt> {
        let start_span = self.current_span();
        self.expect(TokenKind::While)?;
        // Use parse_expr_no_struct to avoid ambiguity with block
        let condition = self.parse_expr_no_struct()?;
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
                        pattern: Pattern::Ident(name),
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

        // Semicolon is optional if this is the last expression in a block (before RBrace)
        // This allows the expression to serve as the block's return value
        self.skip_newlines();
        if !self.check(TokenKind::RBrace) {
            self.expect(TokenKind::Semicolon)?;
        } else {
            // Try to consume semicolon if present, but don't require it
            self.match_token(TokenKind::Semicolon);
        }

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
            // Skip newlines to allow method chaining across lines
            self.skip_newlines();

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
                    is_path: false,
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
                    is_path: false,
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
                // Static method call like HashMap::new or Expr::CallExpression
                let method = if let Some(ast_type) = self.try_expect_ast_type() {
                    ast_type
                } else {
                    self.expect_ident()?
                };
                let span = self.current_span();
                expr = Expr::Member(MemberExpr {
                    object: Box::new(expr),
                    property: method,
                    optional: false,
                    computed: false,
                    is_path: true,
                    span,
                });
            } else if self.match_token(TokenKind::Question) {
                // Try operator: expr?
                let span = self.current_span();
                expr = Expr::Try(Box::new(expr));
            } else {
                break;
            }
        }

        Ok(expr)
    }

    /// Parse function call arguments
    fn parse_args(&mut self) -> ParseResult<Vec<Expr>> {
        let mut args = Vec::new();

        self.skip_newlines();
        if self.check(TokenKind::RParen) {
            return Ok(args);
        }

        loop {
            self.skip_newlines();
            args.push(self.parse_expr()?);
            self.skip_newlines();
            if !self.match_token(TokenKind::Comma) {
                break;
            }
        }

        Ok(args)
    }

    /// Parse primary expression
    fn parse_primary(&mut self) -> ParseResult<Expr> {
        let span = self.current_span();

        // Block expression
        if self.check(TokenKind::LBrace) {
            let block = self.parse_block()?;
            return Ok(Expr::Block(block));
        }

        // Parenthesized expression or unit literal ()
        if self.match_token(TokenKind::LParen) {
            // Check for unit literal ()
            if self.check(TokenKind::RParen) {
                self.advance();
                return Ok(Expr::Literal(Literal::Unit));
            }
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
                self.skip_newlines();
                let mut elements = Vec::new();
                if !self.check(TokenKind::RBracket) {
                    loop {
                        elements.push(self.parse_expr()?);
                        self.skip_newlines();
                        if !self.match_token(TokenKind::Comma) {
                            break;
                        }
                        self.skip_newlines();
                        // Allow trailing comma
                        if self.check(TokenKind::RBracket) {
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
            let name = "Self".to_string();
            if self.check(TokenKind::LBrace) {
                return self.parse_struct_init(name, span);
            }
            return Ok(Expr::Ident(IdentExpr {
                name,
                span,
            }));
        }

        // Identifier or struct init
        if let Some(name) = self.try_expect_ident() {
            // Check for struct initialization or wildcard pattern TypeName(_)
            if self.check(TokenKind::LBrace) {
                return self.parse_struct_init(name, span);
            }
            // Check for wildcard pattern: TypeName(_)
            if self.check(TokenKind::LParen) {
                self.advance(); // consume (
                // Check if next token is underscore (as identifier)
                if self.check_ident("_") {
                    self.advance(); // consume _
                    self.expect(TokenKind::RParen)?;
                    // Return a struct init with a special marker for wildcard
                    return Ok(Expr::StructInit(StructInitExpr {
                        name,
                        fields: vec![("_wildcard".to_string(), Expr::Ident(IdentExpr {
                            name: "_".to_string(),
                            span,
                        }))],
                        span,
                    }));
                }
                // Not a wildcard pattern, parse as call
                let mut args = vec![];
                if !self.check(TokenKind::RParen) {
                    args = self.parse_args()?;
                }
                self.expect(TokenKind::RParen)?;
                return Ok(Expr::Call(CallExpr {
                    callee: Box::new(Expr::Ident(IdentExpr { name, span })),
                    args,
                    type_args: Vec::new(),
                    optional: false,
                    span,
                }));
            }
            return Ok(Expr::Ident(IdentExpr { name, span }));
        }

        // AST node type as identifier
        if let Some(name) = self.try_expect_ast_type() {
            // Check for struct initialization
            if self.check(TokenKind::LBrace) {
                return self.parse_struct_init(name, span);
            }
            // Check for wildcard pattern: TypeName(_)
            if self.check(TokenKind::LParen) {
                self.advance(); // consume (
                // Check if next token is underscore (as identifier)
                if self.check_ident("_") {
                    self.advance(); // consume _
                    self.expect(TokenKind::RParen)?;
                    // Return a struct init with a special marker for wildcard
                    return Ok(Expr::StructInit(StructInitExpr {
                        name,
                        fields: vec![("_wildcard".to_string(), Expr::Ident(IdentExpr {
                            name: "_".to_string(),
                            span,
                        }))],
                        span,
                    }));
                }
                // Not a wildcard - this is invalid syntax for AST types
                // AST types can only be used with {} or (_), not regular calls
                return Err(ParseError::new(
                    "AST type must be followed by { } for struct pattern or (_) for wildcard",
                    self.current_span(),
                ));
            }
            return Ok(Expr::Ident(IdentExpr { name, span }));
        }

        // Self type (can be used for struct initialization)
        if self.match_token(TokenKind::SelfType) {
            let name = "Self".to_string();
            if self.check(TokenKind::LBrace) {
                return self.parse_struct_init(name, span);
            }
            return Ok(Expr::Ident(IdentExpr { name, span }));
        }

        // Type keywords that can be used as path expressions (HashMap::new(), etc.)
        let type_name = match self.peek() {
            Some(Token { kind: TokenKind::HashMap, .. }) => Some("HashMap"),
            Some(Token { kind: TokenKind::HashSet, .. }) => Some("HashSet"),
            Some(Token { kind: TokenKind::Vec, .. }) => Some("Vec"),
            Some(Token { kind: TokenKind::Option, .. }) => Some("Option"),
            Some(Token { kind: TokenKind::Result, .. }) => Some("Result"),
            Some(Token { kind: TokenKind::Str, .. }) => Some("String"),
            _ => None,
        };

        if let Some(name) = type_name {
            self.advance();
            let name = name.to_string();
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

        // Closure body can be either an expression or a block
        let body = if self.check(TokenKind::LBrace) {
            let block = self.parse_block()?;
            Expr::Block(block)
        } else {
            self.parse_expr()?
        };

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
            // Also accept type keywords as identifiers in expression position
            // This allows HashMap::new(), CodeBuilder::new(), etc.
            Some(Token { kind: TokenKind::HashMap, .. }) => {
                self.advance();
                Some("HashMap".to_string())
            }
            Some(Token { kind: TokenKind::HashSet, .. }) => {
                self.advance();
                Some("HashSet".to_string())
            }
            Some(Token { kind: TokenKind::CodeBuilder, .. }) => {
                self.advance();
                Some("CodeBuilder".to_string())
            }
            Some(Token { kind: TokenKind::Vec, .. }) => {
                self.advance();
                Some("Vec".to_string())
            }
            Some(Token { kind: TokenKind::Option, .. }) => {
                self.advance();
                Some("Option".to_string())
            }
            Some(Token { kind: TokenKind::Result, .. }) => {
                self.advance();
                Some("Result".to_string())
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
            Some(Token { kind: TokenKind::SelfType, .. }) => "Self",
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
