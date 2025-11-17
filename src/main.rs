use swc_common::{sync::Lrc, FileName, SourceMap};
use swc_ecma_ast::*;
use swc_ecma_parser::{lexer::Lexer, Parser, Syntax, TsSyntax};
use swc_ecma_visit::{Visit, VisitWith};

use std::fs;
use std::path::{Path, PathBuf};

/// Resolve a relative module path to an absolute path
fn resolve_module_path(current_file: &str, module_path: &str) -> String {
    let current_path = Path::new(current_file);
    let current_dir = current_path.parent().unwrap_or(Path::new("."));

    let mut resolved = current_dir.join(module_path);

    // Normalize the path (handle ../ and ./)
    resolved = resolved.canonicalize().unwrap_or(resolved);

    resolved.to_string_lossy().to_string()
}

/// Recursively parse required modules (like browserify)
fn parse_required_modules(analyzer: &mut BabelVisitorDetector, processed_files: &mut std::collections::HashSet<String>) {
    // Get list of modules to process (clone to avoid borrow issues)
    let modules_to_process: Vec<_> = analyzer.required_modules.iter()
        .filter(|m| !processed_files.contains(&m.resolved_path))
        .map(|m| m.resolved_path.clone())
        .collect();

    for module_path in modules_to_process {
        eprintln!("Parsing required module: {}", module_path);
        processed_files.insert(module_path.clone());

        // Parse the module file
        if let Ok(code) = fs::read_to_string(&module_path) {
            let cm = Lrc::new(SourceMap::default());
            let fm = cm.new_source_file(Lrc::new(FileName::Custom(module_path.clone().into())), code);

            let syntax = Syntax::Typescript(TsSyntax {
                tsx: true,
                ..Default::default()
            });

            let lexer = Lexer::new(syntax, Default::default(), (&*fm).into(), None);
            let mut parser = Parser::new_from(lexer);

            if let Ok(module) = parser.parse_module() {
                // Create a new analyzer for this module
                let mut module_analyzer = BabelVisitorDetector {
                    current_file_path: module_path.clone(),
                    ..Default::default()
                };
                module.visit_with(&mut module_analyzer);

                // Merge helper functions into main analyzer
                analyzer.helper_functions.extend(module_analyzer.helper_functions);

                // Merge required modules
                analyzer.required_modules.extend(module_analyzer.required_modules);

                // Recursively process new requirements
                parse_required_modules(analyzer, processed_files);
            } else {
                eprintln!("Failed to parse module: {}", module_path);
            }
        } else {
            eprintln!("Failed to read module: {}", module_path);
        }
    }
}

fn main() {
    // Read the babel plugin - can switch between simple and full
    let args: Vec<String> = std::env::args().collect();

    let mut plugin_path = "example/simple-babel-plugin/index.js";
    let mut json_output = false;

    // Parse arguments
    for (i, arg) in args.iter().enumerate() {
        match arg.as_str() {
            "--full" => plugin_path = "example/babel-plugin-minimact/index.cjs",
            "--json" => json_output = true,
            "--analyze" => {
                // Next arg is the file to analyze
                if i + 1 < args.len() {
                    plugin_path = &args[i + 1];
                }
            }
            _ => {}
        }
    }

    if !json_output {
        println!("Analyzing: {}", plugin_path);
    }

    let code = fs::read_to_string(plugin_path)
        .expect(&format!("Failed to read {}", plugin_path));

    let cm = Lrc::new(SourceMap::default());

    let fm = cm.new_source_file(Lrc::new(FileName::Custom("plugin.js".into())), code);

    let syntax = Syntax::Typescript(TsSyntax {
        tsx: true,
        ..Default::default()
    });

    let lexer = Lexer::new(syntax, Default::default(), (&*fm).into(), None);
    let mut parser = Parser::new_from(lexer);
    let module = parser.parse_module().expect("Parse failed");

    let mut analyzer = BabelVisitorDetector {
        current_file_path: plugin_path.to_string(),
        ..Default::default()
    };
    module.visit_with(&mut analyzer);

    // Recursively parse required modules
    let mut processed_files = std::collections::HashSet::new();
    processed_files.insert(plugin_path.to_string());

    parse_required_modules(&mut analyzer, &mut processed_files);

    if json_output {
        // JSON output for testing
        println!("{{");
        println!("  \"visitorMethods\": [");
        for (i, method) in analyzer.visitor_methods.iter().enumerate() {
            println!("    {{");
            println!("      \"name\": \"{}\",", method.name);
            println!("      \"transformCount\": {}", method.transforms.len());
            println!("    }}{}", if i < analyzer.visitor_methods.len() - 1 { "," } else { "" });
        }
        println!("  ]");
        println!("}}");
    } else {
        // Human-readable output
        println!("Found {} visitor methods\n", analyzer.visitor_methods.len());

        // Show detailed analysis of each visitor method
        for method in &analyzer.visitor_methods {
            println!("=== Visitor: {} ===", method.name);
            println!("Transforms detected: {}", method.transforms.len());

            for (i, transform) in method.transforms.iter().enumerate() {
                println!("  {}. {:?}", i + 1, transform);
            }
            println!();
        }

        // Show helper functions
        if !analyzer.helper_functions.is_empty() {
            println!("Found {} helper functions\n", analyzer.helper_functions.len());

            for func in &analyzer.helper_functions {
                println!("=== Helper: {} ===", func.name);
                println!("Transforms detected: {}", func.transforms.len());

                for (i, transform) in func.transforms.iter().enumerate() {
                    println!("  {}. {:?}", i + 1, transform);
                }
                println!();
            }
        }

        // Generate complete SWC plugin crate
        if !analyzer.visitor_methods.is_empty() || !analyzer.helper_functions.is_empty() {
            println!("\n=== Generating SWC Plugin Crate ===\n");

            let output_dir = "generated-plugin";
            generate_swc_plugin_crate(output_dir, &analyzer.visitor_methods, &analyzer.helper_functions);

            println!("✓ Generated plugin at: {}/", output_dir);
            println!("✓ To use: cd {} && cargo build", output_dir);
        }
    }
}

/// Convert JS module path to Rust module name
/// e.g., "example/babel-plugin-minimact/src/generators/csharpFile.cjs" -> "csharp_file"
fn get_module_name(source_path: &str) -> String {
    let path = Path::new(source_path);
    let file_stem = path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    // Convert camelCase to snake_case
    to_snake_case(file_stem)
}

/// Generate a module file with helper functions
fn generate_module_file(helpers: &[&HelperFunction]) -> String {
    let mut code = String::new();

    code.push_str("// Auto-generated module\n\n");

    for helper in helpers {
        // Only include code generation functions (filter out AST helpers)
        let is_codegen = helper.name.starts_with("generate") ||
                        helper.name.contains("CSharp") ||
                        helper.name.contains("csharp");

        if is_codegen {
            code.push_str(&generate_helper_function(helper));
            code.push_str("\n");
        }
    }

    code
}

/// Generate mod.rs that exports all modules
fn generate_mod_rs(module_names: &[String]) -> String {
    let mut code = String::new();

    code.push_str("// Auto-generated module declarations\n\n");

    for name in module_names {
        code.push_str(&format!("pub mod {};\n", name));
    }

    code
}

fn generate_swc_plugin_crate(output_dir: &str, visitor_methods: &[VisitorMethod], helper_functions: &[HelperFunction]) {
    use std::path::Path;
    use std::collections::HashMap;

    // Create directory structure
    let dir = Path::new(output_dir);
    let src_dir = dir.join("src");
    fs::create_dir_all(&src_dir).expect("Failed to create directories");

    // Group helper functions by source module
    let mut modules: HashMap<String, Vec<&HelperFunction>> = HashMap::new();
    for helper in helper_functions {
        let module_name = get_module_name(&helper.source_module);
        modules.entry(module_name).or_insert_with(Vec::new).push(helper);
    }

    // Generate Cargo.toml
    let cargo_toml = generate_cargo_toml();
    fs::write(dir.join("Cargo.toml"), cargo_toml).expect("Failed to write Cargo.toml");

    // Create generators directory
    let generators_dir = src_dir.join("generators");
    fs::create_dir_all(&generators_dir).expect("Failed to create generators directory");

    // Generate module files
    let mut module_names = Vec::new();
    for (module_name, helpers) in &modules {
        let module_rs = generate_module_file(helpers);
        let file_path = generators_dir.join(format!("{}.rs", module_name));
        fs::write(&file_path, module_rs).expect("Failed to write module file");
        module_names.push(module_name.clone());
        println!("  - {}/src/generators/{}.rs ({} functions)", output_dir, module_name, helpers.len());
    }

    // Generate generators/mod.rs
    let mod_rs = generate_mod_rs(&module_names);
    fs::write(generators_dir.join("mod.rs"), mod_rs).expect("Failed to write mod.rs");

    // Generate lib.rs (with visitor only, helpers are in modules)
    let lib_rs = generate_lib_rs(visitor_methods);
    fs::write(src_dir.join("lib.rs"), lib_rs).expect("Failed to write lib.rs");

    // Generate main.rs (test runner)
    let main_rs = generate_main_rs();
    fs::write(src_dir.join("main.rs"), main_rs).expect("Failed to write main.rs");

    println!("Generated files:");
    println!("  - {}/Cargo.toml", output_dir);
    println!("  - {}/src/lib.rs", output_dir);
    println!("  - {}/src/generators/mod.rs", output_dir);
    println!("  - {}/src/main.rs", output_dir);
}

