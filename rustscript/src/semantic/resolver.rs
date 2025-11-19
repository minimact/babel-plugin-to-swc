//! Name resolution pass for RustScript

use std::collections::HashMap;
use crate::parser::*;
use crate::semantic::{SemanticError, TypeEnv, TypeInfo, types::ast_type_to_type_info};
use crate::mapping::get_node_mapping;

/// Name resolver - resolves all identifiers and builds the type environment
pub struct Resolver {
    env: TypeEnv,
    errors: Vec<SemanticError>,
}

impl Resolver {
    pub fn new() -> Self {
        Self {
            env: TypeEnv::new(),
            errors: Vec::new(),
        }
    }

    /// Run name resolution
    pub fn resolve(&mut self, program: &Program) -> Result<(), Vec<SemanticError>> {
        match &program.decl {
            TopLevelDecl::Plugin(plugin) => self.resolve_plugin(plugin),
            TopLevelDecl::Writer(writer) => self.resolve_writer(writer),
            TopLevelDecl::Interface(_) => {} // Interfaces don't need resolution
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(std::mem::take(&mut self.errors))
        }
    }

    /// Get the type environment
    pub fn get_env(&self) -> &TypeEnv {
        &self.env
    }

    /// Take ownership of the type environment
    pub fn into_env(self) -> TypeEnv {
        self.env
    }

    fn resolve_plugin(&mut self, plugin: &PluginDecl) {
        // Define the plugin in scope
        self.env.define(
            plugin.name.clone(),
            TypeInfo::Struct {
                name: plugin.name.clone(),
                fields: HashMap::new(),
            },
        );

        // Enter plugin scope
        self.env.push_scope();

        // First pass: collect all struct and enum definitions
        for item in &plugin.body {
            match item {
                PluginItem::Struct(s) => self.declare_struct(s),
                PluginItem::Enum(e) => self.declare_enum(e),
                _ => {}
            }
        }

        // Second pass: collect function signatures
        for item in &plugin.body {
            if let PluginItem::Function(f) = item {
                self.declare_function(f);
            }
        }

        // Third pass: resolve function bodies
        for item in &plugin.body {
            if let PluginItem::Function(f) = item {
                self.resolve_function(f);
            }
        }

        self.env.pop_scope();
    }

    fn resolve_writer(&mut self, writer: &WriterDecl) {
        // Same as plugin for now
        self.env.define(
            writer.name.clone(),
            TypeInfo::Struct {
                name: writer.name.clone(),
                fields: HashMap::new(),
            },
        );

        self.env.push_scope();

        for item in &writer.body {
            match item {
                PluginItem::Struct(s) => self.declare_struct(s),
                PluginItem::Enum(e) => self.declare_enum(e),
                _ => {}
            }
        }

        for item in &writer.body {
            if let PluginItem::Function(f) = item {
                self.declare_function(f);
            }
        }

        for item in &writer.body {
            if let PluginItem::Function(f) = item {
                self.resolve_function(f);
            }
        }

        self.env.pop_scope();
    }

    fn declare_struct(&mut self, s: &StructDecl) {
        let mut fields = HashMap::new();
        for field in &s.fields {
            let ty = ast_type_to_type_info(&field.ty);
            fields.insert(field.name.clone(), ty);
        }
        self.env.define_struct(s.name.clone(), fields);
    }

    fn declare_enum(&mut self, e: &EnumDecl) {
        let mut variants = HashMap::new();
        for variant in &e.variants {
            let fields = variant.fields.as_ref().map(|types| {
                types.iter().map(ast_type_to_type_info).collect()
            });
            variants.insert(variant.name.clone(), fields);
        }
        self.env.define_enum(e.name.clone(), variants);
    }

    fn declare_function(&mut self, f: &FnDecl) {
        let params: Vec<TypeInfo> = f.params.iter().map(|p| ast_type_to_type_info(&p.ty)).collect();
        let ret = f
            .return_type
            .as_ref()
            .map(ast_type_to_type_info)
            .unwrap_or(TypeInfo::Unit);
        self.env.define_function(f.name.clone(), params, ret);
    }

    fn resolve_function(&mut self, f: &FnDecl) {
        self.env.push_scope();

        // Define parameters
        for param in &f.params {
            let ty = ast_type_to_type_info(&param.ty);
            if self.env.is_defined_in_current_scope(&param.name) {
                self.errors.push(SemanticError::new(
                    "RS004",
                    format!("Duplicate parameter name: {}", param.name),
                    param.span,
                ));
            } else {
                self.env.define(param.name.clone(), ty);
            }
        }

        // Resolve body
        self.resolve_block(&f.body);

        self.env.pop_scope();
    }

