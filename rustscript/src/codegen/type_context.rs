//! Type context for type-aware SWC code generation
//!
//! This module implements flow-sensitive typing to handle the architectural
//! divergence between Babel's uniform Node hierarchy and SWC's strict
//! Enum/Struct hierarchy.

use std::collections::HashMap;

/// Classification of SWC types
#[derive(Clone, Debug, PartialEq)]
pub enum SwcTypeKind {
    /// Top-level enums that require pattern matching (Expr, Stmt, Decl, Pat)
    Enum,
    /// Direct structs with accessible fields (MemberExpr, Ident, CallExpr)
    Struct,
    /// Wrapper enums that wrap other types (MemberProp, PropName)
    WrapperEnum,
    /// Interned strings (JsWord, Atom)
    Atom,
    /// Primitive types (String, bool, i32)
    Primitive,
    /// Box<T> wrapper - treat as T but needs dereference
    Boxed(Box<SwcTypeKind>),
    /// Option<T> wrapper
    Optional(Box<SwcTypeKind>),
    /// Unknown type
    Unknown,
}

/// Type context for an expression or variable
#[derive(Clone, Debug)]
pub struct TypeContext {
    /// RustScript type name (e.g., "MemberExpression", "Identifier")
    pub rustscript_type: String,
    /// SWC type name (e.g., "MemberExpr", "Ident")
    pub swc_type: String,
    /// Classification of the SWC type
    pub kind: SwcTypeKind,
    /// For Enums: which variant are we known to be?
    /// e.g., if swc_type is "Expr" but known_variant is "Member",
    /// we can safely access MemberExpr fields
    pub known_variant: Option<String>,
    /// Whether this value needs Box dereference to access
    pub needs_deref: bool,
}

impl TypeContext {
    /// Create an unknown type context
    pub fn unknown() -> Self {
        Self {
            rustscript_type: "Unknown".into(),
            swc_type: "Unknown".into(),
            kind: SwcTypeKind::Unknown,
            known_variant: None,
            needs_deref: false,
        }
    }

    /// Create a type context for a known RustScript type
    pub fn from_rustscript(rs_type: &str) -> Self {
        let (swc_type, kind) = map_rustscript_to_swc(rs_type);
        Self {
            rustscript_type: rs_type.to_string(),
            swc_type,
            kind,
            known_variant: None,
            needs_deref: false,
        }
    }

    /// Create a narrowed type context (after pattern matching)
    pub fn narrowed(rs_type: &str, swc_struct: &str) -> Self {
        Self {
            rustscript_type: rs_type.to_string(),
            swc_type: swc_struct.to_string(),
            kind: SwcTypeKind::Struct,
            known_variant: None,
            needs_deref: false,
        }
    }

    /// Check if this type is boxed
    pub fn is_boxed(&self) -> bool {
        matches!(self.kind, SwcTypeKind::Boxed(_)) || self.needs_deref
    }

    /// Check if this type requires enum unwrapping
    pub fn needs_unwrap(&self) -> bool {
        matches!(self.kind, SwcTypeKind::Enum | SwcTypeKind::WrapperEnum)
            && self.known_variant.is_none()
    }

    /// Get the inner type if boxed
    pub fn unboxed(&self) -> Self {
        if let SwcTypeKind::Boxed(inner) = &self.kind {
            Self {
                rustscript_type: self.rustscript_type.clone(),
                swc_type: self.swc_type.trim_start_matches("Box<")
                    .trim_end_matches('>')
                    .to_string(),
                kind: (**inner).clone(),
                known_variant: self.known_variant.clone(),
                needs_deref: true,
            }
        } else {
            self.clone()
        }
    }
}

/// Type environment for tracking variable types through scopes
pub struct TypeEnvironment {
    /// Stack of scopes, each mapping variable names to types
    scopes: Vec<HashMap<String, TypeContext>>,
}

impl TypeEnvironment {
    /// Create a new type environment with global scope
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
        }
    }

    /// Push a new scope (for entering blocks, if/while let, etc.)
    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Pop the current scope
    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Define a variable in the current scope
    /// This implements variable shadowing - defining a variable with the same
    /// name in an inner scope shadows the outer definition
    pub fn define(&mut self, name: &str, ctx: TypeContext) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), ctx);
        }
    }

    /// Look up a variable's type, searching from innermost to outermost scope
    pub fn lookup(&self, name: &str) -> Option<&TypeContext> {
        for scope in self.scopes.iter().rev() {
            if let Some(ctx) = scope.get(name) {
                return Some(ctx);
            }
        }
        None
    }

    /// Check if a variable is defined in the current (innermost) scope
    pub fn is_defined_in_current_scope(&self, name: &str) -> bool {
        self.scopes.last()
            .map(|s| s.contains_key(name))
            .unwrap_or(false)
    }
}