fn generate_cargo_toml() -> String {
    r#"[package]
name = "swc-generated-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[[bin]]
name = "extract-components"
path = "src/main.rs"

[dependencies]
swc_common = "17"
swc_ecma_ast = "18"
swc_ecma_parser = "27"
swc_ecma_visit = "18"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
"#.to_string()
}

fn generate_lib_rs(visitor_methods: &[VisitorMethod]) -> String {
    let mut code = String::new();

    // Header with imports and data structures
    code.push_str(r#"use swc_ecma_ast::*;
use swc_ecma_visit::{VisitMut, VisitMutWith};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

pub mod generators;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub name: String,
    pub hooks: Vec<Hook>,
    #[serde(rename = "jsxElements")]
    pub jsx_elements: Vec<JsxElementData>,
    pub props: Vec<PropData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hook {
    #[serde(rename = "type")]
    pub hook_type: String,
    #[serde(rename = "stateName")]
    pub state_name: String,
    #[serde(rename = "setterName")]
    pub setter_name: Option<String>,
    #[serde(rename = "initialValue")]
    pub initial_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsxElementData {
    #[serde(rename = "type")]
    pub element_type: String,
    pub attributes: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropData {
    pub name: String,
    #[serde(rename = "type")]
    pub prop_type: String,
}

pub struct ComponentExtractor {
    pub components: Vec<Component>,
    current_component: Option<Component>,
}

impl ComponentExtractor {
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
            current_component: None,
        }
    }

    fn start_component(&mut self, name: String) {
        self.current_component = Some(Component {
            name,
            hooks: Vec::new(),
            jsx_elements: Vec::new(),
            props: Vec::new(),
        });
    }

    fn finish_component(&mut self) {
        if let Some(comp) = self.current_component.take() {
            self.components.push(comp);
        }
    }

    fn add_hook(&mut self, hook: Hook) {
        if let Some(ref mut comp) = self.current_component {
            comp.hooks.push(hook);
        }
    }

    fn add_jsx_element(&mut self, element: JsxElementData) {
        if let Some(ref mut comp) = self.current_component {
            comp.jsx_elements.push(element);
        }
    }
}

"#);

    // Generate visitor implementation
    code.push_str("impl VisitMut for ComponentExtractor {\n");

    // Add module visitor to ensure traversal
    code.push_str(r#"    fn visit_mut_module(&mut self, module: &mut Module) {
        eprintln!("Visiting module with {} items", module.body.len());
        module.visit_mut_children_with(self);
    }

    fn visit_mut_module_decl(&mut self, decl: &mut ModuleDecl) {
        eprintln!("Visiting module decl: {:?}", decl);
        decl.visit_mut_children_with(self);
    }

    fn visit_mut_export_default_decl(&mut self, export: &mut ExportDefaultDecl) {
        eprintln!("Found export default");
        export.visit_mut_children_with(self);
    }

"#);

    // Always include the essential visitor methods
    code.push_str(generate_fn_decl_visitor().as_str());
    code.push_str(r#"    fn visit_mut_fn_expr(&mut self, node: &mut FnExpr) {
        if let Some(ident) = &node.ident {
            let name = ident.sym.to_string();
            if name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                eprintln!("Found component function expression: {}", name);
                self.start_component(name);
                node.visit_mut_children_with(self);
                self.finish_component();
                return;
            }
        }
        node.visit_mut_children_with(self);
    }

"#);
    code.push_str(generate_jsx_element_visitor().as_str());
    code.push_str(generate_var_declarator_visitor().as_str());

    code.push_str("}\n");

    code
}

/// Generate a Rust helper function from detected transforms
fn generate_helper_function(helper: &HelperFunction) -> String {
    let mut code = String::new();
    let func_name = to_snake_case(&helper.name);

    // Detect if this function builds a string (has ArrayPush transforms)
    let has_string_building = helper.transforms.iter().any(|t| {
        matches!(t, Transform::ArrayPush { .. } | Transform::ArrayPushSpread { .. } | Transform::ArrayJoin { .. })
    });

    // Generate Rust parameters from JavaScript parameters
    let rust_params = if helper.params.is_empty() {
        String::new()
    } else {
        helper.params.iter()
            .map(|p| {
                // Convert parameter name to snake_case and infer type
                let param_name = to_snake_case(p);
                // Simple type inference based on parameter name
                let param_type = if param_name.contains("component") && !param_name.ends_with("s") {
                    "&Component"
                } else if param_name.ends_with("s") || param_name == "components" {
                    "&[Component]"
                } else {
                    "&str"
                };
                format!("{}: {}", param_name, param_type)
            })
            .collect::<Vec<_>>()
            .join(", ")
    };

    // Smart return type inference
    let return_type = if helper.name.starts_with("infer") {
        "&'static str"  // inferCSharpType returns literals
    } else if helper.name.starts_with("convert") || has_string_building {
        "String"  // convertToCSharp returns params, string builders return String
    } else {
        "&'static str"
    };

    // Generate function signature (public for C# generation functions)
    code.push_str(&format!("pub fn {}({}) -> {} {{\n", func_name, rust_params, return_type));

    if has_string_building {
        code.push_str("    let mut code = String::new();\n\n");
    }

    // Generate body from transforms - skip the initial array declaration if it's for string building
    for transform in &helper.transforms {
        // Skip `const lines = []` declarations - we use `code` instead
        if let Transform::VariableDeclaration { name, value, .. } = transform {
            if (name == "lines" || name == "code") && value == "value" {
                continue; // Skip this, we already initialized `code`
            }
        }

        // Skip return statements for string builder patterns (handled by implicit return at end)
        if let Transform::ReturnStmt { value } = transform {
            if value.contains(".join(") || value == "lines" || value == "code" {
                continue; // Skip - implicit `code` return will be added at the end
            }
        }

        code.push_str(&generate_transform_code_smart(transform, 1, return_type));
    }

    // Only add implicit return for string-building functions
    if has_string_building {
        code.push_str("\n    code\n");
    }
    code.push_str("}\n");

    code
}

#[derive(Debug, Clone)]
pub enum Transform {
    PropertyAccess { js_path: String, rust_equiv: String },
    MetadataAssignment { key: String, value_expr: String },
    FunctionCall { name: String, args: Vec<String> },
    Conditional { condition: String, then_transforms: Vec<Transform>, else_transforms: Option<Vec<Transform>> },
    VariableDeclaration { name: String, value: String, is_destructured: bool },
    ArrayAccess { object: String, index: String },
    JSXExtract { tag_name: String, property_path: String },
    HookCapture { hook_type: String, state_name: String, setter_name: Option<String>, args: Vec<String> },
    LogicalChain { operator: String, left: Box<Transform>, right: Box<Transform> },
    TernaryExpr { condition: String, then_expr: String, else_expr: String },
    TemplateStore { id: String, template_expr: String },
    TemplateLiteral { parts: Vec<String>, exprs: Vec<String> },
    ReturnStmt { value: String },
    TypeCheck { check_type: String, target: String, rust_pattern: String },
    TraverseCall { visitors: Vec<String> },
    // String building patterns for C# generation
    ArrayPush { array_name: String, value: String },
    ArrayPushSpread { array_name: String, source_array: String },
    ArrayJoin { array_name: String, separator: String },
    // Control flow
    ForOfLoop { iterator: String, iterable: String, body_transforms: Vec<Transform> },
    RawJs { code: String, reason: String },
}

#[derive(Debug)]
pub struct VisitorMethod {
    pub name: String,
    pub transforms: Vec<Transform>,
}

#[derive(Debug)]
pub struct HelperFunction {
    pub name: String,
    pub params: Vec<String>,  // Parameter names from function signature
    pub transforms: Vec<Transform>,
    pub source_module: String,  // Path to the source JS file
}