    fn resolve_block(&mut self, block: &Block) {
        for stmt in &block.stmts {
            self.resolve_stmt(stmt);
        }
    }

    fn resolve_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let(let_stmt) => {
                // Resolve initializer first
                self.resolve_expr(&let_stmt.init);

                // Check for redefinition in same scope
                if self.env.is_defined_in_current_scope(&let_stmt.name) {
                    self.errors.push(SemanticError::new(
                        "RS005",
                        format!("Variable '{}' already defined in this scope", let_stmt.name),
                        let_stmt.span,
                    ));
                }

                // Determine type
                let ty = if let Some(ref type_ann) = let_stmt.ty {
                    ast_type_to_type_info(type_ann)
                } else {
                    // Type will be inferred during type checking
                    self.env.fresh_var()
                };

                self.env.define(let_stmt.name.clone(), ty);
            }

            Stmt::Const(const_stmt) => {
                self.resolve_expr(&const_stmt.init);

                if self.env.is_defined_in_current_scope(&const_stmt.name) {
                    self.errors.push(SemanticError::new(
                        "RS005",
                        format!("Constant '{}' already defined in this scope", const_stmt.name),
                        const_stmt.span,
                    ));
                }

                let ty = if let Some(ref type_ann) = const_stmt.ty {
                    ast_type_to_type_info(type_ann)
                } else {
                    self.env.fresh_var()
                };

                self.env.define(const_stmt.name.clone(), ty);
            }

            Stmt::Expr(expr_stmt) => {
                self.resolve_expr(&expr_stmt.expr);
            }

            Stmt::If(if_stmt) => {
                self.resolve_expr(&if_stmt.condition);
                self.env.push_scope();
                self.resolve_block(&if_stmt.then_branch);
                self.env.pop_scope();

                for (cond, block) in &if_stmt.else_if_branches {
                    self.resolve_expr(cond);
                    self.env.push_scope();
                    self.resolve_block(block);
                    self.env.pop_scope();
                }

                if let Some(ref else_block) = if_stmt.else_branch {
                    self.env.push_scope();
                    self.resolve_block(else_block);
                    self.env.pop_scope();
                }
            }

            Stmt::Match(match_stmt) => {
                self.resolve_expr(&match_stmt.scrutinee);
                for arm in &match_stmt.arms {
                    self.env.push_scope();
                    self.resolve_pattern(&arm.pattern);
                    self.resolve_expr(&arm.body);
                    self.env.pop_scope();
                }
            }

            Stmt::For(for_stmt) => {
                self.resolve_expr(&for_stmt.iter);
                self.env.push_scope();
                // Define loop variable
                let var_type = self.env.fresh_var();
                self.env.define(for_stmt.var.clone(), var_type);
                self.resolve_block(&for_stmt.body);
                self.env.pop_scope();
            }

            Stmt::While(while_stmt) => {
                self.resolve_expr(&while_stmt.condition);
                self.env.push_scope();
                self.resolve_block(&while_stmt.body);
                self.env.pop_scope();
            }

            Stmt::Loop(loop_stmt) => {
                self.env.push_scope();
                self.resolve_block(&loop_stmt.body);
                self.env.pop_scope();
            }

            Stmt::Return(return_stmt) => {
                if let Some(ref value) = return_stmt.value {
                    self.resolve_expr(value);
                }
            }

            Stmt::Break(_) | Stmt::Continue(_) => {}

