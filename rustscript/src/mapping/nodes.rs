//! Node type mappings between RustScript, Babel, and SWC

use std::collections::HashMap;
use once_cell::sync::Lazy;

/// Mapping for a single AST node type
#[derive(Debug, Clone)]
pub struct NodeMapping {
    /// RustScript unified name
    pub rustscript: &'static str,
    /// Babel/ESTree type name
    pub babel: &'static str,
    /// SWC Rust type name
    pub swc: &'static str,
    /// SWC enum variant (if wrapped in enum like Stmt, Expr, Decl)
    pub swc_enum: Option<&'static str>,
    /// Babel type checker function (e.g., "isIdentifier")
    pub babel_checker: &'static str,
    /// SWC pattern match (e.g., "Expr::Ident")
    pub swc_pattern: &'static str,
    /// Visitor method name in RustScript
    pub visitor_method: &'static str,
    /// SWC visitor method name
    pub swc_visitor: &'static str,
}

/// All node type mappings
pub static NODE_MAPPINGS: Lazy<Vec<NodeMapping>> = Lazy::new(|| vec![
    // === Declarations ===
    NodeMapping {
        rustscript: "FunctionDeclaration",
        babel: "FunctionDeclaration",
        swc: "FnDecl",
        swc_enum: Some("Decl::Fn"),
        babel_checker: "isFunctionDeclaration",
        swc_pattern: "Decl::Fn(fn_decl)",
        visitor_method: "visit_function_declaration",
        swc_visitor: "visit_mut_fn_decl",
    },
    NodeMapping {
        rustscript: "VariableDeclaration",
        babel: "VariableDeclaration",
        swc: "VarDecl",
        swc_enum: Some("Decl::Var"),
        babel_checker: "isVariableDeclaration",
        swc_pattern: "Decl::Var(var_decl)",
        visitor_method: "visit_variable_declaration",
        swc_visitor: "visit_mut_var_decl",
    },
    NodeMapping {
        rustscript: "ClassDeclaration",
        babel: "ClassDeclaration",
        swc: "ClassDecl",
        swc_enum: Some("Decl::Class"),
        babel_checker: "isClassDeclaration",
        swc_pattern: "Decl::Class(class_decl)",
        visitor_method: "visit_class_declaration",
        swc_visitor: "visit_mut_class_decl",
    },

    // === Statements ===
    NodeMapping {
        rustscript: "ExpressionStatement",
        babel: "ExpressionStatement",
        swc: "ExprStmt",
        swc_enum: Some("Stmt::Expr"),
        babel_checker: "isExpressionStatement",
        swc_pattern: "Stmt::Expr(expr_stmt)",
        visitor_method: "visit_expression_statement",
        swc_visitor: "visit_mut_expr_stmt",
    },
    NodeMapping {
        rustscript: "BlockStatement",
        babel: "BlockStatement",
        swc: "BlockStmt",
        swc_enum: Some("Stmt::Block"),
        babel_checker: "isBlockStatement",
        swc_pattern: "Stmt::Block(block_stmt)",
        visitor_method: "visit_block_statement",
        swc_visitor: "visit_mut_block_stmt",
    },
    NodeMapping {
        rustscript: "ReturnStatement",
        babel: "ReturnStatement",
        swc: "ReturnStmt",
        swc_enum: Some("Stmt::Return"),
        babel_checker: "isReturnStatement",
        swc_pattern: "Stmt::Return(return_stmt)",
        visitor_method: "visit_return_statement",
        swc_visitor: "visit_mut_return_stmt",
    },
    NodeMapping {
        rustscript: "IfStatement",
        babel: "IfStatement",
        swc: "IfStmt",
        swc_enum: Some("Stmt::If"),
        babel_checker: "isIfStatement",
        swc_pattern: "Stmt::If(if_stmt)",
        visitor_method: "visit_if_statement",
        swc_visitor: "visit_mut_if_stmt",
    },
    NodeMapping {
        rustscript: "ForStatement",
        babel: "ForStatement",
        swc: "ForStmt",
        swc_enum: Some("Stmt::For"),
        babel_checker: "isForStatement",
        swc_pattern: "Stmt::For(for_stmt)",
        visitor_method: "visit_for_statement",
        swc_visitor: "visit_mut_for_stmt",
    },
    NodeMapping {
        rustscript: "ForInStatement",
        babel: "ForInStatement",
        swc: "ForInStmt",
        swc_enum: Some("Stmt::ForIn"),
        babel_checker: "isForInStatement",
        swc_pattern: "Stmt::ForIn(for_in_stmt)",
        visitor_method: "visit_for_in_statement",
        swc_visitor: "visit_mut_for_in_stmt",
    },
    NodeMapping {
        rustscript: "ForOfStatement",
        babel: "ForOfStatement",
        swc: "ForOfStmt",
        swc_enum: Some("Stmt::ForOf"),
        babel_checker: "isForOfStatement",
        swc_pattern: "Stmt::ForOf(for_of_stmt)",
        visitor_method: "visit_for_of_statement",
        swc_visitor: "visit_mut_for_of_stmt",
    },
    NodeMapping {
        rustscript: "WhileStatement",
        babel: "WhileStatement",
        swc: "WhileStmt",
        swc_enum: Some("Stmt::While"),
        babel_checker: "isWhileStatement",
        swc_pattern: "Stmt::While(while_stmt)",
        visitor_method: "visit_while_statement",
        swc_visitor: "visit_mut_while_stmt",
    },
    NodeMapping {
        rustscript: "SwitchStatement",
        babel: "SwitchStatement",
        swc: "SwitchStmt",
        swc_enum: Some("Stmt::Switch"),
        babel_checker: "isSwitchStatement",
        swc_pattern: "Stmt::Switch(switch_stmt)",
        visitor_method: "visit_switch_statement",
        swc_visitor: "visit_mut_switch_stmt",
    },
    NodeMapping {
        rustscript: "TryStatement",
        babel: "TryStatement",
        swc: "TryStmt",
        swc_enum: Some("Stmt::Try"),
        babel_checker: "isTryStatement",
        swc_pattern: "Stmt::Try(try_stmt)",
        visitor_method: "visit_try_statement",
        swc_visitor: "visit_mut_try_stmt",
    },
    NodeMapping {
        rustscript: "ThrowStatement",
        babel: "ThrowStatement",
        swc: "ThrowStmt",
        swc_enum: Some("Stmt::Throw"),
        babel_checker: "isThrowStatement",
        swc_pattern: "Stmt::Throw(throw_stmt)",
        visitor_method: "visit_throw_statement",
        swc_visitor: "visit_mut_throw_stmt",
    },

    // === Expressions ===
    NodeMapping {
        rustscript: "Identifier",
        babel: "Identifier",
        swc: "Ident",
        swc_enum: Some("Expr::Ident"),
        babel_checker: "isIdentifier",
        swc_pattern: "Expr::Ident(ident)",
        visitor_method: "visit_identifier",
        swc_visitor: "visit_mut_ident",
    },
    NodeMapping {
        rustscript: "CallExpression",
        babel: "CallExpression",
        swc: "CallExpr",
        swc_enum: Some("Expr::Call"),
        babel_checker: "isCallExpression",
        swc_pattern: "Expr::Call(call_expr)",
        visitor_method: "visit_call_expression",
        swc_visitor: "visit_mut_call_expr",
    },
    NodeMapping {
        rustscript: "MemberExpression",
        babel: "MemberExpression",
        swc: "MemberExpr",
        swc_enum: Some("Expr::Member"),
        babel_checker: "isMemberExpression",
        swc_pattern: "Expr::Member(member_expr)",
        visitor_method: "visit_member_expression",
        swc_visitor: "visit_mut_member_expr",
    },
    NodeMapping {
        rustscript: "BinaryExpression",
        babel: "BinaryExpression",
        swc: "BinExpr",
        swc_enum: Some("Expr::Bin"),
        babel_checker: "isBinaryExpression",
        swc_pattern: "Expr::Bin(bin_expr)",
        visitor_method: "visit_binary_expression",
        swc_visitor: "visit_mut_bin_expr",
    },
    NodeMapping {
        rustscript: "UnaryExpression",
        babel: "UnaryExpression",
        swc: "UnaryExpr",
        swc_enum: Some("Expr::Unary"),
        babel_checker: "isUnaryExpression",
        swc_pattern: "Expr::Unary(unary_expr)",
        visitor_method: "visit_unary_expression",
        swc_visitor: "visit_mut_unary_expr",
    },
    NodeMapping {
        rustscript: "AssignmentExpression",
        babel: "AssignmentExpression",
        swc: "AssignExpr",
        swc_enum: Some("Expr::Assign"),
        babel_checker: "isAssignmentExpression",
        swc_pattern: "Expr::Assign(assign_expr)",
        visitor_method: "visit_assignment_expression",
        swc_visitor: "visit_mut_assign_expr",
    },
    NodeMapping {
        rustscript: "ConditionalExpression",
        babel: "ConditionalExpression",
        swc: "CondExpr",
        swc_enum: Some("Expr::Cond"),
        babel_checker: "isConditionalExpression",
        swc_pattern: "Expr::Cond(cond_expr)",
        visitor_method: "visit_conditional_expression",
        swc_visitor: "visit_mut_cond_expr",
    },
    NodeMapping {
        rustscript: "LogicalExpression",
        babel: "LogicalExpression",
        swc: "BinExpr",
        swc_enum: Some("Expr::Bin"),
        babel_checker: "isLogicalExpression",
        swc_pattern: "Expr::Bin(bin_expr)",
        visitor_method: "visit_logical_expression",
        swc_visitor: "visit_mut_bin_expr",
    },
    NodeMapping {
        rustscript: "ArrayExpression",
        babel: "ArrayExpression",
        swc: "ArrayLit",
        swc_enum: Some("Expr::Array"),
        babel_checker: "isArrayExpression",
        swc_pattern: "Expr::Array(array_lit)",
        visitor_method: "visit_array_expression",
        swc_visitor: "visit_mut_array_lit",
    },
    NodeMapping {
        rustscript: "ObjectExpression",
        babel: "ObjectExpression",
        swc: "ObjectLit",
        swc_enum: Some("Expr::Object"),
        babel_checker: "isObjectExpression",
        swc_pattern: "Expr::Object(object_lit)",
        visitor_method: "visit_object_expression",
        swc_visitor: "visit_mut_object_lit",
    },
    NodeMapping {
        rustscript: "ArrowFunctionExpression",
        babel: "ArrowFunctionExpression",
        swc: "ArrowExpr",
        swc_enum: Some("Expr::Arrow"),
        babel_checker: "isArrowFunctionExpression",
        swc_pattern: "Expr::Arrow(arrow_expr)",
        visitor_method: "visit_arrow_function_expression",
        swc_visitor: "visit_mut_arrow_expr",
    },
    NodeMapping {
        rustscript: "FunctionExpression",
        babel: "FunctionExpression",
        swc: "FnExpr",
        swc_enum: Some("Expr::Fn"),
        babel_checker: "isFunctionExpression",
        swc_pattern: "Expr::Fn(fn_expr)",
        visitor_method: "visit_function_expression",
        swc_visitor: "visit_mut_fn_expr",
    },
    NodeMapping {
        rustscript: "NewExpression",
        babel: "NewExpression",
        swc: "NewExpr",
        swc_enum: Some("Expr::New"),
        babel_checker: "isNewExpression",
        swc_pattern: "Expr::New(new_expr)",
        visitor_method: "visit_new_expression",
        swc_visitor: "visit_mut_new_expr",
    },
    NodeMapping {
        rustscript: "SequenceExpression",
        babel: "SequenceExpression",
        swc: "SeqExpr",
        swc_enum: Some("Expr::Seq"),
        babel_checker: "isSequenceExpression",
        swc_pattern: "Expr::Seq(seq_expr)",
        visitor_method: "visit_sequence_expression",
        swc_visitor: "visit_mut_seq_expr",
    },
    NodeMapping {
        rustscript: "ThisExpression",
        babel: "ThisExpression",
        swc: "ThisExpr",
        swc_enum: Some("Expr::This"),
        babel_checker: "isThisExpression",
        swc_pattern: "Expr::This(this_expr)",
        visitor_method: "visit_this_expression",
        swc_visitor: "visit_mut_this_expr",
    },
    NodeMapping {
        rustscript: "AwaitExpression",
        babel: "AwaitExpression",
        swc: "AwaitExpr",
        swc_enum: Some("Expr::Await"),
        babel_checker: "isAwaitExpression",
        swc_pattern: "Expr::Await(await_expr)",
        visitor_method: "visit_await_expression",
        swc_visitor: "visit_mut_await_expr",
    },
    NodeMapping {
        rustscript: "YieldExpression",
        babel: "YieldExpression",
        swc: "YieldExpr",
        swc_enum: Some("Expr::Yield"),
        babel_checker: "isYieldExpression",
        swc_pattern: "Expr::Yield(yield_expr)",
        visitor_method: "visit_yield_expression",
        swc_visitor: "visit_mut_yield_expr",
    },

    // === Literals ===
    NodeMapping {
        rustscript: "StringLiteral",
        babel: "StringLiteral",
        swc: "Str",
        swc_enum: Some("Lit::Str"),
        babel_checker: "isStringLiteral",
        swc_pattern: "Lit::Str(str_lit)",
        visitor_method: "visit_string_literal",
        swc_visitor: "visit_mut_str",
    },
    NodeMapping {
        rustscript: "NumericLiteral",
        babel: "NumericLiteral",
        swc: "Number",
        swc_enum: Some("Lit::Num"),
        babel_checker: "isNumericLiteral",
        swc_pattern: "Lit::Num(num_lit)",
        visitor_method: "visit_numeric_literal",
        swc_visitor: "visit_mut_number",
    },
    NodeMapping {
        rustscript: "BooleanLiteral",
        babel: "BooleanLiteral",
        swc: "Bool",
        swc_enum: Some("Lit::Bool"),
        babel_checker: "isBooleanLiteral",
        swc_pattern: "Lit::Bool(bool_lit)",
        visitor_method: "visit_boolean_literal",
        swc_visitor: "visit_mut_bool",
    },
    NodeMapping {
        rustscript: "NullLiteral",
        babel: "NullLiteral",
        swc: "Null",
        swc_enum: Some("Lit::Null"),
        babel_checker: "isNullLiteral",
        swc_pattern: "Lit::Null(null_lit)",
        visitor_method: "visit_null_literal",
        swc_visitor: "visit_mut_null",
    },
    NodeMapping {
        rustscript: "RegExpLiteral",
        babel: "RegExpLiteral",
        swc: "Regex",
        swc_enum: Some("Lit::Regex"),
        babel_checker: "isRegExpLiteral",
        swc_pattern: "Lit::Regex(regex_lit)",
        visitor_method: "visit_regexp_literal",
        swc_visitor: "visit_mut_regex",
    },
    NodeMapping {
        rustscript: "TemplateLiteral",
        babel: "TemplateLiteral",
        swc: "Tpl",
        swc_enum: Some("Expr::Tpl"),
        babel_checker: "isTemplateLiteral",
        swc_pattern: "Expr::Tpl(tpl)",
        visitor_method: "visit_template_literal",
        swc_visitor: "visit_mut_tpl",
    },

    // === JSX ===
    NodeMapping {
        rustscript: "JSXElement",
        babel: "JSXElement",
        swc: "JSXElement",
        swc_enum: Some("Expr::JSXElement"),
        babel_checker: "isJSXElement",
        swc_pattern: "Expr::JSXElement(jsx_element)",
        visitor_method: "visit_jsx_element",
        swc_visitor: "visit_mut_jsx_element",
    },
    NodeMapping {
        rustscript: "JSXFragment",
        babel: "JSXFragment",
        swc: "JSXFragment",
        swc_enum: Some("Expr::JSXFragment"),
        babel_checker: "isJSXFragment",
        swc_pattern: "Expr::JSXFragment(jsx_fragment)",
        visitor_method: "visit_jsx_fragment",
        swc_visitor: "visit_mut_jsx_fragment",
    },
    NodeMapping {
        rustscript: "JSXAttribute",
        babel: "JSXAttribute",
        swc: "JSXAttr",
        swc_enum: None,
        babel_checker: "isJSXAttribute",
        swc_pattern: "JSXAttrOrSpread::JSXAttr(jsx_attr)",
        visitor_method: "visit_jsx_attribute",
        swc_visitor: "visit_mut_jsx_attr",
    },
    NodeMapping {
        rustscript: "JSXExpressionContainer",
        babel: "JSXExpressionContainer",
        swc: "JSXExprContainer",
        swc_enum: None,
        babel_checker: "isJSXExpressionContainer",
        swc_pattern: "JSXElementChild::JSXExprContainer(container)",
        visitor_method: "visit_jsx_expression_container",
        swc_visitor: "visit_mut_jsx_expr_container",
    },
    NodeMapping {
        rustscript: "JSXText",
        babel: "JSXText",
        swc: "JSXText",
        swc_enum: None,
        babel_checker: "isJSXText",
        swc_pattern: "JSXElementChild::JSXText(jsx_text)",
        visitor_method: "visit_jsx_text",
        swc_visitor: "visit_mut_jsx_text",
    },
    NodeMapping {
        rustscript: "JSXOpeningElement",
        babel: "JSXOpeningElement",
        swc: "JSXOpeningElement",
        swc_enum: None,
        babel_checker: "isJSXOpeningElement",
        swc_pattern: "JSXOpeningElement",
        visitor_method: "visit_jsx_opening_element",
        swc_visitor: "visit_mut_jsx_opening_element",
    },

    // === Module ===
    NodeMapping {
        rustscript: "ImportDeclaration",
        babel: "ImportDeclaration",
        swc: "ImportDecl",
        swc_enum: Some("ModuleDecl::Import"),
        babel_checker: "isImportDeclaration",
        swc_pattern: "ModuleDecl::Import(import_decl)",
        visitor_method: "visit_import_declaration",
        swc_visitor: "visit_mut_import_decl",
    },
    NodeMapping {
        rustscript: "ExportNamedDeclaration",
        babel: "ExportNamedDeclaration",
        swc: "ExportDecl",
        swc_enum: Some("ModuleDecl::ExportDecl"),
        babel_checker: "isExportNamedDeclaration",
        swc_pattern: "ModuleDecl::ExportDecl(export_decl)",
        visitor_method: "visit_export_named_declaration",
        swc_visitor: "visit_mut_export_decl",
    },
    NodeMapping {
        rustscript: "ExportDefaultDeclaration",
        babel: "ExportDefaultDeclaration",
        swc: "ExportDefaultDecl",
        swc_enum: Some("ModuleDecl::ExportDefaultDecl"),
        babel_checker: "isExportDefaultDeclaration",
        swc_pattern: "ModuleDecl::ExportDefaultDecl(export_default_decl)",
        visitor_method: "visit_export_default_declaration",
        swc_visitor: "visit_mut_export_default_decl",
    },

    // === Program ===
    NodeMapping {
        rustscript: "Program",
        babel: "Program",
        swc: "Program",
        swc_enum: None,
        babel_checker: "isProgram",
        swc_pattern: "Program",
        visitor_method: "visit_program",
        swc_visitor: "visit_mut_program",
    },
]);