#[derive(Debug)]
struct RequiredModule {
    module_path: String,        // e.g., "../babel-plugin-minimact/src/generators/csharpFile.cjs"
    imported_names: Vec<String>, // e.g., ["generateCSharpFile"]
    resolved_path: String,       // Absolute file path
}

#[derive(Debug, Default)]
struct BabelVisitorDetector {
    visitor_methods: Vec<VisitorMethod>,
    helper_functions: Vec<HelperFunction>,
    required_modules: Vec<RequiredModule>,
    current_file_path: String,
    current_method: Option<String>,
}

#[derive(Debug, Default)]
struct VisitorBodyAnalyzer {
    transforms: Vec<Transform>,
}

impl VisitorBodyAnalyzer {
    fn extract_init_value(&mut self, init: &Option<Box<Expr>>) -> String {
        if let Some(init) = init {
            match &**init {
                Expr::Member(member) => self.extract_member_path(member),
                Expr::Call(call) => {
                    // Extract full function call with arguments
                    if let Some(Transform::FunctionCall { name, args }) = self.detect_function_call(call) {
                        if args.is_empty() {
                            format!("{}()", name)
                        } else {
                            format!("{}({})", name, args.join(", "))
                        }
                    } else {
                        "call()".to_string()
                    }
                }
                Expr::Lit(Lit::Str(s)) => format!("{:?}", s.value),
                Expr::Lit(Lit::Num(n)) => n.value.to_string(),
                Expr::Ident(ident) => ident.sym.to_string(),
                _ => "value".to_string(),
            }
        } else {
            "undefined".to_string()
        }
    }

    fn extract_hook_name(&self, call: &CallExpr) -> Option<String> {
        if let Callee::Expr(expr) = &call.callee {
            if let Expr::Ident(ident) = &**expr {
                let name = ident.sym.to_string();
                if name.starts_with("use") {
                    return Some(name);
                }
            }
        }
        None
    }

    fn extract_condition_expr(&self, expr: &Expr) -> String {
        match expr {
            Expr::Call(call) => {
                // Try to extract the full call expression
                self.extract_expr_string(expr)
            }
            Expr::Bin(bin) => {
                let left = self.extract_expr_string(&bin.left);
                let right = self.extract_expr_string(&bin.right);
                let op = match bin.op {
                    BinaryOp::EqEq | BinaryOp::EqEqEq => "==",
                    BinaryOp::NotEq | BinaryOp::NotEqEq => "!=",
                    BinaryOp::Lt => "<",
                    BinaryOp::LtEq => "<=",
                    BinaryOp::Gt => ">",
                    BinaryOp::GtEq => ">=",
                    BinaryOp::LogicalAnd => "&&",
                    BinaryOp::LogicalOr => "||",
                    _ => "?",
                };
                format!("{} {} {}", left, op, right)
            }
            Expr::Member(member) => self.extract_member_path(member),
            Expr::Ident(ident) => ident.sym.to_string(),
            _ => "condition".to_string(),
        }
    }

    fn extract_expr_string(&self, expr: &Expr) -> String {
        match expr {
            Expr::Ident(ident) => ident.sym.to_string(),
            Expr::Member(member) => self.extract_member_path(member),
            Expr::Lit(Lit::Str(s)) => format!("{:?}", s.value),
            Expr::Lit(Lit::Num(n)) => n.value.to_string(),
            Expr::Lit(Lit::Bool(b)) => b.value.to_string(),
            Expr::Lit(Lit::Regex(r)) => {
                // Capture regex pattern for later translation
                format!("REGEX({})", r.exp)
            }
            Expr::Bin(bin) => {
                // Handle binary expressions recursively
                let left = self.extract_expr_string(&bin.left);
                let right = self.extract_expr_string(&bin.right);
                let op = match bin.op {
                    BinaryOp::EqEq | BinaryOp::EqEqEq => "==",
                    BinaryOp::NotEq | BinaryOp::NotEqEq => "!=",
                    BinaryOp::Lt => "<",
                    BinaryOp::LtEq => "<=",
                    BinaryOp::Gt => ">",
                    BinaryOp::GtEq => ">=",
                    BinaryOp::LogicalAnd => "&&",
                    BinaryOp::LogicalOr => "||",
                    BinaryOp::Add => "+",
                    BinaryOp::Sub => "-",
                    BinaryOp::Mul => "*",
                    BinaryOp::Div => "/",
                    _ => "?",
                };
                format!("{} {} {}", left, op, right)
            }
            Expr::Call(call) => {
                // Extract arguments
                let args: Vec<String> = call.args.iter()
                    .map(|arg| {
                        match &*arg.expr {
                            Expr::Lit(Lit::Str(s)) => {
                                let val = String::from_utf8_lossy(s.value.as_bytes()).to_string();
                                // Properly escape the string
                                format!("\"{}\"", val.replace("\\", "\\\\").replace("\"", "\\\""))
                            }
                            Expr::Lit(Lit::Num(n)) => n.value.to_string(),
                            Expr::Ident(id) => id.sym.to_string(),
                            _ => self.extract_expr_string(&arg.expr),
                        }
                    })
                    .collect();
                let args_str = args.join(", ");

                if let Callee::Expr(e) = &call.callee {
                    match &**e {
                        Expr::Ident(ident) => {
                            return format!("{}({})", ident.sym, args_str);
                        }
                        Expr::Member(member) => {
                            // Handle method calls like lines.join('\n')
                            let obj = self.extract_expr_string(&member.obj);
                            if let MemberProp::Ident(method) = &member.prop {
                                return format!("{}.{}({})", obj, method.sym, args_str);
                            }
                        }
                        _ => {}
                    }
                }
                "call()".to_string()
            }
            Expr::Unary(unary) => {
                // Handle unary expressions like !value, -number, etc.
                let arg = self.extract_expr_string(&unary.arg);
                let op = match unary.op {
                    UnaryOp::Bang => "!",
                    UnaryOp::Minus => "-",
                    UnaryOp::Plus => "+",
                    _ => "?",
                };
                format!("{}{}", op, arg)
            }
            _ => "expr".to_string(),
        }
    }

    fn expr_to_transform(&mut self, expr: &Expr) -> Transform {
        match expr {
            Expr::Call(call) => {
                if let Some(t) = self.detect_function_call(call) {
                    t
                } else {
                    Transform::RawJs {
                        code: "call()".to_string(),
                        reason: "Unknown call expression".to_string(),
                    }
                }
            }
            _ => Transform::RawJs {
                code: self.extract_expr_string(expr),
                reason: "Unsupported expression in logical chain".to_string(),
            },
        }
    }

    fn detect_assignment(&mut self, assign: &AssignExpr) -> Option<Transform> {
        // Detect: path.metadata.foo = bar
        if let AssignTarget::Simple(SimpleAssignTarget::Member(member)) = &assign.left {
            let path = self.extract_member_path(member);
            if path.contains("metadata") {
                let key = path.split('.').last().unwrap_or("unknown");
                return Some(Transform::MetadataAssignment {
                    key: key.to_string(),
                    value_expr: "value".to_string(), // TODO: extract from assign.right
                });
            }
        }
        None
    }

    fn detect_function_call(&mut self, call: &CallExpr) -> Option<Transform> {
        // Detect array.push() patterns: lines.push('string') or lines.push(...other)
        if let Callee::Expr(expr) = &call.callee {
            if let Expr::Member(member) = &**expr {
                if let MemberProp::Ident(method) = &member.prop {
                    if &*method.sym == "push" {
                        // Extract array name
                        let array_name = if let Expr::Ident(arr_ident) = &*member.obj {
                            arr_ident.sym.to_string()
                        } else {
                            "array".to_string()
                        };

                        // Check if it's push(...spread)
                        if let Some(arg) = call.args.first() {
                            if arg.spread.is_some() {
                                // lines.push(...generateComponent(c))
                                let source = self.extract_arg_value(&arg.expr);
                                return Some(Transform::ArrayPushSpread {
                                    array_name,
                                    source_array: source,
                                });
                            } else {
                                // lines.push('string') or lines.push(variable)
                                let value = self.extract_arg_value(&arg.expr);
                                return Some(Transform::ArrayPush {
                                    array_name,
                                    value,
                                });
                            }
                        }
                    } else if &*method.sym == "join" {
                        // Detect array.join() patterns: lines.join('\n')
                        let array_name = if let Expr::Ident(arr_ident) = &*member.obj {
                            arr_ident.sym.to_string()
                        } else {
                            "array".to_string()
                        };

                        let separator = if let Some(arg) = call.args.first() {
                            self.extract_string_literal(&arg.expr)
                        } else {
                            ",".to_string()
                        };

                        return Some(Transform::ArrayJoin {
                            array_name,
                            separator,
                        });
                    }
                }
            } else if let Expr::Ident(ident) = &**expr {
                // Regular function calls like processComponent(path, state, name)
                let func_name = ident.sym.to_string();
                let args: Vec<String> = call.args.iter()
                    .map(|arg| self.extract_arg_name(&arg.expr))
                    .collect();

                return Some(Transform::FunctionCall {
                    name: func_name,
                    args,
                });
            }
        }
        None
    }