impl Default for TypeEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

/// Map RustScript type name to SWC type name and kind
pub fn map_rustscript_to_swc(rs_type: &str) -> (String, SwcTypeKind) {
    match rs_type {
        // Expressions (Enum)
        "Expr" => ("Expr".into(), SwcTypeKind::Enum),
        "Expression" => ("Expr".into(), SwcTypeKind::Enum),

        // Statements (Enum)
        "Stmt" => ("Stmt".into(), SwcTypeKind::Enum),
        "Statement" => ("Stmt".into(), SwcTypeKind::Enum),

        // Declarations (Enum)
        "Decl" => ("Decl".into(), SwcTypeKind::Enum),
        "Declaration" => ("Decl".into(), SwcTypeKind::Enum),

        // Patterns (Enum)
        "Pat" => ("Pat".into(), SwcTypeKind::Enum),
        "Pattern" => ("Pat".into(), SwcTypeKind::Enum),

        // Literals (Enum)
        "Lit" => ("Lit".into(), SwcTypeKind::Enum),
        "Literal" => ("Lit".into(), SwcTypeKind::Enum),

        // Specific expression types (Struct after unwrapping)
        "MemberExpression" => ("MemberExpr".into(), SwcTypeKind::Struct),
        "CallExpression" => ("CallExpr".into(), SwcTypeKind::Struct),
        "Identifier" => ("Ident".into(), SwcTypeKind::Struct),
        "BinaryExpression" => ("BinExpr".into(), SwcTypeKind::Struct),
        "UnaryExpression" => ("UnaryExpr".into(), SwcTypeKind::Struct),
        "AssignmentExpression" => ("AssignExpr".into(), SwcTypeKind::Struct),
        "ArrayExpression" => ("ArrayLit".into(), SwcTypeKind::Struct),
        "ObjectExpression" => ("ObjectLit".into(), SwcTypeKind::Struct),
        "FunctionExpression" => ("FnExpr".into(), SwcTypeKind::Struct),
        "ArrowFunctionExpression" => ("ArrowExpr".into(), SwcTypeKind::Struct),

        // Specific statement types (Struct after unwrapping)
        "BlockStatement" => ("BlockStmt".into(), SwcTypeKind::Struct),
        "ReturnStatement" => ("ReturnStmt".into(), SwcTypeKind::Struct),
        "IfStatement" => ("IfStmt".into(), SwcTypeKind::Struct),
        "WhileStatement" => ("WhileStmt".into(), SwcTypeKind::Struct),
        "ForStatement" => ("ForStmt".into(), SwcTypeKind::Struct),
        "ExpressionStatement" => ("ExprStmt".into(), SwcTypeKind::Struct),

        // Specific declaration types (Struct after unwrapping)
        "FunctionDeclaration" => ("FnDecl".into(), SwcTypeKind::Struct),
        "VariableDeclaration" => ("VarDecl".into(), SwcTypeKind::Struct),
        "ClassDeclaration" => ("ClassDecl".into(), SwcTypeKind::Struct),

        // Wrapper enums (special handling needed)
        "MemberProp" => ("MemberProp".into(), SwcTypeKind::WrapperEnum),
        "PropName" => ("PropName".into(), SwcTypeKind::WrapperEnum),
        "Callee" => ("Callee".into(), SwcTypeKind::WrapperEnum),

        // Atom types
        "JsWord" => ("JsWord".into(), SwcTypeKind::Atom),
        "Atom" => ("Atom".into(), SwcTypeKind::Atom),
        "Str" => ("String".into(), SwcTypeKind::Primitive),

        // Primitives
        "bool" => ("bool".into(), SwcTypeKind::Primitive),
        "i32" => ("i32".into(), SwcTypeKind::Primitive),
        "u32" => ("u32".into(), SwcTypeKind::Primitive),
        "f64" => ("f64".into(), SwcTypeKind::Primitive),
        "String" => ("String".into(), SwcTypeKind::Primitive),

        // Unknown
        _ => (rs_type.to_string(), SwcTypeKind::Unknown),
    }
}