/// Index for fast lookup by RustScript name
pub static NODE_MAP: Lazy<HashMap<&'static str, &'static NodeMapping>> = Lazy::new(|| {
    NODE_MAPPINGS
        .iter()
        .map(|m| (m.rustscript, m))
        .collect()
});

/// Get node mapping by RustScript name
pub fn get_node_mapping(rustscript_name: &str) -> Option<&'static NodeMapping> {
    NODE_MAP.get(rustscript_name).copied()
}

/// Get node mapping by visitor method name
pub fn get_node_mapping_by_visitor(visitor_method: &str) -> Option<&'static NodeMapping> {
    NODE_MAPPINGS
        .iter()
        .find(|m| m.visitor_method == visitor_method)
}

/// Get SWC type from RustScript type
pub fn rustscript_to_swc(rustscript_name: &str) -> String {
    get_node_mapping(rustscript_name)
        .map(|m| m.swc.to_string())
        .unwrap_or_else(|| rustscript_name.to_string())
}

/// Get Babel type from RustScript type
pub fn rustscript_to_babel(rustscript_name: &str) -> String {
    get_node_mapping(rustscript_name)
        .map(|m| m.babel.to_string())
        .unwrap_or_else(|| rustscript_name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_node_mapping() {
        let mapping = get_node_mapping("Identifier").unwrap();
        assert_eq!(mapping.swc, "Ident");
        assert_eq!(mapping.babel, "Identifier");
    }

    #[test]
    fn test_function_declaration_mapping() {
        let mapping = get_node_mapping("FunctionDeclaration").unwrap();
        assert_eq!(mapping.swc, "FnDecl");
        assert_eq!(mapping.swc_visitor, "visit_mut_fn_decl");
    }
}