    fn extract_member_path(&self, member: &MemberExpr) -> String {
        // Build a path like "path.node.id.name" or detect array access like "body.body[0]"
        let mut parts = vec![];

        match &member.prop {
            MemberProp::Ident(ident) => {
                parts.push(ident.sym.to_string());
            }
            MemberProp::Computed(computed) => {
                // Handle array access: obj[index]
                let index = match &*computed.expr {
                    Expr::Lit(Lit::Num(n)) => n.value.to_string(),
                    Expr::Ident(ident) => ident.sym.to_string(),
                    _ => "idx".to_string(),
                };
                parts.push(format!("[{}]", index));
            }
            _ => {}
        }

        let mut current = &member.obj;
        loop {
            match &**current {
                Expr::Ident(ident) => {
                    parts.insert(0, ident.sym.to_string());
                    break;
                }
                Expr::Member(mem) => {
                    match &mem.prop {
                        MemberProp::Ident(ident) => {
                            parts.insert(0, ident.sym.to_string());
                        }
                        MemberProp::Computed(computed) => {
                            let index = match &*computed.expr {
                                Expr::Lit(Lit::Num(n)) => n.value.to_string(),
                                Expr::Ident(ident) => ident.sym.to_string(),
                                _ => "idx".to_string(),
                            };
                            parts.insert(0, format!("[{}]", index));
                        }
                        _ => {}
                    }
                    current = &mem.obj;
                }
                _ => break,
            }
        }

        parts.join(".")
    }

    fn extract_arg_name(&self, expr: &Expr) -> String {
        match expr {
            Expr::Ident(ident) => ident.sym.to_string(),
            Expr::Member(member) => self.extract_member_path(member),
            _ => "expr".to_string(),
        }
    }

    fn extract_arg_value(&self, expr: &Expr) -> String {
        match expr {
            Expr::Lit(Lit::Str(s)) => {
                // Return the string with quotes - use String::from to convert Wtf8
                let content = String::from_utf8_lossy(s.value.as_bytes()).to_string();
                format!("\"{}\"", content)
            }
            Expr::Tpl(tpl) => {
                // Template literal - collect parts and expressions with variable names
                let mut result = String::new();
                result.push('`');
                for (i, quasi) in tpl.quasis.iter().enumerate() {
                    result.push_str(&quasi.raw);
                    if i < tpl.exprs.len() {
                        // Extract the expression variable name
                        let expr_name = self.extract_arg_name(&tpl.exprs[i]);
                        result.push_str(&format!("${{{}}}", expr_name));
                    }
                }
                result.push('`');
                result
            }
            Expr::Ident(ident) => ident.sym.to_string(),
            Expr::Call(call) => {
                // Function call like generateComponent(c)
                if let Callee::Expr(callee_expr) = &call.callee {
                    if let Expr::Ident(func_ident) = &**callee_expr {
                        let func_name = func_ident.sym.to_string();
                        let args: Vec<String> = call.args.iter()
                            .map(|arg| self.extract_arg_name(&arg.expr))
                            .collect();
                        return format!("{}({})", func_name, args.join(", "));
                    }
                }
                "call()".to_string()
            }
            _ => "value".to_string(),
        }
    }

    fn extract_string_literal(&self, expr: &Expr) -> String {
        match expr {
            Expr::Lit(Lit::Str(s)) => String::from_utf8_lossy(s.value.as_bytes()).to_string(),
            _ => ",".to_string(),
        }
    }

    fn translate_path_to_rust(&self, js_path: &str) -> String {
        // Translate common Babel patterns to Rust
        match js_path {
            p if p.contains("path.node.id.name") => "node.ident.sym".to_string(),
            p if p.contains("path.node.params") => "node.params".to_string(),
            p if p.contains("path.parent.type") => "parent_type".to_string(),
            _ => js_path.replace("path.node", "node"),
        }
    }
}

impl Visit for VisitorBodyAnalyzer {
    fn visit_expr_stmt(&mut self, stmt: &ExprStmt) {
        match &*stmt.expr {
            Expr::Assign(assign) => {
                if let Some(transform) = self.detect_assignment(assign) {
                    self.transforms.push(transform);
                }
            }
            Expr::Call(call) => {
                if let Some(transform) = self.detect_function_call(call) {
                    self.transforms.push(transform);
                }
            }
            _ => {}
        }
        stmt.visit_children_with(self);
    }

    fn visit_var_decl(&mut self, var_decl: &VarDecl) {
        for decl in &var_decl.decls {
            match &decl.name {
                // Simple identifier: const name = value
                Pat::Ident(ident) => {
                    let var_name = ident.id.sym.to_string();
                    let value = self.extract_init_value(&decl.init);

                    self.transforms.push(Transform::VariableDeclaration {
                        name: var_name,
                        value,
                        is_destructured: false,
                    });
                }
                // Object destructuring: const { id, name } = node
                Pat::Object(obj_pat) => {
                    let source = self.extract_init_value(&decl.init);

                    for prop in &obj_pat.props {
                        match prop {
                            ObjectPatProp::KeyValue(kv) => {
                                if let PropName::Ident(key_ident) = &kv.key {
                                    if let Pat::Ident(val_ident) = &*kv.value {
                                        let var_name = val_ident.id.sym.to_string();
                                        let value = format!("{}.{}", source, key_ident.sym);

                                        self.transforms.push(Transform::VariableDeclaration {
                                            name: var_name,
                                            value,
                                            is_destructured: true,
                                        });
                                    }
                                }
                            }
                            ObjectPatProp::Assign(assign) => {
                                let var_name = assign.key.sym.to_string();
                                let value = format!("{}.{}", source, assign.key.sym);

                                self.transforms.push(Transform::VariableDeclaration {
                                    name: var_name,
                                    value,
                                    is_destructured: true,
                                });
                            }
                            _ => {}
                        }
                    }
                }
                // Array destructuring: const [state, setState] = useState(0)
                Pat::Array(array_pat) => {
                    if let Some(init) = &decl.init {
                        if let Expr::Call(call) = &**init {
                            // Check if it's a hook call
                            if let Some(hook_name) = self.extract_hook_name(call) {
                                let mut state_name = None;
                                let mut setter_name = None;

                                if array_pat.elems.len() >= 1 {
                                    if let Some(Pat::Ident(id)) = &array_pat.elems[0] {
                                        state_name = Some(id.id.sym.to_string());
                                    }
                                }
                                if array_pat.elems.len() >= 2 {
                                    if let Some(Pat::Ident(id)) = &array_pat.elems[1] {
                                        setter_name = Some(id.id.sym.to_string());
                                    }
                                }

                                let args: Vec<String> = call.args.iter()
                                    .map(|arg| self.extract_arg_name(&arg.expr))
                                    .collect();

                                self.transforms.push(Transform::HookCapture {
                                    hook_type: hook_name,
                                    state_name: state_name.unwrap_or_else(|| "state".to_string()),
                                    setter_name,
                                    args,
                                });
                            }
                        }
                    }
                }
                _ => {
                    // Fallback for unknown patterns
                    self.transforms.push(Transform::RawJs {
                        code: "let unknown = value;".to_string(),
                        reason: "Unknown destructuring pattern".to_string(),
                    });
                }
            }
        }
        var_decl.visit_children_with(self);
    }

    fn visit_for_of_stmt(&mut self, for_of: &ForOfStmt) {
        // Extract iterator variable name: for (const component of components)
        let iterator = match &for_of.left {
            ForHead::VarDecl(var_decl) => {
                if let Some(decl) = var_decl.decls.first() {
                    if let Pat::Ident(ident) = &decl.name {
                        ident.id.sym.to_string()
                    } else {
                        "item".to_string()
                    }
                } else {
                    "item".to_string()
                }
            }
            ForHead::Pat(pat) => {
                if let Pat::Ident(ident) = &**pat {
                    ident.id.sym.to_string()
                } else {
                    "item".to_string()
                }
            }
            _ => "item".to_string(),
        };

        // Extract iterable: components
        let iterable = match &*for_of.right {
            Expr::Ident(ident) => ident.sym.to_string(),
            Expr::Member(member) => self.extract_member_path(member),
            _ => "iterable".to_string(),
        };

        // Analyze loop body
        let mut body_analyzer = VisitorBodyAnalyzer::default();
        for_of.body.visit_with(&mut body_analyzer);

        self.transforms.push(Transform::ForOfLoop {
            iterator,
            iterable,
            body_transforms: body_analyzer.transforms,
        });

        // Don't visit children - we already analyzed the body above
        // for_of.visit_children_with(self);
    }