/// Classify an SWC type name
pub fn classify_swc_type(type_name: &str) -> SwcTypeKind {
    match type_name {
        // Top-level enums
        "Expr" | "Stmt" | "Decl" | "Pat" | "Lit" | "ModuleItem" => SwcTypeKind::Enum,

        // Wrapper enums
        "MemberProp" | "PropName" | "JSXObject" | "Callee" => SwcTypeKind::WrapperEnum,

        // Structs
        "Ident" | "MemberExpr" | "CallExpr" | "FnDecl" | "BinExpr" |
        "BlockStmt" | "ReturnStmt" | "IfStmt" | "WhileStmt" | "ForStmt" |
        "VarDecl" | "ClassDecl" | "FnExpr" | "ArrowExpr" | "AssignExpr" |
        "ArrayLit" | "ObjectLit" | "ExprStmt" | "UnaryExpr" => SwcTypeKind::Struct,

        // Atoms
        "JsWord" | "Atom" => SwcTypeKind::Atom,

        // Box<T>
        s if s.starts_with("Box<") => {
            let inner = &s[4..s.len() - 1];
            SwcTypeKind::Boxed(Box::new(classify_swc_type(inner)))
        }

        // Option<T>
        s if s.starts_with("Option<") => {
            let inner = &s[7..s.len() - 1];
            SwcTypeKind::Optional(Box::new(classify_swc_type(inner)))
        }

        // Primitives
        "i32" | "f64" | "bool" | "String" | "usize" => SwcTypeKind::Primitive,

        _ => SwcTypeKind::Unknown,
    }
}

/// Get the SWC enum and variant for a RustScript type
/// Returns (enum_name, variant_name, struct_name)
pub fn get_swc_variant(rs_type: &str) -> (String, String, String) {
    match rs_type {
        // Expressions
        "MemberExpression" => ("Expr".into(), "Member".into(), "MemberExpr".into()),
        "CallExpression" => ("Expr".into(), "Call".into(), "CallExpr".into()),
        "Identifier" => ("Expr".into(), "Ident".into(), "Ident".into()),
        "BinaryExpression" => ("Expr".into(), "Bin".into(), "BinExpr".into()),
        "UnaryExpression" => ("Expr".into(), "Unary".into(), "UnaryExpr".into()),
        "AssignmentExpression" => ("Expr".into(), "Assign".into(), "AssignExpr".into()),
        "ArrayExpression" => ("Expr".into(), "Array".into(), "ArrayLit".into()),
        "ObjectExpression" => ("Expr".into(), "Object".into(), "ObjectLit".into()),
        "FunctionExpression" => ("Expr".into(), "Fn".into(), "FnExpr".into()),
        "ArrowFunctionExpression" => ("Expr".into(), "Arrow".into(), "ArrowExpr".into()),
        "StringLiteral" => ("Lit".into(), "Str".into(), "Str".into()),
        "NumericLiteral" => ("Lit".into(), "Num".into(), "Number".into()),
        "BooleanLiteral" => ("Lit".into(), "Bool".into(), "Bool".into()),

        // Statements
        "BlockStatement" => ("Stmt".into(), "Block".into(), "BlockStmt".into()),
        "ReturnStatement" => ("Stmt".into(), "Return".into(), "ReturnStmt".into()),
        "IfStatement" => ("Stmt".into(), "If".into(), "IfStmt".into()),
        "WhileStatement" => ("Stmt".into(), "While".into(), "WhileStmt".into()),
        "ForStatement" => ("Stmt".into(), "For".into(), "ForStmt".into()),
        "ExpressionStatement" => ("Stmt".into(), "Expr".into(), "ExprStmt".into()),

        // Declarations
        "FunctionDeclaration" => ("Decl".into(), "Fn".into(), "FnDecl".into()),
        "VariableDeclaration" => ("Decl".into(), "Var".into(), "VarDecl".into()),
        "ClassDeclaration" => ("Decl".into(), "Class".into(), "ClassDecl".into()),

        _ => ("Unknown".into(), rs_type.to_string(), rs_type.to_string()),
    }
}

/// Field mapping with type information
#[derive(Clone, Debug)]
pub struct TypedFieldMapping {
    pub rustscript_field: &'static str,
    pub swc_field: &'static str,
    pub needs_deref: bool,
    pub result_type_rs: &'static str,
    pub result_type_swc: &'static str,
    pub read_conversion: &'static str,  // e.g., ".to_string()"
    pub write_conversion: &'static str, // e.g., ".into()"
}