            Stmt::Traverse(traverse_stmt) => {
                // Resolve the target expression
                self.resolve_expr(&traverse_stmt.target);

                // Handle the traverse kind
                match &traverse_stmt.kind {
                    crate::parser::TraverseKind::Inline(inline) => {
                        // Create a new scope for the inline visitor
                        self.env.push_scope();

                        // Resolve state variables
                        for let_stmt in &inline.state {
                            self.resolve_expr(&let_stmt.init);
                            let ty = if let Some(ref type_ann) = let_stmt.ty {
                                ast_type_to_type_info(type_ann)
                            } else {
                                self.env.fresh_var()
                            };
                            self.env.define(let_stmt.name.clone(), ty);
                        }

                        // Resolve methods
                        for method in &inline.methods {
                            self.resolve_function(method);
                        }

                        self.env.pop_scope();
                    }
                    crate::parser::TraverseKind::Delegated(visitor_name) => {
                        // Check if the visitor exists (would need to track plugin definitions)
                        // For now, just note it for later validation
                        let _ = visitor_name;
                    }
                }
            }
        }
    }

    fn resolve_pattern(&mut self, pattern: &Pattern) {
        match pattern {
            Pattern::Ident(name) => {
                // Bind the pattern variable
                let var_type = self.env.fresh_var();
                self.env.define(name.clone(), var_type);
            }
            Pattern::Struct { fields, .. } => {
                for (_, pat) in fields {
                    self.resolve_pattern(pat);
                }
            }
            Pattern::Or(patterns) => {
                for pat in patterns {
                    self.resolve_pattern(pat);
                }
            }
            Pattern::Variant { inner, .. } => {
                // Resolve inner pattern if present (e.g., Some(x) -> resolve x)
                if let Some(inner_pat) = inner {
                    self.resolve_pattern(inner_pat);
                }
            }
            Pattern::Literal(_) | Pattern::Wildcard => {}
        }
    }

    fn resolve_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Ident(ident) => {
                if self.env.lookup(&ident.name).is_none() {
                    // Check for special names and built-in macros
                    let is_special = matches!(ident.name.as_str(),
                        "self" | "Self" | "matches!" | "format!" | "vec!" | "Some" | "None" | "Ok" | "Err"
                    );
                    // Check if it's a known AST node type (used in matches!)
                    let is_ast_type = get_node_mapping(&ident.name).is_some();
                    if !is_special && !is_ast_type {
                        self.errors.push(SemanticError::new(
                            "RS006",
                            format!("Undefined variable: {}", ident.name),
                            ident.span,
                        ));
                    }
                }
            }

            Expr::Binary(binary) => {
                self.resolve_expr(&binary.left);
                self.resolve_expr(&binary.right);
            }

            Expr::Unary(unary) => {
                self.resolve_expr(&unary.operand);
            }

            Expr::Call(call) => {
                self.resolve_expr(&call.callee);
                for arg in &call.args {
                    self.resolve_expr(arg);
                }
            }

            Expr::Member(member) => {
                self.resolve_expr(&member.object);
                // Property name doesn't need resolution
            }

            Expr::Index(index) => {
                self.resolve_expr(&index.object);
                self.resolve_expr(&index.index);
            }

            Expr::StructInit(init) => {
                // Check struct exists
                if self.env.get_struct_fields(&init.name).is_none()
                   && self.env.lookup(&init.name).is_none() {
                    self.errors.push(SemanticError::new(
                        "RS007",
                        format!("Unknown struct: {}", init.name),
                        init.span,
                    ));
                }
                for (_, value) in &init.fields {
                    self.resolve_expr(value);
                }
            }

            Expr::VecInit(vec_init) => {
                for elem in &vec_init.elements {
                    self.resolve_expr(elem);
                }
            }

            Expr::If(if_expr) => {
                self.resolve_expr(&if_expr.condition);
                self.env.push_scope();
                self.resolve_block(&if_expr.then_branch);
                self.env.pop_scope();
                if let Some(ref else_block) = if_expr.else_branch {
                    self.env.push_scope();
                    self.resolve_block(else_block);
                    self.env.pop_scope();
                }
            }

            Expr::Match(match_expr) => {
                self.resolve_expr(&match_expr.scrutinee);
                for arm in &match_expr.arms {
                    self.env.push_scope();
                    self.resolve_pattern(&arm.pattern);
                    self.resolve_expr(&arm.body);
                    self.env.pop_scope();
                }
            }

            Expr::Closure(closure) => {
                self.env.push_scope();
                for param in &closure.params {
                    let var_type = self.env.fresh_var();
                    self.env.define(param.clone(), var_type);
                }
                self.resolve_expr(&closure.body);
                self.env.pop_scope();
            }

            Expr::Ref(ref_expr) => {
                self.resolve_expr(&ref_expr.expr);
            }

            Expr::Deref(deref_expr) => {
                self.resolve_expr(&deref_expr.expr);
            }

            Expr::Assign(assign) => {
                self.resolve_expr(&assign.target);
                self.resolve_expr(&assign.value);
            }

            Expr::CompoundAssign(compound) => {
                self.resolve_expr(&compound.target);
                self.resolve_expr(&compound.value);
            }

            Expr::Range(range) => {
                if let Some(ref start) = range.start {
                    self.resolve_expr(start);
                }
                if let Some(ref end) = range.end {
                    self.resolve_expr(end);
                }
            }

            Expr::Paren(inner) => {
                self.resolve_expr(inner);
            }

            Expr::Literal(_) => {}
        }
    }
}

impl Default for Resolver {
    fn default() -> Self {
        Self::new()
    }
}