    fn visit_if_stmt(&mut self, if_stmt: &IfStmt) {
        let condition = self.extract_condition_expr(&if_stmt.test);

        let mut then_analyzer = VisitorBodyAnalyzer::default();
        if_stmt.cons.visit_with(&mut then_analyzer);

        let else_transforms = if let Some(alt) = &if_stmt.alt {
            let mut else_analyzer = VisitorBodyAnalyzer::default();
            alt.visit_with(&mut else_analyzer);
            Some(else_analyzer.transforms)
        } else {
            None
        };

        self.transforms.push(Transform::Conditional {
            condition,
            then_transforms: then_analyzer.transforms,
            else_transforms,
        });

        // Don't visit children - we already analyzed then/else bodies above
        // if_stmt.visit_children_with(self);
    }

    fn visit_cond_expr(&mut self, cond: &CondExpr) {
        // Ternary: condition ? then_expr : else_expr
        let condition = self.extract_condition_expr(&cond.test);
        let then_expr = self.extract_expr_string(&cond.cons);
        let else_expr = self.extract_expr_string(&cond.alt);

        self.transforms.push(Transform::TernaryExpr {
            condition,
            then_expr,
            else_expr,
        });

        cond.visit_children_with(self);
    }

    fn visit_bin_expr(&mut self, bin: &BinExpr) {
        // Note: Binary expressions (including && and ||) are already handled
        // in extract_condition_expr and extract_expr_string recursively.
        // We don't need to create separate LogicalChain transforms here
        // as they would be duplicates of what's already in the condition string.

        // Skip visiting children to avoid duplicates
        // bin.visit_children_with(self);
    }

    fn visit_call_expr(&mut self, call: &CallExpr) {
        // Detect type checking: t.isFunctionDeclaration(node)
        if let Callee::Expr(expr) = &call.callee {
            if let Expr::Member(member) = &**expr {
                if let (Expr::Ident(obj), MemberProp::Ident(method)) = (&*member.obj, &member.prop) {
                    if &*obj.sym == "t" && method.sym.starts_with("is") {
                        let check_type = method.sym.to_string();
                        let target = if let Some(arg) = call.args.first() {
                            self.extract_arg_name(&arg.expr)
                        } else {
                            "node".to_string()
                        };
                        let rust_pattern = translate_type_check(&check_type, &target);

                        self.transforms.push(Transform::TypeCheck {
                            check_type,
                            target,
                            rust_pattern,
                        });
                    }
                }
            }
        }

        call.visit_children_with(self);
    }

    fn visit_return_stmt(&mut self, ret: &ReturnStmt) {
        if let Some(arg) = &ret.arg {
            let value = self.extract_expr_string(arg);
            self.transforms.push(Transform::ReturnStmt { value });
        }
        ret.visit_children_with(self);
    }

    fn visit_tpl(&mut self, tpl: &Tpl) {
        // Template literal: `Hello ${name}!`
        let mut parts = Vec::new();
        let mut exprs = Vec::new();

        for quasi in &tpl.quasis {
            parts.push(quasi.raw.to_string());
        }

        for expr in &tpl.exprs {
            exprs.push(self.extract_expr_string(expr));
        }

        self.transforms.push(Transform::TemplateLiteral { parts, exprs });

        tpl.visit_children_with(self);
    }
}

fn translate_type_check(check_type: &str, target: &str) -> String {
    match check_type {
        "isFunctionDeclaration" => format!("if let Decl::Fn(fn_decl) = {}", target),
        "isIdentifier" => format!("if let Expr::Ident(ident) = {}", target),
        "isCallExpression" => format!("if let Expr::Call(call) = {}", target),
        "isArrowFunctionExpression" => format!("if let Expr::Arrow(arrow) = {}", target),
        "isMemberExpression" => format!("if let Expr::Member(member) = {}", target),
        "isObjectPattern" => format!("if let Pat::Object(obj_pat) = {}", target),
        "isArrayPattern" => format!("if let Pat::Array(array_pat) = {}", target),
        "isObjectProperty" => format!("if let PropOrSpread::Prop(prop) = {}", target),
        "isJSXElement" => format!("if let Expr::JSXElement(jsx) = {}", target),
        "isVariableDeclarator" => format!("if let VarDeclarator {{ .. }} = {}", target),
        "isBlockStatement" => format!("if let Stmt::Block(block) = {}", target),
        "isReturnStatement" => format!("if let Stmt::Return(ret) = {}", target),
        "isStringLiteral" => format!("if let Lit::Str(s) = {}", target),
        "isNumericLiteral" => format!("if let Lit::Num(n) = {}", target),
        "isBooleanLiteral" => format!("if let Lit::Bool(b) = {}", target),
        _ => format!("// TODO: Translate {}", check_type),
    }
}