/// Get field mapping for a parent type and field name
pub fn get_typed_field_mapping(parent_swc_type: &str, field: &str) -> Option<TypedFieldMapping> {
    match (parent_swc_type, field) {
        // MemberExpr fields
        ("MemberExpr", "object") => Some(TypedFieldMapping {
            rustscript_field: "object",
            swc_field: "obj",
            needs_deref: true, // Box<Expr>
            result_type_rs: "Expr",
            result_type_swc: "Expr",
            read_conversion: "",
            write_conversion: "",
        }),
        ("MemberExpr", "property") => Some(TypedFieldMapping {
            rustscript_field: "property",
            swc_field: "prop",
            needs_deref: false,
            result_type_rs: "MemberProp",
            result_type_swc: "MemberProp", // WrapperEnum!
            read_conversion: "",
            write_conversion: "",
        }),

        // CallExpr fields
        ("CallExpr", "callee") => Some(TypedFieldMapping {
            rustscript_field: "callee",
            swc_field: "callee",
            needs_deref: false,
            result_type_rs: "Callee",
            result_type_swc: "Callee",
            read_conversion: "",
            write_conversion: "",
        }),
        ("CallExpr", "arguments") => Some(TypedFieldMapping {
            rustscript_field: "arguments",
            swc_field: "args",
            needs_deref: false,
            result_type_rs: "Vec<ExprOrSpread>",
            result_type_swc: "Vec<ExprOrSpread>",
            read_conversion: "",
            write_conversion: "",
        }),

        // Ident fields
        ("Ident", "name") => Some(TypedFieldMapping {
            rustscript_field: "name",
            swc_field: "sym",
            needs_deref: false,
            result_type_rs: "Str",
            result_type_swc: "JsWord",
            read_conversion: ".to_string()",
            write_conversion: ".into()",
        }),

        // BlockStmt fields
        ("BlockStmt", "body") | ("BlockStmt", "stmts") => Some(TypedFieldMapping {
            rustscript_field: "body",
            swc_field: "stmts",
            needs_deref: false,
            result_type_rs: "Vec<Stmt>",
            result_type_swc: "Vec<Stmt>",
            read_conversion: "",
            write_conversion: "",
        }),

        // ReturnStmt fields
        ("ReturnStmt", "argument") => Some(TypedFieldMapping {
            rustscript_field: "argument",
            swc_field: "arg",
            needs_deref: false,
            result_type_rs: "Option<Expr>",
            result_type_swc: "Option<Box<Expr>>",
            read_conversion: "",
            write_conversion: "",
        }),

        // FnDecl fields
        ("FnDecl", "id") => Some(TypedFieldMapping {
            rustscript_field: "id",
            swc_field: "ident",
            needs_deref: false,
            result_type_rs: "Identifier",
            result_type_swc: "Ident",
            read_conversion: "",
            write_conversion: "",
        }),
        ("FnDecl", "params") => Some(TypedFieldMapping {
            rustscript_field: "params",
            swc_field: "function.params",
            needs_deref: false,
            result_type_rs: "Vec<Pat>",
            result_type_swc: "Vec<Param>",
            read_conversion: "",
            write_conversion: "",
        }),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_environment_shadowing() {
        let mut env = TypeEnvironment::new();

        // Define outer variable
        env.define("current", TypeContext::from_rustscript("Expr"));
        assert_eq!(env.lookup("current").unwrap().swc_type, "Expr");

        // Push scope and shadow with narrowed type
        env.push_scope();
        env.define("current", TypeContext::narrowed("MemberExpression", "MemberExpr"));
        assert_eq!(env.lookup("current").unwrap().swc_type, "MemberExpr");

        // Pop scope, should see outer type again
        env.pop_scope();
        assert_eq!(env.lookup("current").unwrap().swc_type, "Expr");
    }

    #[test]
    fn test_swc_variant_mapping() {
        let (enum_name, variant, struct_name) = get_swc_variant("MemberExpression");
        assert_eq!(enum_name, "Expr");
        assert_eq!(variant, "Member");
        assert_eq!(struct_name, "MemberExpr");
    }

    #[test]
    fn test_classify_swc_type() {
        assert!(matches!(classify_swc_type("Expr"), SwcTypeKind::Enum));
        assert!(matches!(classify_swc_type("MemberExpr"), SwcTypeKind::Struct));
        assert!(matches!(classify_swc_type("MemberProp"), SwcTypeKind::WrapperEnum));
        assert!(matches!(classify_swc_type("JsWord"), SwcTypeKind::Atom));
    }
}
