use swc_common::{sync::Lrc, FileName, SourceMap};
use swc_ecma_ast::*;
use swc_ecma_parser::{lexer::Lexer, Parser, Syntax, TsSyntax};
use swc_ecma_visit::{Visit, VisitWith};

use std::fs;
use std::path::{Path, PathBuf};

mod metadata;
use metadata::PluginMetadata;

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
                    metadata: analyzer.metadata,
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

    // Try to load metadata file
    let metadata_path = format!("{}/metadata.json",
        Path::new(plugin_path).parent().unwrap().display());
    let metadata = PluginMetadata::load(&metadata_path).ok();

    if let Some(ref meta) = metadata {
        if !json_output {
            println!("✓ Loaded metadata for: {}", meta.plugin.name);
            println!("  {} structs, {} functions",
                meta.structs.len(), meta.functions.len());
        }
    } else if !json_output {
        println!("ℹ No metadata found at {}, using inference mode", metadata_path);
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
        metadata: metadata.as_ref().map(|m| m as &_),
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
            generate_swc_plugin_crate(output_dir, &analyzer.visitor_methods, &analyzer.helper_functions, metadata.as_ref().map(|m| m as &_));

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
/// Generate Rust struct definitions from metadata
fn generate_structs_from_metadata(metadata: &PluginMetadata) -> String {
    let mut code = String::new();

    for (struct_name, struct_def) in &metadata.structs {
        // Add description if available
        if let Some(desc) = &struct_def.description {
            code.push_str(&format!("/// {}\n", desc));
        }

        code.push_str("#[derive(Debug, Clone, Serialize, Deserialize)]\n");
        code.push_str(&format!("pub struct {} {{\n", struct_name));

        for (field_name, field_def) in &struct_def.fields {
            let snake_name = to_snake_case(field_name);
            code.push_str(&format!("    pub {}: {},\n", snake_name, field_def.field_type));
        }

        code.push_str("}\n\n");

        // Generate impl with Default
        code.push_str(&format!("impl Default for {} {{\n", struct_name));
        code.push_str("    fn default() -> Self {\n");
        code.push_str("        Self {\n");

        for (field_name, field_def) in &struct_def.fields {
            let snake_name = to_snake_case(field_name);
            let default_value = if let Some(default) = &field_def.default {
                default.clone()
            } else {
                match field_def.field_type.as_str() {
                    "String" => "String::new()".to_string(),
                    s if s.starts_with("Vec<") => "vec![]".to_string(),
                    "bool" => "false".to_string(),
                    s if s.starts_with("Option<") => "None".to_string(),
                    _ => "Default::default()".to_string(),
                }
            };

            code.push_str(&format!("            {}: {},\n", snake_name, default_value));
        }

        code.push_str("        }\n");
        code.push_str("    }\n");
        code.push_str("}\n\n");
    }

    code
}

fn generate_module_file(helpers: &[&HelperFunction], metadata: Option<&PluginMetadata>) -> String {
    let mut code = String::new();

    code.push_str("// Auto-generated module\n\n");

    // Add common SWC imports needed by generated code
    code.push_str("use swc_ecma_ast::*;\n");
    code.push_str("use swc_common::DUMMY_SP;\n");

    // Import metadata structs if available
    if let Some(meta) = metadata {
        if !meta.structs.is_empty() {
            code.push_str("use crate::{");
            let struct_names: Vec<String> = meta.structs.keys().cloned().collect();
            code.push_str(&struct_names.join(", "));
            code.push_str("};\n");
        }
    }

    code.push_str("\n");

    for helper in helpers {
        // Only include code generation functions (filter out AST helpers and internal functions)
        let mut is_excluded = helper.name.starts_with("_") ||                    // Private functions
                         helper.name == "visit" ||                           // AST visitor helpers
                         helper.name.starts_with("is") && helper.name.len() < 15 || // Short "is" checks like isJSX
                         helper.name == "getComponentName" ||                // Implemented as ComponentExtractor::get_component_name_from_context
                         helper.name == "escapeCSharpString";                // Implemented as ComponentExtractor::escape_c_sharp_string

        // Check metadata for functions to skip
        if let Some(meta) = metadata {
            if let Some(hints) = meta.code_generation_hints.as_ref() {
                if let Some(skip_list) = hints.get("skip_helper_generation") {
                    if let Some(array) = skip_list.as_array() {
                        for item in array {
                            if let Some(func_name) = item.as_str() {
                                if helper.name == func_name {
                                    is_excluded = true;
                                    eprintln!("Skipping helper function '{}' per metadata hint", func_name);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        if !is_excluded {
            code.push_str(&generate_helper_function(helper, metadata));
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

fn generate_swc_plugin_crate(
    output_dir: &str,
    visitor_methods: &[VisitorMethod],
    helper_functions: &[HelperFunction],
    metadata: Option<&PluginMetadata>
) {
    use std::path::Path;
    use std::collections::HashMap;

    // Create directory structure
    let dir = Path::new(output_dir);
    let src_dir = dir.join("src");
    fs::create_dir_all(&src_dir).expect("Failed to create directories");

    // Group helper functions by source module and deduplicate
    let mut modules: HashMap<String, Vec<&HelperFunction>> = HashMap::new();
    for helper in helper_functions {
        let module_name = get_module_name(&helper.source_module);
        let helpers_vec = modules.entry(module_name).or_insert_with(Vec::new);

        // Deduplicate: only add if not already present
        if !helpers_vec.iter().any(|h| h.name == helper.name) {
            helpers_vec.push(helper);
        }
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
        let module_rs = generate_module_file(helpers, metadata);
        let file_path = generators_dir.join(format!("{}.rs", module_name));
        fs::write(&file_path, module_rs).expect("Failed to write module file");
        module_names.push(module_name.clone());
        println!("  - {}/src/generators/{}.rs ({} functions)", output_dir, module_name, helpers.len());
    }

    // Generate generators/mod.rs
    let mod_rs = generate_mod_rs(&module_names);
    fs::write(generators_dir.join("mod.rs"), mod_rs).expect("Failed to write mod.rs");

    // Generate lib.rs (with visitor only, helpers are in modules)
    let lib_rs = generate_lib_rs(visitor_methods, helper_functions, metadata.as_ref().map(|m| m as &_));
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
regex = "1"
once_cell = "1"
"#.to_string()
}

/// Generate a visitor method with full inlining support
fn generate_visitor_method_with_inlining(
    method: &VisitorMethod,
    helpers_map: &std::collections::HashMap<String, &HelperFunction>,
    metadata: Option<&PluginMetadata>,
) -> String {
    let mut code = String::new();

    // Map Babel visitor names to SWC visitor method names
    let swc_method_name = match method.name.as_str() {
        "FunctionDeclaration" => "visit_mut_fn_decl",
        "ArrowFunctionExpression" => "visit_mut_arrow_expr",
        "VariableDeclarator" => "visit_mut_var_declarator",
        "CallExpression" => "visit_mut_call_expr",
        "JSXElement" => "visit_mut_jsx_element",
        _ => {
            eprintln!("Unknown visitor method: {}, skipping", method.name);
            return format!("    // TODO: Unknown visitor method: {}\n", method.name);
        }
    };

    // Determine node parameter type
    let node_type = match method.name.as_str() {
        "FunctionDeclaration" => "FnDecl",
        "ArrowFunctionExpression" => "ArrowExpr",
        "VariableDeclarator" => "VarDeclarator",
        "CallExpression" => "CallExpr",
        "JSXElement" => "JSXElement",
        _ => "Node",
    };

    code.push_str(&format!("    fn {}(&mut self, n: &mut {}) {{\n", swc_method_name, node_type));

    // Generate the visitor body with inlining
    let mut visited = std::collections::HashSet::new();
    for transform in &method.transforms {
        let empty_bindings = std::collections::HashMap::new();
        code.push_str(&generate_transform_with_bindings(
            transform,
            &empty_bindings,
            helpers_map,
            2, // indent level
            &mut visited,
            metadata,
        ));
    }

    // Always traverse children
    code.push_str("        n.visit_mut_children_with(self);\n");
    code.push_str("    }\n\n");

    code
}

fn generate_lib_rs(
    visitor_methods: &[VisitorMethod],
    helper_functions: &[HelperFunction],
    metadata: Option<&PluginMetadata>
) -> String {
    let mut code = String::new();

    // Build helpers map for inlining
    let mut helpers_map: std::collections::HashMap<String, &HelperFunction> = std::collections::HashMap::new();
    for helper in helper_functions {
        helpers_map.insert(helper.name.clone(), helper);
    }

    // Header with imports
    code.push_str(r#"use swc_ecma_ast::*;
use swc_ecma_visit::{VisitMut, VisitMutWith};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

pub mod generators;

"#);

    // Generate struct definitions from metadata or use defaults
    if let Some(meta) = metadata {
        code.push_str("// Structs from metadata\n");
        code.push_str(&generate_structs_from_metadata(meta));
    } else {
        code.push_str("// Default structs (no metadata provided)\n");
        code.push_str(r#"#[derive(Debug, Clone, Serialize, Deserialize)]
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
"#);
    }

    code.push_str(r#"
/// Parent context tracking for Babel path emulation
#[derive(Debug, Clone)]
pub enum ParentContext {
    VarDeclarator(String),  // e.g. `const MyComp = ...`
    FnDecl(String),         // e.g. `function MyComp() {}`
    ExportDefault,
    ExportNamed(String),
    BlockStmt,
    Unknown,
}

impl ParentContext {
    pub fn get_type(&self) -> &'static str {
        match self {
            ParentContext::VarDeclarator(_) => "VariableDeclarator",
            ParentContext::FnDecl(_) => "FunctionDeclaration",
            ParentContext::ExportDefault => "ExportDefaultDeclaration",
            ParentContext::ExportNamed(_) => "ExportNamedDeclaration",
            ParentContext::BlockStmt => "BlockStatement",
            ParentContext::Unknown => "Unknown",
        }
    }

    pub fn get_id_name(&self) -> Option<&str> {
        match self {
            ParentContext::VarDeclarator(name) => Some(name),
            ParentContext::FnDecl(name) => Some(name),
            ParentContext::ExportNamed(name) => Some(name),
            _ => None,
        }
    }
}

pub struct ComponentExtractor {
    pub components: Vec<Component>,
    current_component: Option<Component>,
    inside_component: bool,
    /// Stack of parent nodes for context tracking (emulates Babel's path.parent)
    parent_stack: Vec<ParentContext>,
}

impl ComponentExtractor {
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
            current_component: None,
            inside_component: false,
            parent_stack: Vec::new(),
        }
    }

    fn get_parent(&self) -> Option<&ParentContext> {
        self.parent_stack.last()
    }

    fn push_parent(&mut self, context: ParentContext) {
        self.parent_stack.push(context);
    }

    fn pop_parent(&mut self) {
        self.parent_stack.pop();
    }

    fn start_component(&mut self, name: String) {
        self.inside_component = true;
        self.current_component = Some(Component {
            name,
            hooks: Vec::new(),
            jsx_elements: Vec::new(),
            props: Vec::new(),
            local_variables: Vec::new(),
            helper_functions: Vec::new(),
        });
    }

    fn finish_component(&mut self) {
        if let Some(comp) = self.current_component.take() {
            self.components.push(comp);
        }
        self.inside_component = false;
    }

    fn add_hook(&mut self, hook: Hook) {
        if let Some(ref mut comp) = self.current_component {
            comp.hooks.push(hook);
        }
    }

    fn add_jsx_element(&mut self, element: JsxElement) {
        if let Some(ref mut comp) = self.current_component {
            comp.jsx_elements.push(element);
        }
    }

    // Helper method to get component name from current context
    // Emulates Babel's path.node.id and path.parent logic
    fn get_component_name_from_context(&self, node_has_id: bool, node_id_name: Option<&str>) -> Option<String> {
        // If current node has an id, use it
        if node_has_id {
            if let Some(name) = node_id_name {
                return Some(name.to_string());
            }
        }

        // Check parent context
        if let Some(parent) = self.get_parent() {
            match parent {
                ParentContext::VarDeclarator(name) => return Some(name.clone()),
                ParentContext::ExportNamed(name) => {
                    // For export named, check if node has id, otherwise return None
                    if node_has_id && node_id_name.is_some() {
                        return node_id_name.map(|s| s.to_string());
                    }
                    return None;
                }
                _ => {}
            }
        }

        None
    }

    // Helper to escape C# strings
    fn escape_c_sharp_string(&self, s: &str) -> String {
        s.replace('\\', "\\\\")
         .replace('"', "\\\"")
         .replace('\n', "\\n")
         .replace('\r', "\\r")
         .replace('\t', "\\t")
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

    // Use fallback hardcoded visitors (properly structured with flat visitor pattern)
    // TODO: Refactor generate_visitor_method_with_inlining to properly flatten path.traverse()
    if false && !visitor_methods.is_empty() {
        eprintln!("Generating {} visitor methods with inlining support", visitor_methods.len());
        for method in visitor_methods {
            code.push_str(&generate_visitor_method_with_inlining(method, &helpers_map, metadata));
        }
    } else {
        // Fallback to hardcoded visitors
        eprintln!("Using fallback hardcoded visitors with proper flat visitor pattern");
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
    }

    code.push_str("}\n");

    code
}

/// Infer return type when no metadata available
fn infer_return_type(helper: &HelperFunction, has_map_filter: bool, has_string_building: bool) -> String {
    if has_map_filter {
        "Vec<_>".to_string()  // map/filter returns a vector
    } else if helper.name.starts_with("infer") {
        "&'static str".to_string()  // inferCSharpType returns literals
    } else if helper.name.starts_with("convert") || has_string_building {
        "String".to_string()  // convertToCSharp returns params, string builders return String
    } else if helper.name.contains("escape") || helper.name.contains("Escape") {
        "String".to_string()  // escape functions use .replace() which returns String
    } else {
        "&'static str".to_string()
    }
}

/// Generate inferred parameters when no metadata available
fn generate_inferred_params(helper: &HelperFunction) -> String {
    if helper.params.is_empty() {
        String::new()
    } else {
        helper.params.iter()
            .map(|p| {
                // Convert parameter name to snake_case and infer type
                let param_name = to_snake_case(p);

                // Infer type from usage in function body
                let param_type = infer_param_type_from_usage(&param_name, &helper.transforms);

                format!("{}: {}", param_name, param_type)
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn infer_param_type_from_usage(param_name: &str, transforms: &[Transform]) -> &'static str {
    // Check how the parameter is used in the function body
    for transform in transforms {
        let usage = match transform {
            Transform::Conditional { condition, .. } => Some(condition.as_str()),
            Transform::VariableDeclaration { value, .. } => Some(value.as_str()),
            Transform::ReturnStmt { value } => Some(value.as_str()),
            _ => None,
        };

        if let Some(code) = usage {
            if code.contains(param_name) {
                // Check for SWC AST type patterns
                if code.contains(&format!("matches!({}, Expr::", param_name)) {
                    return "&Expr";
                }
                if code.contains(&format!("matches!({}, TsType::", param_name)) {
                    return "&TsType";
                }
                if code.contains(&format!("matches!({}, JSXAttrValue::", param_name)) ||
                   code.contains(&format!("matches!({}, Lit::", param_name)) {
                    return "&JSXAttrValue";
                }
                if code.contains(&format!("{}.len()", param_name)) || code.contains(&format!("&{}", param_name)) {
                    if param_name.ends_with("s") || param_name == "components" {
                        return "&[Component]";
                    }
                }
            }
        }
    }

    // Fallback based on name patterns
    if param_name.contains("component") && !param_name.ends_with("s") {
        "&Component"
    } else if param_name.ends_with("s") || param_name == "components" {
        "&[Component]"
    } else {
        "&str"
    }
}

/// Generate a Rust helper function from detected transforms
fn generate_helper_function(helper: &HelperFunction, metadata: Option<&PluginMetadata>) -> String {
    let mut code = String::new();
    let func_name = to_snake_case(&helper.name);

    // Detect if this function builds a string (has ArrayPush transforms)
    let has_string_building = helper.transforms.iter().any(|t| {
        matches!(t, Transform::ArrayPush { .. } | Transform::ArrayPushSpread { .. } | Transform::ArrayJoin { .. })
    });

    // Generate Rust parameters from JavaScript parameters, using metadata if available
    let rust_params = if let Some(meta) = metadata {
        if let Some(func_def) = meta.get_function_signature(&helper.name) {
            // Use metadata signature
            func_def.params.iter()
                .map(|p| format!("{}: {}", to_snake_case(&p.name), p.param_type))
                .collect::<Vec<_>>()
                .join(", ")
        } else {
            // Fallback to inference
            generate_inferred_params(helper)
        }
    } else {
        generate_inferred_params(helper)
    };

    // Check if this is a map/filter pattern
    let has_map_filter = helper.transforms.get(0).map_or(false, |t| {
        if let Transform::ReturnStmt { value } = t {
            value.contains(".map(expr)") && value.contains(".filter(Boolean)")
        } else {
            false
        }
    });

    // Get return type from metadata or infer it
    let return_type = if let Some(meta) = metadata {
        if let Some(func_def) = meta.get_function_signature(&helper.name) {
            func_def.returns.clone()
        } else {
            infer_return_type(helper, has_map_filter, has_string_building)
        }
    } else {
        infer_return_type(helper, has_map_filter, has_string_building)
    };

    // Generate function signature (public for C# generation functions)
    code.push_str(&format!("pub fn {}({}) -> {} {{\n", func_name, rust_params, return_type));

    if has_string_building {
        code.push_str("    let mut code = String::new();\n\n");
    }

    // Check if this is a map/filter pattern
    let has_map_filter = helper.transforms.get(0).map_or(false, |t| {
        if let Transform::ReturnStmt { value } = t {
            value.contains(".map(expr)") && value.contains(".filter(Boolean)")
        } else {
            false
        }
    });

    // Generate body from transforms
    if has_map_filter {
        // Special handling for map/filter pattern
        if let Some(Transform::ReturnStmt { value }) = helper.transforms.get(0) {
            // Extract array name
            if let Some(map_pos) = value.find(".map(") {
                let array_name = &value[..map_pos];

                code.push_str(&format!("    {}.iter().filter_map(|item| {{\n", array_name));

                // Generate the body transforms (skip the first ReturnStmt)
                for transform in helper.transforms.iter().skip(1) {
                    code.push_str(&generate_transform_code_smart(transform, 2, "Option<_>", metadata));
                }

                code.push_str("    }).collect()\n");
            }
        }
    } else {
        // Normal transform generation with parameter substitution
        // Build param bindings: original_name -> snake_case_name
        // SKIP parameters that have metadata mappings (they're handled in translate_js_to_rust_with_metadata)
        let mut param_bindings = std::collections::HashMap::new();

        for param in &helper.params {
            // Check if metadata has a mapping for this parameter
            let has_metadata_mapping = if let Some(meta) = metadata {
                meta.translate_babel_pattern(param).is_some()
            } else {
                false
            };

            if !has_metadata_mapping {
                // No metadata mapping, use snake_case conversion
                let snake_case_param = to_snake_case(param);
                if param != &snake_case_param {
                    param_bindings.insert(param.clone(), snake_case_param);
                }
            }
            // If has_metadata_mapping, skip - translation already handled
        }

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

            // Generate with parameter substitution
            let transform_code = generate_transform_code_smart(transform, 1, &return_type, metadata);
            // Apply parameter name substitution
            let mut substituted_code = transform_code;
            for (original, snake) in &param_bindings {
                substituted_code = substituted_code.replace(original, snake);
            }
            code.push_str(&substituted_code);
        }
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
struct BabelVisitorDetector<'a> {
    visitor_methods: Vec<VisitorMethod>,
    helper_functions: Vec<HelperFunction>,
    required_modules: Vec<RequiredModule>,
    current_file_path: String,
    current_method: Option<String>,
    metadata: Option<&'a PluginMetadata>,
}

#[derive(Debug)]
struct VisitorBodyAnalyzer<'a> {
    transforms: Vec<Transform>,
    metadata: Option<&'a PluginMetadata>,
}

impl<'a> Default for VisitorBodyAnalyzer<'a> {
    fn default() -> Self {
        Self {
            transforms: Vec::new(),
            metadata: None,
        }
    }
}

impl<'a> VisitorBodyAnalyzer<'a> {
    /// Infer struct name from variable name and fields, using metadata if available
    fn infer_struct_name(&self, var_name: &str, fields: &[String]) -> String {
        // Try metadata first - find struct that has matching fields
        if let Some(meta) = self.metadata {
            // Extract field names from "field: value" format
            let field_names: Vec<String> = fields.iter()
                .filter_map(|f| f.split(':').next().map(|s| s.trim().to_string()))
                .collect();

            // Find best matching struct in metadata
            let mut best_match = None;
            let mut best_score = 0;

            for (struct_name, struct_def) in &meta.structs {
                let meta_fields: Vec<String> = struct_def.fields.keys()
                    .map(|k| to_snake_case(k))
                    .collect();

                // Count how many fields match
                let score = field_names.iter()
                    .filter(|f| meta_fields.contains(f))
                    .count();

                if score > best_score {
                    best_score = score;
                    best_match = Some(struct_name.as_str());
                }
            }

            if let Some(matched) = best_match {
                if best_score > 0 {
                    return matched.to_string();
                }
            }
        }

        // Fallback to heuristic matching
        let normalized = var_name.to_lowercase().replace("_", "");

        let struct_name = match normalized.as_str() {
            "component" | "comp" => "Component",
            "hook" | "hookinfo" => "Hook",
            "prop" | "propdata" => "Prop",
            "variable" | "varinfo" | "localvariable" => "Variable",
            "helper" | "helperfunc" | "helperfunction" => "HelperFunc",
            "element" | "jsxelement" | "jsx" => "JsxElement",
            _ => {
                // Try to infer from fields
                if fields.iter().any(|f| f.starts_with("hook_type:")) {
                    "Hook"
                } else if fields.iter().any(|f| f.starts_with("props:") || f.starts_with("hooks:")) {
                    "Component"
                } else if fields.iter().any(|f| f.starts_with("has_initializer:")) {
                    "Variable"
                } else {
                    // Capitalize first letter as fallback
                    return var_name.chars().next()
                        .map(|c| c.to_uppercase().collect::<String>())
                        .unwrap_or_default() + &var_name[1..];
                }
            }
        };

        struct_name.to_string()
    }

    fn extract_init_value(&mut self, init: &Option<Box<Expr>>) -> String {
        self.extract_init_value_with_context(init, "value")
    }

    fn extract_init_value_with_context(&mut self, init: &Option<Box<Expr>>, var_name: &str) -> String {
        if let Some(init) = init {
            match &**init {
                Expr::Member(member) => self.extract_member_path(member),
                Expr::Call(call) => {
                    // Extract full function call with arguments
                    if let Some(Transform::FunctionCall { name, args }) = self.detect_function_call(call) {
                        let rust_name = convert_identifier(&name, self.metadata.as_ref().map(|m| m as &_));
                        if args.is_empty() {
                            format!("{}()", rust_name)
                        } else {
                            // Translate args and add & for references
                            let rust_args: Vec<String> = args.iter()
                                .map(|arg| {
                                    let translated = convert_identifier(arg, self.metadata.as_ref().map(|m| m as &_));
                                    format!("&{}", translated)
                                })
                                .collect();
                            format!("{}({})", rust_name, rust_args.join(", "))
                        }
                    } else {
                        "call()".to_string()
                    }
                }
                Expr::Lit(Lit::Str(s)) => format!("{:?}", s.value),
                Expr::Lit(Lit::Num(n)) => n.value.to_string(),
                Expr::Lit(Lit::Bool(b)) => b.value.to_string(),
                Expr::Ident(ident) => convert_identifier(&ident.sym.to_string(), self.metadata.as_ref().map(|m| m as &_)),
                Expr::Object(obj) => {
                    // Translate object literal to Rust struct initialization
                    // For now, we'll generate a simplified struct literal
                    let mut fields = Vec::new();
                    for prop in &obj.props {
                        if let PropOrSpread::Prop(prop) = prop {
                            if let Prop::KeyValue(kv) = &**prop {
                                let mut key = match &kv.key {
                                    PropName::Ident(id) => convert_identifier(&String::from_utf8_lossy(id.sym.as_bytes()), self.metadata.as_ref().map(|m| m as &_)),
                                    PropName::Str(s) => convert_identifier(&String::from_utf8_lossy(s.value.as_bytes()), self.metadata.as_ref().map(|m| m as &_)),
                                    _ => "unknown".to_string(),
                                };
                                // Avoid Rust keywords
                                if key == "type" {
                                    key = "hook_type".to_string();
                                }
                                let value = self.extract_init_value_with_context(&Some(kv.value.clone()), &key);
                                fields.push(format!("{}: {}", key, value));
                            }
                        }
                    }
                    if fields.is_empty() {
                        "Default::default()".to_string()
                    } else {
                        let struct_name = self.infer_struct_name(var_name, &fields);

                        // Filter fields based on metadata for this struct
                        let filtered_fields = if let Some(meta) = self.metadata {
                            if let Some(struct_def) = meta.structs.get(&struct_name) {
                                let valid_fields: std::collections::HashSet<String> = struct_def.fields.keys()
                                    .map(|k| to_snake_case(k))
                                    .collect();

                                fields.into_iter()
                                    .filter(|f| {
                                        let field_name = f.split(':').next().unwrap_or("").trim();
                                        valid_fields.contains(field_name)
                                    })
                                    .collect::<Vec<_>>()
                            } else {
                                fields
                            }
                        } else {
                            fields
                        };

                        if filtered_fields.is_empty() {
                            format!("{}::default()", struct_name)
                        } else {
                            format!("{} {{ {} }}", struct_name, filtered_fields.join(", "))
                        }
                    }
                },
                Expr::Array(arr) => {
                    // Translate array literal to Rust vec![]
                    if arr.elems.is_empty() {
                        "vec![]".to_string()
                    } else {
                        let elements: Vec<String> = arr.elems.iter()
                            .filter_map(|elem| elem.as_ref())
                            .map(|elem| self.extract_init_value(&Some(Box::new((*elem.expr).clone()))))
                            .collect();
                        format!("vec![{}]", elements.join(", "))
                    }
                },
                Expr::Bin(_) => "/* TODO: binary expression */false".to_string(),
                Expr::Unary(_) => "/* TODO: unary expression */false".to_string(),
                Expr::Cond(_) => "/* TODO: conditional expression */Default::default()".to_string(),
                _ => format!("/* TODO: unsupported expr type {} */Default::default()",
                           std::any::type_name::<Expr>()),
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
            Expr::Unary(unary) => {
                let op = match unary.op {
                    UnaryOp::Bang => "!",
                    UnaryOp::Minus => "-",
                    UnaryOp::Plus => "+",
                    _ => "?",
                };
                let arg = self.extract_expr_string(&unary.arg);
                format!("{}{}", op, arg)
            }
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

impl<'a> Visit for VisitorBodyAnalyzer<'a> {
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
                    let value = self.extract_init_value_with_context(&decl.init, &var_name);

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
        let mut body_analyzer = VisitorBodyAnalyzer {
            transforms: Vec::new(),
            metadata: self.metadata,
        };
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

        let mut then_analyzer = VisitorBodyAnalyzer {
            transforms: Vec::new(),
            metadata: self.metadata,
        };
        if_stmt.cons.visit_with(&mut then_analyzer);

        let else_transforms = if let Some(alt) = &if_stmt.alt {
            let mut else_analyzer = VisitorBodyAnalyzer {
                transforms: Vec::new(),
                metadata: self.metadata,
            };
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

impl<'a> Visit for BabelVisitorDetector<'a> {
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

            let mut body_analyzer = VisitorBodyAnalyzer {
                transforms: Vec::new(),
                metadata: self.metadata,
            };

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
                                                    let mut body_analyzer = VisitorBodyAnalyzer {
                                                        transforms: Vec::new(),
                                                        metadata: self.metadata,
                                                    };

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
                                                    let mut body_analyzer = VisitorBodyAnalyzer {
                                                        transforms: Vec::new(),
                                                        metadata: self.metadata,
                                                    };

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
        body_code.push_str(&generate_transform_code(transform, 2, None));
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

fn generate_transform_code_smart(transform: &Transform, indent: usize, return_type: &str, metadata: Option<&PluginMetadata>) -> String {
    let indent_str = " ".repeat(indent * 4);

    match transform {
        Transform::ReturnStmt { value } => {
            if value.contains(".join(") || value == "lines" || value == "code" {
                String::new()
            } else if value == "expr" {
                // Skip generic 'expr' placeholders
                String::new()
            } else {
                let rust_value = translate_js_to_rust_with_metadata(value, metadata);

                let final_value = if return_type == "Option<_>" {
                    // For Option return type (filter_map), wrap in Some()
                    if rust_value == "expr" {
                        "None".to_string()
                    } else {
                        format!("Some({})", rust_value)
                    }
                } else if return_type == "String" {
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
            let cond = translate_condition(condition, metadata);

            // Skip conditionals with false conditions (dead code)
            if cond == "false" {
                return String::new();
            }

            // Check if this is a matches!() pattern that should use if let instead
            // Pattern: matches!(value, Expr::Lit(_))
            let use_if_let = cond.starts_with("matches!(");
            let (if_let_pattern, var_name) = if use_if_let {
                // Extract: matches!(value, Expr::Lit(_)) -> ("Expr::Lit(ref lit)", "value")
                if let Some(start) = cond.find('(') {
                    if let Some(comma) = cond[start..].find(',') {
                        let var = cond[start+1..start+comma].trim();
                        let pattern_start = start + comma + 1;
                        if let Some(end) = cond[pattern_start..].rfind(')') {
                            let pattern = cond[pattern_start..pattern_start+end].trim();
                            // Simply replace wildcards with ref bindings - don't nest!
                            // matches!(value, JSXAttrValue::Lit(Lit::Str(_))) stays JSXAttrValue::Lit(Lit::Str(ref str_lit))
                            let enhanced_pattern = pattern
                                .replace("Lit::Str(_)", "Lit::Str(ref str_lit)")
                                .replace("Lit::Num(_)", "Lit::Num(ref num_lit)")
                                .replace("Lit::Bool(_)", "Lit::Bool(ref bool_lit)")
                                .replace("Lit::Null(_)", "Lit::Null(_)")
                                .replace("Expr::Ident(_)", "Expr::Ident(ref ident)")
                                .replace("Expr::Member(_)", "Expr::Member(ref member_expr)")
                                .replace("Expr::Call(_)", "Expr::Call(ref call_expr)")
                                .replace("Expr::Array(_)", "Expr::Array(ref array_expr)")
                                .replace("Expr::Object(_)", "Expr::Object(ref obj_expr)")
                                .replace("JSXAttrValue::JSXExprContainer(_)", "JSXAttrValue::JSXExprContainer(ref jsx_expr_container)")
                                .replace("(_)", "(ref lit)");
                            (Some(enhanced_pattern), Some(var.to_string()))
                        } else {
                            (None, None)
                        }
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };

            // Check if this is an is_none() check on an Option parameter
            let is_none_check = cond.ends_with(".is_none()");
            let none_var_name = if is_none_check {
                cond.trim_end_matches(".is_none()").trim()
            } else {
                ""
            };

            let mut result = if is_none_check && !none_var_name.is_empty() {
                // Generate early return for None case
                let mut s = format!("{}if {}.is_none() {{\n", indent_str, none_var_name);
                for t in then_transforms {
                    s.push_str(&generate_transform_code_smart(t, indent + 1, return_type, metadata));
                }
                s.push_str(&format!("{}}}\n", indent_str));
                // Add unwrap after the None check
                s.push_str(&format!("{}let {} = {}.unwrap();\n", indent_str, none_var_name, none_var_name));
                s
            } else if let (Some(pattern), Some(var)) = (if_let_pattern, var_name) {
                // Use if let instead of matches!
                // Extract the innermost variable name from pattern for substitution
                // Pattern like "Expr::Lit(Lit::Bool(ref bool_lit))" -> extract "bool_lit"
                let inner_var = if let Some(ref_pos) = pattern.rfind("ref ") {
                    let after_ref = &pattern[ref_pos + 4..];
                    if let Some(paren_pos) = after_ref.find(')') {
                        after_ref[..paren_pos].trim()
                    } else {
                        ""
                    }
                } else {
                    ""
                };

                let mut s = format!("{}if let {} = {} {{\n", indent_str, pattern, var);
                for t in then_transforms {
                    let mut code = generate_transform_code_smart(t, indent + 1, return_type, metadata);
                    // Substitute var.field with inner_var.field
                    // Also fix metadata-mapped variable names that are wrong for this context
                    if !inner_var.is_empty() {
                        code = code.replace(&format!("{}.value", var), &format!("{}.value", inner_var));
                        code = code.replace(&format!("{}.elements", var), &format!("{}.elems", inner_var));
                        code = code.replace(&format!("{}.properties", var), &format!("{}.props", inner_var));
                        // Fix wrongly-mapped variable names (e.g., num_lit in bool block should be bool_lit)
                        // For str_lit, also ensure we have the utf8 conversion
                        if inner_var == "str_lit" {
                            code = code.replace("num_lit.value", &format!("String::from_utf8_lossy({}.value.as_bytes())", inner_var));
                        } else {
                            code = code.replace("num_lit.value", &format!("{}.value", inner_var));
                        }
                        code = code.replace("str_lit.value", &format!("String::from_utf8_lossy({}.value.as_bytes())", inner_var));
                        code = code.replace("bool_lit.value", &format!("{}.value", inner_var));
                        // Fix variables that should reference the destructured inner variable
                        code = code.replace("ident.sym", &format!("String::from_utf8_lossy({}.sym.as_bytes())", inner_var));
                        code = code.replace("array_expr.elems", &format!("{}.elems", inner_var));
                        code = code.replace("obj_expr.props", &format!("{}.props", inner_var));
                        // Convert JavaScript .toString() to Rust .to_string()
                        code = code.replace(".toString()", ".to_string()");
                    }
                    s.push_str(&code);
                }
                s.push_str(&format!("{}}}", indent_str));

                if let Some(else_block) = else_transforms {
                    s.push_str(" else {\n");
                    for t in else_block {
                        s.push_str(&generate_transform_code_smart(t, indent + 1, return_type, metadata));
                    }
                    s.push_str(&format!("{}}}", indent_str));
                }
                s.push('\n');
                s
            } else {
                let mut s = format!("{}if {} {{\n", indent_str, cond);
                for t in then_transforms {
                    s.push_str(&generate_transform_code_smart(t, indent + 1, return_type, metadata));
                }
                s.push_str(&format!("{}}}", indent_str));

                if let Some(else_block) = else_transforms {
                    s.push_str(" else {\n");
                    for t in else_block {
                        s.push_str(&generate_transform_code_smart(t, indent + 1, return_type, metadata));
                    }
                    s.push_str(&format!("{}}}", indent_str));
                }
                s.push('\n');
                s
            };
            result
        }

        Transform::ForOfLoop { iterator, iterable, body_transforms } => {
            // Apply metadata mapping to iterable
            let mapped_iterable = translate_js_to_rust_with_metadata(iterable, metadata);
            let iterable_ref = if mapped_iterable.contains('.') {
                format!("&{}", mapped_iterable)
            } else {
                mapped_iterable.clone()
            };
            let mut result = format!("{}for {} in {} {{\n", indent_str, iterator, iterable_ref);
            for t in body_transforms {
                result.push_str(&generate_transform_code_smart(t, indent + 1, return_type, metadata));
            }
            result.push_str(&format!("{}}}\n", indent_str));
            result
        }

        // Fallback to legacy for other types
        _ => generate_transform_code(transform, indent, metadata)
    }
}

/// Inline a helper function call by substituting its body with argument bindings
fn inline_helper_call(
    helper_name: &str,
    args: &[String],
    helpers_map: &std::collections::HashMap<String, &HelperFunction>,
    indent: usize,
    visited: &mut std::collections::HashSet<String>,
    metadata: Option<&PluginMetadata>,
) -> String {
    let mut output = String::new();

    // Prevent infinite recursion
    if visited.contains(helper_name) {
        output.push_str(&format!("{}// Recursive call to {} detected, skipping inline\n", " ".repeat(indent * 4), helper_name));
        return output;
    }

    visited.insert(helper_name.to_string());

    if let Some(helper) = helpers_map.get(helper_name) {
        // Create argument bindings: param_name -> actual_value
        let mut arg_bindings = std::collections::HashMap::new();
        for (i, param) in helper.params.iter().enumerate() {
            if let Some(arg_value) = args.get(i) {
                arg_bindings.insert(param.clone(), arg_value.clone());
            }
        }

        // Generate inlined code with substitutions
        for transform in &helper.transforms {
            output.push_str(&generate_transform_with_bindings(transform, &arg_bindings, helpers_map, indent, visited, metadata));
        }
    } else {
        output.push_str(&format!("{}// TODO: helper '{}' not found in helpers map\n", " ".repeat(indent * 4), helper_name));
    }

    visited.remove(helper_name);
    output
}

/// Generate transform code with argument substitution for inlining
fn generate_transform_with_bindings(
    transform: &Transform,
    arg_bindings: &std::collections::HashMap<String, String>,
    helpers_map: &std::collections::HashMap<String, &HelperFunction>,
    indent: usize,
    visited: &mut std::collections::HashSet<String>,
    metadata: Option<&PluginMetadata>,
) -> String {
    let indent_str = " ".repeat(indent * 4);

    match transform {
        Transform::FunctionCall { name, args } => {
            let rust_name = to_snake_case(name);

            // Check if this is a known helper that should be inlined
            if helpers_map.contains_key(name.as_str()) {
                // Translate and substitute arguments before inlining
                let substituted_args: Vec<String> = args.iter()
                    .map(|arg| {
                        // First translate to Rust, then apply current bindings
                        let rust_arg = translate_js_to_rust(arg);
                        substitute_arg(&rust_arg, arg_bindings)
                    })
                    .collect();

                // Inline the helper function
                return inline_helper_call(name, &substituted_args, helpers_map, indent, visited, metadata);
            }

            // Check if this is a ComponentExtractor method (getComponentName, etc.)
            let is_helper_method = rust_name == "get_component_name" ||
                                   rust_name == "escape_c_sharp_string" ||
                                   rust_name == "ts_type_to_c_sharp_type" ||
                                   rust_name == "infer_type";

            if is_helper_method {
                // Translate to self method call
                match rust_name.as_str() {
                    "get_component_name" => {
                        // getComponentName(path) -> self.get_component_name_from_context(node.ident.is_some(), node.ident.as_ref().map(|i| i.sym.as_ref()))
                        format!("{}let component_name = self.get_component_name_from_context(n.ident.is_some(), n.ident.as_ref().map(|i| i.sym.as_ref()));\n", indent_str)
                    }
                    "escape_c_sharp_string" => {
                        let rust_args = args.iter()
                            .map(|a| substitute_arg(&translate_js_to_rust(a), arg_bindings))
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!("{}self.escape_c_sharp_string({});\n", indent_str, rust_args)
                    }
                    _ => {
                        format!("{}// TODO: translate {}(...);\n", indent_str, rust_name)
                    }
                }
            } else {
                // Regular function call
                let rust_args = args.iter()
                    .map(|a| substitute_arg(&translate_js_to_rust(a), arg_bindings))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}{}({});\n", indent_str, rust_name, rust_args)
            }
        }

        Transform::VariableDeclaration { name, value, is_destructured } => {
            // Check metadata mappings first, then fall back to snake_case
            let rust_name = if let Some(meta) = metadata {
                if let Some(mapped) = meta.translate_babel_pattern(name) {
                    mapped.to_string()
                } else {
                    to_snake_case(name)
                }
            } else {
                to_snake_case(name)
            };
            let rust_value = substitute_arg(&translate_js_to_rust_with_metadata(value, metadata), arg_bindings);
            if *is_destructured {
                format!("{}// Destructured: let {} = {};\n", indent_str, rust_name, rust_value)
            } else {
                // Add type annotation for empty vecs
                if rust_value == "vec![]" {
                    format!("{}let {}: Vec<String> = {};\n", indent_str, rust_name, rust_value)
                } else {
                    format!("{}let {} = {};\n", indent_str, rust_name, rust_value)
                }
            }
        }

        Transform::Conditional { condition, then_transforms, else_transforms } => {
            let cond = substitute_arg(&translate_condition(condition, metadata), arg_bindings);
            let mut result = format!("{}if {} {{\n", indent_str, cond);
            for t in then_transforms {
                result.push_str(&generate_transform_with_bindings(t, arg_bindings, helpers_map, indent + 1, visited, metadata));
            }
            result.push_str(&format!("{}}}", indent_str));

            if let Some(else_block) = else_transforms {
                result.push_str(" else {\n");
                for t in else_block {
                    result.push_str(&generate_transform_with_bindings(t, arg_bindings, helpers_map, indent + 1, visited, metadata));
                }
                result.push_str(&format!("{}}}", indent_str));
            }
            result.push('\n');
            result
        }

        _ => {
            // Fallback to regular generation for other transform types
            generate_transform_code(transform, indent, None)
        }
    }
}

/// Substitute argument references in expressions
fn substitute_arg(expr: &str, bindings: &std::collections::HashMap<String, String>) -> String {
    let mut result = expr.to_string();

    // Replace parameter names with actual values
    // Sort by length (longest first) to avoid partial replacements
    let mut sorted_bindings: Vec<_> = bindings.iter().collect();
    sorted_bindings.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

    for (param, value) in sorted_bindings {
        // Only replace whole words (use word boundaries)
        result = result.replace(param.as_str(), value);
    }

    result
}

fn generate_transform_code(transform: &Transform, indent: usize, metadata: Option<&PluginMetadata>) -> String {
    let indent_str = " ".repeat(indent * 4);

    match transform {
        Transform::VariableDeclaration { name, value, is_destructured } => {
            let rust_name = if let Some(meta) = metadata {
                if let Some(mapped) = meta.translate_babel_pattern(name) {
                    mapped.to_string()
                } else {
                    to_snake_case(name)
                }
            } else {
                to_snake_case(name)
            };
            let rust_value = translate_js_to_rust_with_metadata(value, metadata);
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

            // Check if this is a helper function that should be a self method call
            // Helper functions from imported modules should use self.method_name()
            let is_helper_method = rust_name == "get_component_name" ||
                                   rust_name == "escape_c_sharp_string" ||
                                   rust_name == "ts_type_to_c_sharp_type" ||
                                   rust_name == "infer_type";

            if is_helper_method {
                // Translate to self method call with proper context handling
                match rust_name.as_str() {
                    "get_component_name" => {
                        // getComponentName(path) -> self.get_component_name_from_context(node.ident.is_some(), node.ident.as_ref().map(|i| i.sym.as_ref()))
                        // Since we're in a visitor, the current node is available
                        format!("{}self.get_component_name_from_context(n.ident.is_some(), n.ident.as_ref().map(|i| i.sym.as_ref()));\n", indent_str)
                    }
                    "escape_c_sharp_string" => {
                        format!("{}self.escape_c_sharp_string({});\n", indent_str, rust_args)
                    }
                    _ => {
                        // Other helpers - call as regular functions for now
                        format!("{}{}({});\n", indent_str, rust_name, rust_args)
                    }
                }
            } else {
                format!("{}{}({});\n", indent_str, rust_name, rust_args)
            }
        }
        Transform::MetadataAssignment { key, value_expr } => {
            format!("{}self.metadata.insert(\"{}\", {});\n", indent_str, key, value_expr)
        }
        Transform::Conditional { condition, then_transforms, else_transforms } => {
            let rust_condition = translate_condition(condition, metadata);
            let mut result = format!("{}if {} {{\n", indent_str, rust_condition);
            for t in then_transforms {
                result.push_str(&generate_transform_code(t, indent + 1, metadata));
            }
            result.push_str(&format!("{}}}", indent_str));

            if let Some(else_t) = else_transforms {
                result.push_str(" else {\n");
                for t in else_t {
                    result.push_str(&generate_transform_code(t, indent + 1, metadata));
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
            let rust_condition = translate_condition(condition, metadata);
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

            // Escape quotes and braces in parts (but not the {} placeholders we add)
            let format_str = parts.iter().enumerate()
                .map(|(i, part)| {
                    // Escape backslashes, quotes, and braces in the user's string content
                    let escaped_part = part
                        .replace("\\", "\\\\")
                        .replace("\"", "\\\"")
                        .replace("{", "{{{{")  // {{ in source becomes {{{{ in format string to produce {{
                        .replace("}", "}}}}"); // }} in source becomes }}}} in format string to produce }}
                    if i < exprs.len() {
                        // Add format placeholder {} after this part
                        format!("{}{{}}", escaped_part)
                    } else {
                        escaped_part
                    }
                })
                .collect::<Vec<_>>()
                .join("");

            let rust_exprs = exprs.iter()
                .map(|e| {
                    let translated = translate_js_to_rust_with_metadata(e, metadata);
                    // If the expression contains .value and it's an atom type, wrap with utf8_lossy
                    if translated.contains("str_lit.value") || translated.contains("ident.sym") {
                        format!("String::from_utf8_lossy({}.as_bytes())", translated)
                    } else {
                        translated
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");

            // Always generate return statements for template literals (safer default)
            if exprs.is_empty() {
                format!("{}return \"{}\";\n", indent_str, format_str)
            } else {
                format!("{}return format!(\"{}\", {});\n", indent_str, format_str, rust_exprs)
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
                result.push_str(&generate_transform_code(t, indent + 1, metadata));
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

// Wrapper for backwards compatibility
fn translate_js_to_rust(js_expr: &str) -> String {
    translate_js_to_rust_with_metadata(js_expr, None)
}

fn translate_js_to_rust_with_metadata(js_expr: &str, metadata: Option<&PluginMetadata>) -> String {
    // Check metadata mappings FIRST
    // Try exact match first
    if let Some(meta) = metadata {
        if let Some(mapped) = meta.translate_babel_pattern(js_expr) {
            return mapped.to_string();
        }

        // If no exact match and expression contains '.', try mapping progressively shorter prefixes
        // For "node.value.toString()", try "node.value.toString", then "node.value", then "node"
        if js_expr.contains('.') {
            let parts: Vec<&str> = js_expr.split('.').collect();
            // Try from longest to shortest prefix
            for i in (1..=parts.len()).rev() {
                let prefix = parts[..i].join(".");
                if let Some(mapped_prefix) = meta.translate_babel_pattern(&prefix) {
                    let suffix = &js_expr[prefix.len()..];
                    eprintln!("  translate_js_to_rust: Mapped {} -> {}", prefix, mapped_prefix);
                    return format!("{}{}", mapped_prefix, suffix);
                }
            }
        }
    }

    match js_expr {
        // Handle visitor context arguments
        "path" => "n".to_string(),  // Babel path -> SWC node (n)
        "state" => "self".to_string(),  // Babel state -> self

        // Handle string literals FIRST before function call patterns
        e if e.starts_with('"') && e.ends_with('"') => e.to_string(),
        e if e.starts_with('\'') && e.ends_with('\'') => e.to_string(),
        // Handle struct literals - don't re-translate already formatted Rust code
        // Pattern: Component { field: value, ... } - starts with uppercase letter and has " {"
        e if e.contains(" {") && e.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) => e.to_string(),
        // Handle vec![] macro - already valid Rust
        e if e.trim() == "vec![]" => e.to_string(),
        // Handle map/filter chains: array.map(fn).filter(Boolean) -> array.iter().filter_map(fn).collect()
        e if e.contains(".map(expr)") && e.contains(".filter(Boolean)") => {
            // Extract the array name before .map
            if let Some(map_pos) = e.find(".map(") {
                let array_name = &e[..map_pos];
                // This pattern is used to map and filter out null/undefined values
                // In Rust, we'll use filter_map which combines both operations
                format!("{}.iter().filter_map(|item| /* map logic here */).collect::<Vec<_>>()", array_name)
            } else {
                "vec![]".to_string()
            }
        }
        // Handle .filter(Boolean) by itself - filters out falsy values (removing nulls/undefined)
        e if e.ends_with(".filter(Boolean)") => {
            let prefix = &e[..e.len() - ".filter(Boolean)".len()];
            format!("{}.into_iter().flatten().collect::<Vec<_>>()", prefix)
        }
        // Handle regex replace chains: str.replace(/\\/g, '\\\\').replace(/"/g, '\\"')...
        e if e.contains(".replace(REGEX(") || e.contains(".replace(&REGEX(") || e.contains(".replace(&r_e_g_e_x(") => {
            // For the escapeCSharpString function specifically, just use simple string replaces
            // since these are literal character replacements, not regex patterns
            let mut result = e.to_string();

            // Translate JavaScript regex replace to Rust string replace
            // JavaScript: str.replace(/\\/g, '\\\\') -> Rust: str.replace('\\', "\\\\")
            result = result.replace(".replace(REGEX(\\\\), \"\\\\\\\\\")", ".replace('\\\\', \"\\\\\\\\\")");
            result = result.replace(".replace(REGEX(\"), \"\\\\\\\"\")", ".replace('\"', \"\\\\\\\"\")");
            result = result.replace(".replace(REGEX(\\n), \"\\\\n\")", ".replace('\\n', \"\\\\n\")");
            result = result.replace(".replace(REGEX(\\r), \"\\\\r\")", ".replace('\\r', \"\\\\r\")");
            result = result.replace(".replace(REGEX(\\t), \"\\\\t\")", ".replace('\\t', \"\\\\t\")");

            result
        }
        e if e.contains("path.node.id.name") => "node.ident.sym.to_string()".to_string(),
        e if e.contains("path.node.id") => "path.node.id".to_string(),
        e if e.contains("path.node") => e.replace("path.node", "node"),
        e if e.contains("path.parent.type") => "parent_type".to_string(),
        e if e.contains("path.parent.id.name") => "path.parent.id.name".to_string(),
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
        // Handle bracket notation for map/object access: typeMap.[key] -> typeMap.get(key) or [key]
        e if e.contains(".[") => {
            // Pattern: typeMap.[typeName] -> typeMap[&typeName]
            // But for numeric indices like .[0], just remove the dot: .[0] -> [0]
            let result = e.replace(".[", "[");
            // Check if it's a numeric index by seeing if the char after [ is a digit
            if let Some(bracket_pos) = result.find('[') {
                if let Some(next_char) = result.chars().nth(bracket_pos + 1) {
                    if next_char.is_ascii_digit() {
                        // Numeric index - just use [0] without &
                        result
                    } else {
                        // Variable index - add &
                        result.replace("[", "[&")
                    }
                } else {
                    result
                }
            } else {
                result
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
        // Default: convert camelCase/PascalCase identifiers to snake_case
        _ => {
            // Check if it looks like an identifier (alphanumeric, no operators/special chars)
            if js_expr.chars().all(|c| c.is_alphanumeric() || c == '_') {
                to_snake_case(js_expr)
            } else {
                js_expr.to_string()
            }
        }
    }
}

fn translate_condition(js_condition: &str, metadata: Option<&PluginMetadata>) -> String {
    // Check metadata mappings FIRST for exact matches
    if let Some(meta) = metadata {
        if let Some(mapped) = meta.translate_babel_pattern(js_condition) {
            eprintln!("  Condition mapped: {} -> {}", js_condition, mapped);
            return mapped.to_string();
        }
    }

    // Translate JavaScript condition patterns to Rust
    if js_condition.starts_with("t.is") {
        // Extract the type check and variable
        // Pattern: t.isIdentifier(node) -> matches!(node, Expr::Ident(_))
        // Pattern: t.isIdentifier(node, { name: 'useState' }) -> matches!(node, Expr::Ident(ident) if ident.sym == "useState")
        if let Some(paren_pos) = js_condition.find('(') {
            let check_type = &js_condition[2..paren_pos]; // e.g., "isIdentifier"
            let args_end = js_condition.rfind(')').unwrap_or(js_condition.len());
            let args_str = &js_condition[paren_pos+1..args_end];

            // Split arguments - check if there's a second argument with constraints
            let args: Vec<&str> = args_str.split(',').map(|s| s.trim()).collect();
            let original_var_name = args[0];

            // Extract just the variable name (before any dots)
            let base_var_name = original_var_name.split('.').next().unwrap_or(original_var_name);
            let suffix = if original_var_name.contains('.') {
                &original_var_name[base_var_name.len()..]
            } else {
                ""
            };

            // Apply metadata mapping to the base variable name only
            let mapped_base = if let Some(meta) = metadata {
                if let Some(mapped) = meta.translate_babel_pattern(base_var_name) {
                    eprintln!("  Mapped {} -> {}", base_var_name, mapped);
                    mapped
                } else {
                    eprintln!("  No mapping for {}", base_var_name);
                    base_var_name
                }
            } else {
                eprintln!("  No metadata available for {}", base_var_name);
                base_var_name
            };

            // Reconstruct with suffix
            let var_name = format!("{}{}", mapped_base, suffix);

            // Check for additional constraints like { name: 'useState' }
            let has_name_constraint = args.len() > 1 && args[1].contains("name:");
            let name_value = if has_name_constraint {
                // Extract the name value from { name: 'useState' } or { name: "useState" }
                if let Some(name_pos) = args[1].find("name:") {
                    let after_colon = &args[1][name_pos+5..].trim();
                    // Extract the string value
                    if let Some(quote_start) = after_colon.find(|c| c == '\'' || c == '"') {
                        let quote_char = after_colon.chars().nth(quote_start).unwrap();
                        let value_start = quote_start + 1;
                        if let Some(quote_end) = after_colon[value_start..].find(quote_char) {
                            Some(&after_colon[value_start..value_start+quote_end])
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            // Map Babel type checks to SWC pattern matches
            let rust_pattern = match check_type {
                "isJSXAttribute" => format!("matches!({}, JSXAttrOrSpread::JSXAttr(_))", var_name),
                "isJSXSpreadAttribute" => format!("matches!({}, JSXAttrOrSpread::SpreadElement(_))", var_name),
                "isJSXExpressionContainer" => format!("matches!({}, JSXAttrValue::JSXExprContainer(_))", var_name),
                "isStringLiteral" => format!("matches!({}, Lit::Str(_))", var_name),
                "isIdentifier" => {
                    if let Some(name) = name_value {
                        format!("matches!({}, Expr::Ident(ident) if ident.sym == \"{}\")", var_name, name)
                    } else {
                        format!("matches!({}, Expr::Ident(_))", var_name)
                    }
                },
                "isMemberExpression" => format!("matches!({}, Expr::Member(_))", var_name),
                "isCallExpression" => format!("matches!({}, Expr::Call(_))", var_name),
                "isTSArrayType" => format!("matches!({}, TsType::TsArrayType(_))", var_name),
                "isTSStringKeyword" => format!("matches!({}, TsType::TsKeywordType(TsKeywordType {{ kind: TsKeywordTypeKind::TsStringKeyword, .. }}))", var_name),
                "isTSNumberKeyword" => format!("matches!({}, TsType::TsKeywordType(TsKeywordType {{ kind: TsKeywordTypeKind::TsNumberKeyword, .. }}))", var_name),
                "isTSBooleanKeyword" => format!("matches!({}, TsType::TsKeywordType(TsKeywordType {{ kind: TsKeywordTypeKind::TsBooleanKeyword, .. }}))", var_name),
                "isTSAnyKeyword" => format!("matches!({}, TsType::TsKeywordType(TsKeywordType {{ kind: TsKeywordTypeKind::TsAnyKeyword, .. }}))", var_name),
                "isTSTypeLiteral" => format!("matches!({}, TsType::TsTypeLit(_))", var_name),
                "isTSTypeReference" => format!("matches!({}, TsType::TsTypeRef(_))", var_name),
                "isNumericLiteral" => format!("matches!({}, Lit::Num(_))", var_name),
                "isStringLiteral" => format!("matches!({}, Lit::Str(_))", var_name),
                "isBooleanLiteral" => format!("matches!({}, Lit::Bool(_))", var_name),
                "isNullLiteral" => format!("matches!({}, Lit::Null(_))", var_name),
                "isArrayExpression" => format!("matches!({}, Expr::Array(_))", var_name),
                "isObjectExpression" => format!("matches!({}, Expr::Object(_))", var_name),
                _ => format!("/* TODO: translate {} */ true", check_type),
            };

            return rust_pattern;
        }
        return "true".to_string();
    }

    let mut result = js_condition.to_string();

    // Handle simple truthiness checks on path.node.id or path.parent properties
    // Pattern: if (path.node.id) -> if (path.node.id.is_some())
    // Pattern: if (path.parent.type == "VariableDeclarator") -> proper translation
    if result == "path.node.id" {
        return "path.node.id.is_some()".to_string();
    }
    if result == "path.parent.id" {
        return "path.parent.id.is_some()".to_string();
    }

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

    // Handle bracket notation: obj.[key] -> obj.get(&key)
    // Count how many .[ we have BEFORE replacing
    let bracket_count = result.matches(".[").count();
    result = result.replace(".[", ".get(&");
    // Replace the same number of closing brackets ] with )
    for _ in 0..bracket_count {
        if let Some(pos) = result.find(']') {
            result.replace_range(pos..pos+1, ")");
        }
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

    // Convert camelCase identifiers to snake_case using regex
    // Matches word boundaries followed by camelCase identifiers
    let re = regex::Regex::new(r"\b([a-z][a-zA-Z0-9]*)\b").unwrap();
    result = re.replace_all(&result, |caps: &regex::Captures| {
        let ident = &caps[1];
        // Check if it contains uppercase (is camelCase)
        if ident.chars().any(|c| c.is_uppercase()) {
            to_snake_case(ident)
        } else {
            ident.to_string()
        }
    }).to_string();

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
    escape_rust_keyword(&result)
}

/// Escape Rust keywords by appending underscore suffix
fn escape_rust_keyword(s: &str) -> String {
    match s {
        "type" | "ref" | "match" | "const" | "static" | "mut" | "impl" | "trait" |
        "fn" | "let" | "if" | "else" | "while" | "for" | "loop" | "return" | "break" |
        "continue" | "as" | "use" | "mod" | "pub" | "crate" | "self" | "super" | "in" => {
            format!("{}_", s)
        }
        _ => s.to_string()
    }
}

/// Convert identifier to Rust, checking metadata mappings first
fn convert_identifier(name: &str, metadata: Option<&PluginMetadata>) -> String {
    // Check if metadata has an explicit mapping
    if let Some(meta) = metadata {
        if let Some(mapped) = meta.translate_babel_pattern(name) {
            return mapped.to_string();
        }
    }
    // Fall back to snake_case conversion
    to_snake_case(name)
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

        if self.inside_component {
            self.add_jsx_element(JsxElement {
                tag_name: element_type,
                is_self_closing: node.closing.is_none(),
            });
        }

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
                                    id.id.sym.to_string()
                                } else {
                                    "setState".to_string()
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