impl Visit for BabelVisitorDetector {
    fn visit_var_decl(&mut self, var_decl: &VarDecl) {
        // Detect require() calls: const { generateCSharpFile } = require('../path/to/module.cjs')
        for decl in &var_decl.decls {
            if let Some(init) = &decl.init {
                if let Expr::Call(call) = &**init {
                    if let Callee::Expr(callee_expr) = &call.callee {
                        if let Expr::Ident(ident) = &**callee_expr {
                            if &*ident.sym == "require" {
                                // Extract module path from require('path')
                                if let Some(arg) = call.args.first() {
                                    if let Expr::Lit(Lit::Str(s)) = &*arg.expr {
                                        let module_path = String::from_utf8_lossy(s.value.as_bytes()).to_string();

                                        // Extract imported names from destructuring
                                        let mut imported_names = Vec::new();
                                        if let Pat::Object(obj_pat) = &decl.name {
                                            for prop in &obj_pat.props {
                                                match prop {
                                                    ObjectPatProp::KeyValue(kv) => {
                                                        if let PropName::Ident(key_ident) = &kv.key {
                                                            imported_names.push(key_ident.sym.to_string());
                                                        }
                                                    }
                                                    ObjectPatProp::Assign(assign) => {
                                                        imported_names.push(assign.key.sym.to_string());
                                                    }
                                                    _ => {}
                                                }
                                            }
                                        }

                                        // Resolve relative path
                                        let resolved_path = resolve_module_path(&self.current_file_path, &module_path);

                                        eprintln!("Detected require: {} -> {}", module_path, resolved_path);
                                        eprintln!("  Imported: {:?}", imported_names);

                                        self.required_modules.push(RequiredModule {
                                            module_path: module_path.clone(),
                                            imported_names,
                                            resolved_path,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        var_decl.visit_children_with(self);
    }

    fn visit_fn_decl(&mut self, func: &FnDecl) {
        // Detect top-level helper functions
        let func_name = func.ident.sym.to_string();

        // Skip the main module.exports function
        if func_name != "exports" {
            // Extract parameter names
            let params: Vec<String> = func.function.params.iter()
                .filter_map(|param| {
                    if let Pat::Ident(ident) = &param.pat {
                        Some(ident.id.sym.to_string())
                    } else {
                        None
                    }
                })
                .collect();

            let mut body_analyzer = VisitorBodyAnalyzer::default();

            if let Some(body) = &func.function.body {
                body.visit_with(&mut body_analyzer);
            }

            self.helper_functions.push(HelperFunction {
                name: func_name,
                params,
                transforms: body_analyzer.transforms,
                source_module: self.current_file_path.clone(),
            });
        }

        func.visit_children_with(self);
    }

    fn visit_object_lit(&mut self, obj: &ObjectLit) {
        for prop in &obj.props {
            if let PropOrSpread::Prop(prop_box) = prop {
                if let Prop::KeyValue(kv) = &**prop_box {
                    if let PropName::Ident(key) = &kv.key {
                        if &*key.sym == "visitor" {
                            if let Expr::Object(inner_obj) = &*kv.value {
                                for method in &inner_obj.props {
                                    if let PropOrSpread::Prop(method_box) = method {
                                        match &**method_box {
                                            Prop::KeyValue(method_kv) => {
                                                if let PropName::Ident(method_ident) = &method_kv.key {
                                                    let method_name = method_ident.sym.to_string();
                                                    let mut body_analyzer = VisitorBodyAnalyzer::default();

                                                    // Analyze the method body (function expression)
                                                    method_kv.value.visit_with(&mut body_analyzer);

                                                    self.visitor_methods.push(VisitorMethod {
                                                        name: method_name,
                                                        transforms: body_analyzer.transforms,
                                                    });
                                                }
                                            }
                                            Prop::Method(method_prop) => {
                                                if let PropName::Ident(method_ident) = &method_prop.key {
                                                    let method_name = method_ident.sym.to_string();
                                                    let mut body_analyzer = VisitorBodyAnalyzer::default();

                                                    // Analyze the method body
                                                    if let Some(body) = &method_prop.function.body {
                                                        body.visit_with(&mut body_analyzer);
                                                    }

                                                    self.visitor_methods.push(VisitorMethod {
                                                        name: method_name,
                                                        transforms: body_analyzer.transforms,
                                                    });
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn generate_rust_visitor_with_transforms(visitor_name: &str, transforms: &[Transform]) -> String {
    let rust_method = babel_to_swc_method(visitor_name);
    let node_param = get_node_param(visitor_name);

    let mut body_code = String::new();

    // Generate code for each transform
    for transform in transforms {
        body_code.push_str(&generate_transform_code(transform, 2));
    }

    // If no transforms, add a placeholder comment
    if body_code.is_empty() {
        body_code = "        // No transforms detected in Babel plugin body\n".to_string();
    }

    format!(
r#"impl Visit for MyTranspiler {{
    fn {}(&mut self, {}: &{}) {{
{}        {}.visit_children_with(self);
    }}
}}"#,
        rust_method,
        node_param.0,
        node_param.1,
        body_code,
        node_param.0
    )
}

fn generate_transform_code_smart(transform: &Transform, indent: usize, return_type: &str) -> String {
    let indent_str = " ".repeat(indent * 4);

    match transform {
        Transform::ReturnStmt { value } => {
            if value.contains(".join(") || value == "lines" || value == "code" {
                String::new()
            } else {
                let rust_value = translate_js_to_rust(value);

                let final_value = if return_type == "String" {
                    // For String return type, add .to_string() to literals
                    if rust_value.starts_with('"') && !rust_value.contains("format!") {
                        format!("{}.to_string()", rust_value)
                    } else if !rust_value.ends_with("()") {
                        // Parameters/variables also need .to_string()
                        format!("{}.to_string()", rust_value)
                    } else {
                        rust_value
                    }
                } else {
                    // For &'static str, return as-is
                    rust_value
                };

                format!("{}return {};\n", indent_str, final_value)
            }
        }

        Transform::Conditional { condition, then_transforms, else_transforms } => {
            let cond = translate_condition(condition);
            let mut result = format!("{}if {} {{\n", indent_str, cond);
            for t in then_transforms {
                result.push_str(&generate_transform_code_smart(t, indent + 1, return_type));
            }
            result.push_str(&format!("{}}}", indent_str));

            if let Some(else_block) = else_transforms {
                result.push_str(" else {\n");
                for t in else_block {
                    result.push_str(&generate_transform_code_smart(t, indent + 1, return_type));
                }
                result.push_str(&format!("{}}}", indent_str));
            }
            result.push('\n');
            result
        }

        Transform::ForOfLoop { iterator, iterable, body_transforms } => {
            let iterable_ref = if iterable.contains('.') {
                format!("&{}", iterable)
            } else {
                iterable.clone()
            };
            let mut result = format!("{}for {} in {} {{\n", indent_str, iterator, iterable_ref);
            for t in body_transforms {
                result.push_str(&generate_transform_code_smart(t, indent + 1, return_type));
            }
            result.push_str(&format!("{}}}\n", indent_str));
            result
        }

        // Fallback to legacy for other types
        _ => generate_transform_code(transform, indent)
    }
}

fn generate_transform_code(transform: &Transform, indent: usize) -> String {
    let indent_str = " ".repeat(indent * 4);

    match transform {
        Transform::VariableDeclaration { name, value, is_destructured } => {
            let rust_name = to_snake_case(name);
            let rust_value = translate_js_to_rust(value);
            if *is_destructured {
                format!("{}// Destructured: let {} = {};\n", indent_str, rust_name, rust_value)
            } else {
                format!("{}let {} = {};\n", indent_str, rust_name, rust_value)
            }
        }
        Transform::FunctionCall { name, args } => {
            let rust_name = to_snake_case(name);
            let rust_args = args.iter()
                .map(|a| translate_js_to_rust(a))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}{}({});\n", indent_str, rust_name, rust_args)
        }
        Transform::MetadataAssignment { key, value_expr } => {
            format!("{}self.metadata.insert(\"{}\", {});\n", indent_str, key, value_expr)
        }
        Transform::Conditional { condition, then_transforms, else_transforms } => {
            let rust_condition = translate_condition(condition);
            let mut result = format!("{}if {} {{\n", indent_str, rust_condition);
            for t in then_transforms {
                result.push_str(&generate_transform_code(t, indent + 1));
            }
            result.push_str(&format!("{}}}", indent_str));

            if let Some(else_t) = else_transforms {
                result.push_str(" else {\n");
                for t in else_t {
                    result.push_str(&generate_transform_code(t, indent + 1));
                }
                result.push_str(&format!("{}}}", indent_str));
            }
            result.push('\n');
            result
        }
        Transform::PropertyAccess { js_path, rust_equiv } => {
            format!("{}// Access: {} -> {}\n", indent_str, js_path, rust_equiv)
        }
        Transform::ArrayAccess { object, index } => {
            let rust_object = translate_js_to_rust(object);
            format!("{}// Array access: {}[{}]\n", indent_str, rust_object, index)
        }
        Transform::JSXExtract { tag_name, property_path } => {
            format!("{}// JSX extract: <{}> property: {}\n", indent_str, tag_name, property_path)
        }
        Transform::HookCapture { hook_type, state_name, setter_name, args } => {
            let rust_hook = to_snake_case(hook_type);
            let args_str = args.iter().map(|a| translate_js_to_rust(a)).collect::<Vec<_>>().join(", ");

            if let Some(setter) = setter_name {
                format!("{}let ({}, {}) = {}({});\n", indent_str, state_name, setter, rust_hook, args_str)
            } else {
                format!("{}let {} = {}({});\n", indent_str, state_name, rust_hook, args_str)
            }
        }
        Transform::LogicalChain { operator, left, right } => {
            let op = if operator.contains("And") { "&&" } else { "||" };
            format!("{}// Logical chain: {:?} {} {:?}\n", indent_str, left, op, right)
        }
        Transform::TernaryExpr { condition, then_expr, else_expr } => {
            let rust_condition = translate_condition(condition);
            let rust_then = translate_js_to_rust(then_expr);
            let rust_else = translate_js_to_rust(else_expr);
            format!("{}let result = if {} {{ {} }} else {{ {} }};\n",
                    indent_str, rust_condition, rust_then, rust_else)
        }
        Transform::TemplateStore { id, template_expr } => {
            format!("{}self.templates.insert(\"{}\", {});\n", indent_str, id, template_expr)
        }
        Transform::TemplateLiteral { parts, exprs } => {
            // Generate format!() call for template literals
            // `Hello ${name}!` becomes format!("Hello {}!", name)
            let format_str = parts.iter().enumerate()
                .map(|(i, part)| {
                    if i < exprs.len() {
                        format!("{}{{}}", part)
                    } else {
                        part.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join("");

            let rust_exprs = exprs.iter()
                .map(|e| translate_js_to_rust(e))
                .collect::<Vec<_>>()
                .join(", ");

            if exprs.is_empty() {
                format!("{}let result = \"{}\";\n", indent_str, format_str)
            } else {
                format!("{}let result = format!(\"{}\", {});\n", indent_str, format_str, rust_exprs)
            }
        }
        Transform::ReturnStmt { value } => {
            // Skip return statements for string builder patterns
            // Pattern: return lines.join('\n') -> implicit 'code' return
            // Pattern: return lines -> implicit 'code' return
            if value.contains(".join(") || value == "lines" || value == "code" {
                // Skip - the function will have an implicit return of 'code'
                String::new()
            } else {
                let mut rust_value = translate_js_to_rust(value);
                // Add .to_string() to all values except function calls
                // This works for both String and &'static str returns:
                // - For String return type: converts &str to String
                // - For &'static str return type: "literal".to_string() is valid but unnecessary
                //   (the user may need to manually fix this for optimal code)
                if !rust_value.ends_with("()") {
                    rust_value = format!("{}.to_string()", rust_value);
                }
                format!("{}return {};\n", indent_str, rust_value)
            }
        }
        Transform::TypeCheck { check_type, target, rust_pattern } => {
            format!("{}// Type check: {} -> {}\n", indent_str, check_type, rust_pattern)
        }
        Transform::TraverseCall { visitors } => {
            format!("{}// path.traverse() with {} visitors\n", indent_str, visitors.len())
        }
        Transform::ArrayPush { array_name: _, value } => {
            // Always use 'code' for the string builder, regardless of original name (lines/code/etc)
            let rust_value = if value.starts_with('"') || value.starts_with('\'') {
                // String literal: lines.push("Hello") -> code.push_str("Hello\n");
                let content = value.trim_matches(|c| c == '"' || c == '\'');
                format!("code.push_str(\"{}\\n\");\n", content.replace("\\", "\\\\").replace("\"", "\\\""))
            } else if value.starts_with('`') {
                // Template literal: lines.push(`namespace ${ns};`) -> code.push_str(&format!("namespace {}\n", ns));
                let template = value.trim_matches('`');
                if template.contains("${") {
                    // Extract variable names from ${var} patterns
                    let mut format_str = template.to_string();
                    let mut vars = Vec::new();

                    // Find all ${varName} patterns
                    while let Some(start) = format_str.find("${") {
                        if let Some(end) = format_str[start..].find('}') {
                            let var_name = &format_str[start+2..start+end];
                            vars.push(var_name.to_string());
                            // Replace ${var} with {}
                            format_str.replace_range(start..start+end+1, "{}");
                        } else {
                            break;
                        }
                    }

                    if vars.is_empty() {
                        format!("code.push_str(\"{}\\n\");\n", template)
                    } else {
                        // Translate variable names in the template
                        let rust_vars: Vec<String> = vars.iter()
                            .map(|v| translate_js_to_rust(v))
                            .collect();
                        format!("code.push_str(&format!(\"{}\\n\", {}));\n", format_str, rust_vars.join(", "))
                    }
                } else {
                    format!("code.push_str(\"{}\\n\");\n", template)
                }
            } else {
                // Variable or expression: lines.push(result) -> code.push_str(&result);
                format!("code.push_str(&{});\n", value)
            };
            format!("{}{}", indent_str, rust_value)
        }
        Transform::ArrayPushSpread { array_name: _, source_array } => {
            // lines.push(...generateComponent(c)) -> code.push_str(&generate_component(&c));
            let rust_source = translate_js_to_rust(&source_array);
            format!("{}code.push_str(&{});\n", indent_str, rust_source)
        }
        Transform::ArrayJoin { array_name, separator } => {
            // lines.join('\n') -> already a String, no-op or return it
            format!("{}// {} already a String (joined with \"{}\")\n", indent_str, array_name, separator.replace("\n", "\\n"))
        }
        Transform::ForOfLoop { iterator, iterable, body_transforms } => {
            // for (const component of components) { ... }  ->  for component in components { ... }
            // Add & only for struct field access (contains .), not for simple parameters
            let iterable_ref = if iterable.contains('.') {
                format!("&{}", iterable)
            } else {
                iterable.clone()
            };
            let mut result = format!("{}for {} in {} {{\n", indent_str, iterator, iterable_ref);
            for t in body_transforms {
                result.push_str(&generate_transform_code(t, indent + 1));
            }
            result.push_str(&format!("{}}}\n", indent_str));
            result
        }
        Transform::RawJs { code, reason } => {
            format!("{}// TODO: {} - {}\n", indent_str, reason, code)
        }
    }
}

fn babel_to_swc_method(visitor_name: &str) -> &'static str {
    match visitor_name {
        "Program" => "visit_module",
        "FunctionDeclaration" => "visit_fn_decl",
        "ArrowFunctionExpression" => "visit_arrow_expr",
        "CallExpression" => "visit_call_expr",
        "JSXElement" => "visit_jsx_element",
        "VariableDeclarator" => "visit_var_declarator",
        _ => "visit_unknown",
    }
}

fn get_node_param(visitor_name: &str) -> (&'static str, &'static str) {
    match visitor_name {
        "Program" => ("module", "Module"),
        "FunctionDeclaration" => ("node", "FnDecl"),
        "ArrowFunctionExpression" => ("node", "ArrowExpr"),
        "CallExpression" => ("node", "CallExpr"),
        "JSXElement" => ("node", "JSXElement"),
        "VariableDeclarator" => ("node", "VarDeclarator"),
        _ => ("node", "Node"),
    }
}

fn translate_js_to_rust(js_expr: &str) -> String {
    match js_expr {
        // Handle string literals FIRST before function call patterns
        e if e.starts_with('"') && e.ends_with('"') => e.to_string(),
        e if e.starts_with('\'') && e.ends_with('\'') => e.to_string(),
        e if e.contains("path.node.id.name") => "node.ident.sym.to_string()".to_string(),
        e if e.contains("path.node") => e.replace("path.node", "node"),
        e if e.contains("path.parent.type") => "parent_type".to_string(),
        // Translate JavaScript string methods to Rust equivalents
        e if e.contains(".startsWith(") => {
            e.replace(".startsWith(", ".starts_with(")
        }
        e if e.contains(".endsWith(") => {
            e.replace(".endsWith(", ".ends_with(")
        }
        // Translate regex test() - for now, comment as it needs complex translation
        e if e.contains(".test(") => {
            "/* TODO: translate regex test */true".to_string()
        }
        // Translate function calls: generateComponent(component) -> generate_component(&component)
        e if e.contains("(") && e.contains(")") => {
            // Extract function name and args
            if let Some(paren_pos) = e.find('(') {
                let func_part = &e[..paren_pos];
                // Safety check for malformed expressions
                if paren_pos + 1 >= e.len() {
                    return e.to_string();
                }
                let args_part = &e[paren_pos+1..e.len()-1]; // Extract just the args without parens

                // Convert camelCase function name to snake_case
                let rust_func = to_snake_case(func_part);

                if args_part.is_empty() {
                    format!("{}()", rust_func)
                } else {
                    // Split args and translate each one
                    let rust_args: Vec<String> = args_part.split(',')
                        .map(|arg| {
                            let trimmed = arg.trim();
                            // Recursively translate each argument
                            let translated = translate_js_to_rust(trimmed);
                            format!("&{}", translated)
                        })
                        .collect();

                    format!("{}({})", rust_func, rust_args.join(", "))
                }
            } else {
                js_expr.to_string()
            }
        }
        // Translate member access: component.hooks.length -> component.hooks.len()
        e if e.ends_with(".length") => {
            format!("{}.len()", &e[..e.len()-7])
        }
        // Translate struct field names from our metadata format
        e if e.contains(".type") => {
            // hook.type -> hook.hook_type (avoid Rust keyword)
            e.replace(".type", ".hook_type")
        }
        e if e.contains(".stateName") => {
            e.replace(".stateName", ".state_name")
        }
        e if e.contains(".setterName") => {
            e.replace(".setterName", ".setter_name")
        }
        e if e.contains(".initialValue") => {
            e.replace(".initialValue", ".initial_value")
        }
        e if e.contains(".jsxElements") => {
            e.replace(".jsxElements", ".jsx_elements")
        }
        // Translate standalone variable names (not in property access)
        "csharpType" => "csharp_type".to_string(),
        "csharpValue" => "csharp_value".to_string(),
        "jsValue" => "js_value".to_string(),
        "initialValue" => "initial_value".to_string(),
        "state" => "self".to_string(),
        "path" => "node".to_string(),
        _ => js_expr.to_string(),
    }
}

fn translate_condition(js_condition: &str) -> String {
    // Translate JavaScript condition patterns to Rust
    if js_condition.starts_with("t.is") {
        // t.isIdentifier(...) -> matches!(node, Node::Ident(_))
        return "/* condition */".to_string();
    }

    let mut result = js_condition.to_string();

    // Translate truthiness checks on arrays
    // component.hooks && component.hooks.len() > 0  ->  !component.hooks.is_empty() && ...
    // We need to be careful with the order here
    if result.contains(" && ") {
        let parts: Vec<&str> = result.split(" && ").collect();
        let translated_parts: Vec<String> = parts.iter().map(|part| {
            let trimmed = part.trim();
            // Check if this is a simple array/vec reference (no operators)
            if !trimmed.contains("==") && !trimmed.contains("!=") &&
               !trimmed.contains('>') && !trimmed.contains('<') &&
               !trimmed.contains('(') && !trimmed.contains(')') &&
               (trimmed.contains(".hooks") || trimmed.contains(".props") ||
                trimmed.contains(".jsx_elements") || trimmed.ends_with("s")) {
                // This is likely an array truthiness check
                format!("!{}.is_empty()", trimmed)
            } else {
                trimmed.to_string()
            }
        }).collect();
        result = translated_parts.join(" && ");
    }

    // Translate property access
    result = result.replace(".length", ".len()");  // array.length -> array.len()
    result = result.replace(".type", ".hook_type");  // hook.type -> hook.hook_type
    result = result.replace(".stateName", ".state_name");
    result = result.replace(".setterName", ".setter_name");
    result = result.replace(".initialValue", ".initial_value");
    result = result.replace(".jsxElements", ".jsx_elements");
    result = result.replace("path.parent.type", "parent_type");

    // Translate JavaScript string methods to Rust
    result = result.replace(".startsWith(", ".starts_with(");
    result = result.replace(".endsWith(", ".ends_with(");

    // Translate regex test patterns
    // Pattern: REGEX(^\d+$).test(value) -> value.chars().all(|c| c.is_numeric())
    // Pattern: REGEX(^\d+\.\d+$).test(value) -> value.contains('.') && value.parse::<f64>().is_ok()
    if result.contains("REGEX(") && result.contains(").test(") {
        if let Some(regex_start) = result.find("REGEX(") {
            if let Some(regex_end) = result.find(").test(") {
                let pattern = &result[regex_start + 6..regex_end];
                if let Some(test_start) = result.find(".test(") {
                    if let Some(test_end) = result[test_start..].find(")") {
                        let var_name = &result[test_start + 6..test_start + test_end];

                        // Translate common regex patterns
                        let rust_check = match pattern {
                            r"^\d+$" => format!("{}.chars().all(|c| c.is_numeric())", var_name),
                            r"^\d+\.\d+$" | r"^\d+\\\\.\\d+$" => format!("{}.contains('.') && {}.parse::<f64>().is_ok()", var_name, var_name),
                            _ => format!("/* TODO: translate regex /{}/  */ true", pattern),
                        };
                        result = rust_check;
                    }
                }
            }
        }
    }

    // Translate unary ! on strings to .is_empty()
    // Pattern: !variableName || ... -> variableName.is_empty() || ...
    // Pattern: !variableName && ... -> variableName.is_empty() && ...
    // But skip if already contains .is_empty() (from && truthiness translation above)
    if result.starts_with("!") && !result.contains("!(") && !result.contains(".is_empty()") {
        // Find where the variable name ends (space, ||, &&, etc.)
        let var_end = result[1..].find(|c: char| c == ' ' || c == '|' || c == '&')
            .map(|pos| pos + 1)
            .unwrap_or(result.len());

        let var_name = &result[1..var_end];
        let rest = &result[var_end..];
        result = format!("{}.is_empty(){}", var_name, rest);
    }

    // Translate variable names (camelCase -> snake_case)
    result = result.replace("csharpType", "csharp_type");
    result = result.replace("csharpValue", "csharp_value");
    result = result.replace("jsValue", "js_value");
    result = result.replace("initialValue", "initial_value");

    // Translate operators
    result = result.replace("===", "==");
    result = result.replace("!==", "!=");

    result
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_lowercase().next().unwrap());
    }
    result
}

fn generate_fn_decl_visitor() -> String {
    r#"    fn visit_mut_fn_decl(&mut self, node: &mut FnDecl) {
        let name = node.ident.sym.to_string();

        // Check if this is a component (starts with uppercase)
        if name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
            eprintln!("Found component function: {}", name); // Debug
            self.start_component(name);

            // Visit children to collect hooks and JSX
            node.visit_mut_children_with(self);

            self.finish_component();
        } else {
            node.visit_mut_children_with(self);
        }
    }

"#.to_string()
}

fn generate_jsx_element_visitor() -> String {
    r#"    fn visit_mut_jsx_element(&mut self, node: &mut JSXElement) {
        let element_type = match &node.opening.name {
            JSXElementName::Ident(id) => id.sym.to_string(),
            _ => "unknown".to_string(),
        };

        let mut attributes = HashMap::new();
        for attr in &node.opening.attrs {
            if let JSXAttrOrSpread::JSXAttr(jsx_attr) = attr {
                if let JSXAttrName::Ident(name_ident) = &jsx_attr.name {
                    let value = match &jsx_attr.value {
                        Some(JSXAttrValue::Str(s)) => format!("{:?}", s.value), // Use Debug formatting for Wtf8
                        Some(JSXAttrValue::JSXExprContainer(_)) => "<expression>".to_string(),
                        Some(JSXAttrValue::JSXElement(_)) => "<jsx-element>".to_string(),
                        Some(JSXAttrValue::JSXFragment(_)) => "<fragment>".to_string(),
                        None => "true".to_string(),
                    };
                    attributes.insert(name_ident.sym.to_string(), value);
                }
            }
        }

        self.add_jsx_element(JsxElementData {
            element_type,
            attributes,
        });

        node.visit_mut_children_with(self);
    }

"#.to_string()
}

fn generate_var_declarator_visitor() -> String {
    r#"    fn visit_mut_var_declarator(&mut self, node: &mut VarDeclarator) {
        eprintln!("Visiting var declarator: {:?}", node.name);

        // Check for useState hooks with array destructuring
        if let Pat::Array(array_pat) = &node.name {
            if let Some(init) = &mut node.init {
                if let Expr::Call(call) = &mut **init {
                    if let Callee::Expr(expr) = &call.callee {
                        if let Expr::Ident(ident) = &**expr {
                            if &*ident.sym == "useState" {
                                eprintln!("Found useState hook!");
                                let state_name = if let Some(Some(Pat::Ident(id))) = array_pat.elems.get(0) {
                                    id.id.sym.to_string()
                                } else {
                                    "state".to_string()
                                };

                                let setter_name = if let Some(Some(Pat::Ident(id))) = array_pat.elems.get(1) {
                                    Some(id.id.sym.to_string()
)
                                } else {
                                    None
                                };

                                let initial_value = if let Some(arg) = call.args.get(0) {
                                    match &*arg.expr {
                                        Expr::Lit(Lit::Num(n)) => n.value.to_string(),
                                        Expr::Lit(Lit::Str(s)) => format!("{:?}", s.value), // Use Debug for Wtf8
                                        Expr::Lit(Lit::Bool(b)) => b.value.to_string(),
                                        _ => "unknown".to_string(),
                                    }
                                } else {
                                    "undefined".to_string()
                                };

                                self.add_hook(Hook {
                                    hook_type: "useState".to_string(),
                                    state_name,
                                    setter_name,
                                    initial_value,
                                });
                            }
                        }
                    }
                }
            }
        }

        // Check for arrow function components
        if let Pat::Ident(ident) = &node.name {
            let name = ident.id.sym.to_string();

            if let Some(init) = &mut node.init {
                if let Expr::Arrow(_arrow) = &mut **init {
                    if name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                        self.start_component(name);
                        node.visit_mut_children_with(self);
                        self.finish_component();
                        return;
                    }
                }
            }
        }

        node.visit_mut_children_with(self);
    }

"#.to_string()
}

fn generate_main_rs() -> String {
    r#"use swc_common::{sync::Lrc, FileName, SourceMap};
use swc_ecma_ast::*;
use swc_ecma_parser::{lexer::Lexer, Parser, Syntax, TsSyntax};
use swc_ecma_visit::VisitMutWith;
use swc_generated_plugin::ComponentExtractor;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();

    let file_path = if args.len() > 1 {
        &args[1]
    } else {
        "../fixtures/Counter.tsx"
    };

    // Read the source file
    let code = fs::read_to_string(file_path)
        .expect(&format!("Failed to read {}", file_path));

    // Parse the file
    let cm = Lrc::new(SourceMap::default());
    let fm = cm.new_source_file(Lrc::new(FileName::Custom(file_path.into())), code);

    let syntax = Syntax::Typescript(TsSyntax {
        tsx: true,
        ..Default::default()
    });

    let lexer = Lexer::new(syntax, Default::default(), (&*fm).into(), None);
    let mut parser = Parser::new_from(lexer);
    let mut module = parser.parse_module().expect("Parse failed");

    // Run the extractor
    let mut extractor = ComponentExtractor::new();
    module.visit_mut_with(&mut extractor);

    // Output JSON metadata
    let json = serde_json::to_string_pretty(&extractor.components)
        .expect("Failed to serialize");
    println!("{}", json);

    // Generate C# code if components were found
    if !extractor.components.is_empty() {
        let csharp_code = swc_generated_plugin::generate_c_sharp_file(&extractor.components);

        // Write to .cs file
        let cs_path = file_path.replace(".tsx", ".cs").replace(".ts", ".cs").replace(".jsx", ".cs").replace(".js", ".cs");
        fs::write(&cs_path, csharp_code)
            .expect(&format!("Failed to write C# file: {}", cs_path));

        eprintln!("✓ Generated C# file: {}", cs_path);
    }
}
"#.to_string()
}
