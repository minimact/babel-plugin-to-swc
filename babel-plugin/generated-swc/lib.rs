use swc_ecma_ast::*;
use swc_ecma_visit::{VisitMut, VisitMutWith};
use swc_common::DUMMY_SP;
use std::collections::{HashMap, HashSet};
use serde::{Serialize, Deserialize};
use serde_json;

mod component;
mod extractors;
mod generators;
mod utils;
mod helpers; // Generated from template JSON files

use helpers::*; // Import all helper function stubs

use component::*; // Import Component and all related types

/// Parent context for tracking the AST path (like Babel's path.parent)
#[derive(Clone, Debug)]
pub enum ParentContext {
    Program,
    VarDeclarator(String),          // Variable name
    ExportNamed(Option<String>),    // Export name
    CallExpression,
    MemberExpression,
    BlockStatement,
    ReturnStatement,
    FunctionDeclaration(String),    // Function name
    ArrowFunction,
}

/// Main Minimact transformer
pub struct MinimactTransformer {
    /// Stack of parent contexts for path tracking
    parent_stack: Vec<ParentContext>,

    /// All processed components
    components: Vec<Component>,

    /// Top-level helper functions
    top_level_functions: Vec<TopLevelFunction>,

    /// External imports (non-Minimact, non-relative)
    external_imports: HashSet<String>,

    /// Input file path for output generation
    input_file_path: String,
}

/// Visitor for extracting hooks from component body
/// Passed to visit_mut_with for selective traversal
struct HookExtractor<'a> {
    component: &'a mut Component,
    parent_stack: &'a mut Vec<ParentContext>,
}

/// Visitor for extracting local variables
struct LocalVariableExtractor<'a> {
    component: &'a mut Component,
    parent_stack: &'a mut Vec<ParentContext>,
}

/// Visitor for extracting helper functions
struct HelperFunctionExtractor<'a> {
    component: &'a mut Component,
}

/// Visitor for capturing render body (return statement)
struct RenderBodyExtractor<'a> {
    component: &'a mut Component,
    /// Depth tracking to only capture top-level return
    depth: usize,
}

/// Visitor for extracting JSX templates
struct TemplateExtractor<'a> {
    component: &'a mut Component,
    path_generator: &'a mut HexPathGenerator,
}

/// Visitor for tracking external imports
struct ImportExtractor<'a> {
    external_imports: &'a mut HashSet<String>,
}

/// Visitor for JSX template extraction
struct JSXTemplateExtractor<'a> {
    component: &'a mut Component,
    current_path: Vec<usize>,
}

/// Visitor for loop (.map) pattern extraction
struct LoopExtractor<'a> {
    component: &'a mut Component,
}

/// Visitor for structural template extraction (conditionals)
struct StructuralExtractor<'a> {
    component: &'a mut Component,
}

/// Visitor for expression template extraction
struct ExpressionExtractor<'a> {
    component: &'a mut Component,
}

/// Hex path generator for JSX elements
pub struct HexPathGenerator {
    counter: u32,
}

impl HexPathGenerator {
    pub fn new() -> Self {
        Self { counter: 0 }
    }

    pub fn next(&mut self) -> String {
        let path = format!("{:x}", self.counter);
        self.counter += 1;
        path
    }
}

#[derive(Clone, Debug)]
pub struct TopLevelFunction {
    pub name: String,
    pub node: FnDecl,
}

impl MinimactTransformer {
    pub fn new(input_file_path: String) -> Self {
        Self {
            parent_stack: vec![ParentContext::Program],
            components: Vec::new(),
            top_level_functions: Vec::new(),
            external_imports: HashSet::new(),
            input_file_path,
        }
    }

    /// Check if a function is a component (starts with uppercase)
    fn is_component_name(name: &str) -> bool {
        name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
    }

    /// Check if a function is a custom hook (starts with "use")
    fn is_custom_hook(name: &str) -> bool {
        name.starts_with("use") && name.len() > 3 &&
            name.chars().nth(3).map(|c| c.is_uppercase()).unwrap_or(false)
    }

    /// Process a component function
    fn process_component(&mut self, func: &mut FnDecl) {
        let name = func.ident.sym.to_string();

        // Skip non-components
        if !Self::is_component_name(&name) {
            return;
        }

        // Check for custom hooks first
        if Self::is_custom_hook(&name) {
            self.process_custom_hook(func);
            return;
        }

        // Create new component
        let mut component = Component::new(name.clone());

        // Extract props from parameters
        self.extract_props(&func.function.params, &mut component);

        // Traverse function body with specialized visitors
        if let Some(body) = &mut func.function.body {
            // 1. Extract hooks (useState, useEffect, etc.)
            let mut hook_extractor = HookExtractor {
                component: &mut component,
                parent_stack: &mut self.parent_stack,
            };
            body.visit_mut_with(&mut hook_extractor);

            // 2. Extract local variables
            let mut var_extractor = LocalVariableExtractor {
                component: &mut component,
                parent_stack: &mut self.parent_stack,
            };
            body.visit_mut_with(&mut var_extractor);

            // 3. Extract helper functions
            let mut func_extractor = HelperFunctionExtractor {
                component: &mut component,
            };
            body.visit_mut_with(&mut func_extractor);

            // 4. Capture render body
            let mut render_extractor = RenderBodyExtractor {
                component: &mut component,
                depth: 0,
            };
            body.visit_mut_with(&mut render_extractor);
        }

        // Extract templates from render body (after capturing it)
        if let Some(render_body) = &mut component.render_body.clone() {
            // 5. Assign hex paths to JSX elements
            let mut path_gen = HexPathGenerator::new();
            Self::assign_hex_paths_to_jsx(render_body, &mut path_gen);

            // 6. Extract text and attribute templates
            let mut template_extractor = JSXTemplateExtractor {
                component: &mut component,
                current_path: Vec::new(),
            };
            render_body.visit_mut_with(&mut template_extractor);

            // 7. Extract loop templates (.map patterns)
            let mut loop_extractor = LoopExtractor {
                component: &mut component,
            };
            render_body.visit_mut_with(&mut loop_extractor);

            // 8. Extract structural templates (conditionals)
            let mut structural_extractor = StructuralExtractor {
                component: &mut component,
            };
            render_body.visit_mut_with(&mut structural_extractor);

            // 9. Extract expression templates
            let mut expr_extractor = ExpressionExtractor {
                component: &mut component,
            };
            render_body.visit_mut_with(&mut expr_extractor);
        }

        // Add component to list
        self.components.push(component);
    }

    /// Assign hex paths to all JSX elements in the tree
    fn assign_hex_paths_to_jsx(expr: &mut Expr, path_gen: &mut HexPathGenerator) {
        match expr {
            Expr::JSXElement(jsx) => {
                // Generate and assign key to this element
                let hex_key = path_gen.next();

                // Add key attribute if not present
                let has_key = jsx.opening.attrs.iter().any(|attr| {
                    if let JSXAttrOrSpread::JSXAttr(a) = attr {
                        if let JSXAttrName::Ident(name) = &a.name {
                            return name.sym.to_string() == "key";
                        }
                    }
                    false
                });

                if !has_key {
                    // Add data-minimact-key attribute
                    jsx.opening.attrs.push(JSXAttrOrSpread::JSXAttr(JSXAttr {
                        span: DUMMY_SP,
                        name: JSXAttrName::Ident(IdentName::new(hex_key.clone().into(), DUMMY_SP)),
                        value: Some(JSXAttrValue::Str(Str {
                            span: DUMMY_SP,
                            value: hex_key.into(),
                            raw: None,
                        })),
                    }));
                }

                // Recurse into children
                for child in &mut jsx.children {
                    match child {
                        JSXElementChild::JSXElement(child_jsx) => {
                            Self::assign_hex_paths_to_jsx(&mut Expr::JSXElement(child_jsx.clone()), path_gen);
                        }
                        JSXElementChild::JSXExprContainer(container) => {
                            if let JSXExpr::Expr(expr) = &mut container.expr {
                                Self::assign_hex_paths_to_jsx(expr, path_gen);
                            }
                        }
                        _ => {}
                    }
                }
            }
            Expr::JSXFragment(frag) => {
                for child in &mut frag.children {
                    match child {
                        JSXElementChild::JSXElement(child_jsx) => {
                            Self::assign_hex_paths_to_jsx(&mut Expr::JSXElement(child_jsx.clone()), path_gen);
                        }
                        JSXElementChild::JSXExprContainer(container) => {
                            if let JSXExpr::Expr(expr) = &mut container.expr {
                                Self::assign_hex_paths_to_jsx(expr, path_gen);
                            }
                        }
                        _ => {}
                    }
                }
            }
            Expr::Cond(cond) => {
                Self::assign_hex_paths_to_jsx(&mut cond.cons, path_gen);
                Self::assign_hex_paths_to_jsx(&mut cond.alt, path_gen);
            }
            Expr::Paren(paren) => {
                Self::assign_hex_paths_to_jsx(&mut paren.expr, path_gen);
            }
            _ => {}
        }
    }

    /// Process an arrow function component
    fn process_arrow_component(&mut self, arrow: &mut ArrowExpr, name: String) {
        // Skip non-components
        if !Self::is_component_name(&name) {
            return;
        }

        // Create new component
        let mut component = Component::new(name.clone());

        // Extract props from parameters
        self.extract_arrow_props(&arrow.params, &mut component);

        // Traverse body with specialized visitors
        match &mut *arrow.body {
            BlockStmtOrExpr::BlockStmt(block) => {
                // Same extraction pattern as function components
                let mut hook_extractor = HookExtractor {
                    component: &mut component,
                    parent_stack: &mut self.parent_stack,
                };
                block.visit_mut_with(&mut hook_extractor);

                let mut var_extractor = LocalVariableExtractor {
                    component: &mut component,
                    parent_stack: &mut self.parent_stack,
                };
                block.visit_mut_with(&mut var_extractor);

                let mut func_extractor = HelperFunctionExtractor {
                    component: &mut component,
                };
                block.visit_mut_with(&mut func_extractor);

                let mut render_extractor = RenderBodyExtractor {
                    component: &mut component,
                    depth: 0,
                };
                block.visit_mut_with(&mut render_extractor);
            }
            BlockStmtOrExpr::Expr(expr) => {
                // Implicit return - the expression IS the render body
                component.render_body = Some(expr.clone());
            }
        }

        // Add component to list
        self.components.push(component);
    }

    /// Extract props from function parameters with TypeScript type support
    fn extract_props(&mut self, params: &[Param], component: &mut Component) {
        if params.is_empty() {
            return;
        }

        // Get type annotation from parameter if present
        let type_annotation = params[0].pat.as_ident()
            .and_then(|id| id.type_ann.as_ref())
            .or_else(|| {
                if let Pat::Object(obj) = &params[0].pat {
                    obj.type_ann.as_ref()
                } else {
                    None
                }
            });

        // Build a map of prop names to types from TSTypeLiteral
        let mut prop_types: HashMap<String, String> = HashMap::new();
        if let Some(ann) = type_annotation {
            if let TsType::TsTypeLit(type_lit) = &*ann.type_ann {
                for member in &type_lit.members {
                    if let TsTypeElement::TsPropertySignature(prop_sig) = member {
                        if let Expr::Ident(ident) = &*prop_sig.key {
                            let prop_name = ident.sym.to_string();
                            let prop_type = prop_sig.type_ann.as_ref()
                                .map(|ann| Self::ts_type_to_csharp(&ann.type_ann))
                                .unwrap_or_else(|| "dynamic".to_string());
                            prop_types.insert(prop_name, prop_type);
                        }
                    }
                }
            }
        }

        match &params[0].pat {
            // Destructured props: function Component({ prop1, prop2 })
            Pat::Object(obj_pat) => {
                for prop in &obj_pat.props {
                    match prop {
                        ObjectPatProp::KeyValue(kv) => {
                            if let PropName::Ident(ident) = &kv.key {
                                let name = ident.sym.to_string();
                                let prop_type = prop_types.get(&name)
                                    .cloned()
                                    .unwrap_or_else(|| "dynamic".to_string());
                                component.props.push(crate::component::Prop {
                                    name,
                                    prop_type,
                                });
                            }
                        }
                        ObjectPatProp::Assign(assign) => {
                            let name = assign.key.sym.to_string();
                            let prop_type = prop_types.get(&name)
                                .cloned()
                                .unwrap_or_else(|| "dynamic".to_string());
                            component.props.push(crate::component::Prop {
                                name,
                                prop_type,
                            });
                        }
                        ObjectPatProp::Rest(_) => {}
                    }
                }
            }
            // Props as single object: function Component(props)
            Pat::Ident(ident) => {
                component.props.push(crate::component::Prop {
                    name: ident.id.sym.to_string(),
                    prop_type: "dynamic".to_string(),
                });
            }
            _ => {}
        }
    }

    /// Convert TypeScript type to C# type
    fn ts_type_to_csharp(ts_type: &TsType) -> String {
        match ts_type {
            TsType::TsKeywordType(kw) => {
                match kw.kind {
                    TsKeywordTypeKind::TsStringKeyword => "string".to_string(),
                    TsKeywordTypeKind::TsNumberKeyword => "int".to_string(),
                    TsKeywordTypeKind::TsBooleanKeyword => "bool".to_string(),
                    TsKeywordTypeKind::TsVoidKeyword => "void".to_string(),
                    TsKeywordTypeKind::TsNullKeyword => "object".to_string(),
                    TsKeywordTypeKind::TsUndefinedKeyword => "object".to_string(),
                    TsKeywordTypeKind::TsAnyKeyword => "dynamic".to_string(),
                    _ => "dynamic".to_string(),
                }
            }
            TsType::TsArrayType(arr) => {
                let elem_type = Self::ts_type_to_csharp(&arr.elem_type);
                format!("List<{}>", elem_type)
            }
            TsType::TsTypeRef(type_ref) => {
                if let TsEntityName::Ident(ident) = &type_ref.type_name {
                    let name = ident.sym.to_string();
                    match name.as_str() {
                        "Array" => {
                            if let Some(params) = &type_ref.type_params {
                                if let Some(param) = params.params.get(0) {
                                    let elem_type = Self::ts_type_to_csharp(param);
                                    return format!("List<{}>", elem_type);
                                }
                            }
                            "List<dynamic>".to_string()
                        }
                        "Record" | "object" => "Dictionary<string, dynamic>".to_string(),
                        "Function" => "Action".to_string(),
                        _ => name, // Use type name as-is for custom types
                    }
                } else {
                    "dynamic".to_string()
                }
            }
            TsType::TsFnOrConstructorType(_) => "Action".to_string(),
            TsType::TsUnionOrIntersectionType(_) => "dynamic".to_string(),
            TsType::TsLitType(lit) => {
                match &lit.lit {
                    TsLit::Str(_) => "string".to_string(),
                    TsLit::Number(_) => "int".to_string(),
                    TsLit::Bool(_) => "bool".to_string(),
                    _ => "dynamic".to_string(),
                }
            }
            _ => "dynamic".to_string(),
        }
    }

    /// Extract props from arrow function parameters
    fn extract_arrow_props(&mut self, params: &[Pat], component: &mut Component) {
        if params.is_empty() {
            return;
        }

        match &params[0] {
            Pat::Object(obj_pat) => {
                for prop in &obj_pat.props {
                    match prop {
                        ObjectPatProp::KeyValue(kv) => {
                            if let PropName::Ident(ident) = &kv.key {
                                component.props.push(crate::component::Prop {
                                    name: ident.sym.to_string(),
                                    prop_type: "dynamic".to_string(),
                                });
                            }
                        }
                        ObjectPatProp::Assign(assign) => {
                            component.props.push(crate::component::Prop {
                                name: assign.key.sym.to_string(),
                                prop_type: "dynamic".to_string(),
                            });
                        }
                        ObjectPatProp::Rest(_) => {}
                    }
                }
            }
            Pat::Ident(ident) => {
                component.props.push(crate::component::Prop {
                    name: ident.id.sym.to_string(),
                    prop_type: "dynamic".to_string(),
                });
            }
            _ => {}
        }
    }

    /// Process a custom hook
    fn process_custom_hook(&mut self, _func: &mut FnDecl) {
        // TODO: Implement custom hook processing
    }
}

impl VisitMut for MinimactTransformer {
    /// Program entry/exit - manual iteration for selective traversal
    fn visit_mut_program(&mut self, program: &mut Program) {
        match program {
            Program::Module(module) => {
                // First pass: collect top-level functions and imports
                for item in &mut module.body {
                    match item {
                        // Track imports
                        ModuleItem::ModuleDecl(ModuleDecl::Import(import)) => {
                            self.process_import(import);
                        }
                        // Collect helper functions (lowercase)
                        ModuleItem::Stmt(Stmt::Decl(Decl::Fn(fn_decl))) => {
                            let name = fn_decl.ident.sym.to_string();
                            if !Self::is_component_name(&name) {
                                self.top_level_functions.push(TopLevelFunction {
                                    name,
                                    node: fn_decl.clone(),
                                });
                            }
                        }
                        _ => {}
                    }
                }

                // Second pass: process components with manual iteration
                for item in &mut module.body {
                    match item {
                        // Function declarations (components)
                        ModuleItem::Stmt(Stmt::Decl(Decl::Fn(fn_decl))) => {
                            let name = fn_decl.ident.sym.to_string();
                            if Self::is_component_name(&name) || Self::is_custom_hook(&name) {
                                self.push_parent(ParentContext::FunctionDeclaration(name.clone()));
                                self.process_component(fn_decl);
                                self.pop_parent();
                            }
                        }
                        // Export named declarations
                        ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export)) => {
                            if let Decl::Fn(fn_decl) = &mut export.decl {
                                let name = fn_decl.ident.sym.to_string();
                                self.push_parent(ParentContext::ExportNamed(Some(name.clone())));
                                if Self::is_component_name(&name) || Self::is_custom_hook(&name) {
                                    self.process_component(fn_decl);
                                }
                                self.pop_parent();
                            } else if let Decl::Var(var_decl) = &mut export.decl {
                                // Check for arrow function components
                                self.process_var_decl(var_decl);
                            }
                        }
                        // Variable declarations (arrow function components)
                        ModuleItem::Stmt(Stmt::Decl(Decl::Var(var_decl))) => {
                            self.process_var_decl(var_decl);
                        }
                        _ => {}
                    }
                }
            }
            Program::Script(script) => {
                // Similar handling for scripts
                for stmt in &mut script.body {
                    match stmt {
                        Stmt::Decl(Decl::Fn(fn_decl)) => {
                            let name = fn_decl.ident.sym.to_string();
                            if Self::is_component_name(&name) || Self::is_custom_hook(&name) {
                                self.push_parent(ParentContext::FunctionDeclaration(name.clone()));
                                self.process_component(fn_decl);
                                self.pop_parent();
                            }
                        }
                        Stmt::Decl(Decl::Var(var_decl)) => {
                            self.process_var_decl(var_decl);
                        }
                        _ => {}
                    }
                }
            }
        }

        // After processing: generate outputs (C# code, template JSON, etc.)
        self.generate_outputs();
    }
}

impl MinimactTransformer {
    /// Process variable declaration (for arrow function components at top level)
    fn process_var_decl(&mut self, var_decl: &mut VarDecl) {
        for decl in &mut var_decl.decls {
            if let Pat::Ident(ident) = &decl.name {
                let name = ident.id.sym.to_string();

                // Check if init is an arrow function
                if let Some(init) = &mut decl.init {
                    if let Expr::Arrow(arrow) = &mut **init {
                        if Self::is_component_name(&name) {
                            self.parent_stack.push(ParentContext::VarDeclarator(name.clone()));
                            self.process_arrow_component(arrow, name);
                            self.parent_stack.pop();
                        }
                    }
                }
            }
        }
    }

    /// Generate outputs (C# code, templates, etc.)
    fn generate_outputs(&self) {
        use std::fs;
        use std::path::Path;

        if self.input_file_path.is_empty() || self.components.is_empty() {
            return;
        }

        let input_path = Path::new(&self.input_file_path);
        let output_dir = input_path.parent().unwrap_or(Path::new("."));

        for component in &self.components {
            // 1. Generate C# file
            let cs_code = self.generate_csharp_code(component);
            let cs_file_path = output_dir.join(format!("{}.cs", component.name));

            if let Err(e) = fs::write(&cs_file_path, &cs_code) {
                eprintln!("[Minimact C#] Failed to write {:?}: {}", cs_file_path, e);
            } else {
                println!("[Minimact C#] Generated {:?}", cs_file_path);
            }

            // 2. Generate .templates.json file
            if !component.templates.is_empty() {
                let templates_json = self.generate_templates_json(component);
                let templates_file_path = output_dir.join(format!("{}.templates.json", component.name));

                if let Err(e) = fs::write(&templates_file_path, &templates_json) {
                    eprintln!("[Minimact Templates] Failed to write {:?}: {}", templates_file_path, e);
                } else {
                    println!("[Minimact Templates] Generated {:?}", templates_file_path);
                }
            }

            // 3. Generate .timeline-templates.json if timeline exists
            // TODO: Implement when timeline analysis is complete

            // 4. Generate .structural-changes.json for hot reload
            // This would require comparing with previous state
            // TODO: Implement structural change detection
        }

        // Note: .tsx.keys file generation requires access to original source
        // which is handled differently in SWC vs Babel
    }

    /// Generate C# code for a component
    fn generate_csharp_code(&self, component: &Component) -> String {
        let mut code = String::new();

        // Using statements
        code.push_str("using System;\n");
        code.push_str("using System.Collections.Generic;\n");
        code.push_str("using Minimact;\n\n");

        // Namespace and class
        code.push_str(&format!("public class {} : MinimactComponent\n{{\n", component.name));

        // Properties from props
        for prop in &component.props {
            code.push_str(&format!("    public {} {} {{ get; set; }}\n", prop.prop_type, prop.name));
        }

        // State properties
        for state in &component.use_state {
            code.push_str(&format!("    private {} _{};\n", state.state_type, state.var_name));
            code.push_str(&format!("    public {} {} {{ get => _{}; set {{ _{} = value; StateHasChanged(); }} }}\n\n",
                state.state_type, state.var_name, state.var_name, state.var_name));
        }

        // Refs
        for ref_info in &component.use_ref {
            code.push_str(&format!("    public dynamic {} {{ get; set; }} = {};\n",
                ref_info.name, ref_info.initial_value));
        }

        // Constructor
        code.push_str(&format!("\n    public {}()\n    {{\n", component.name));
        for state in &component.use_state {
            code.push_str(&format!("        _{} = {};\n", state.var_name, state.initial_value));
        }
        code.push_str("    }\n");

        // Helper functions
        for func in &component.helper_functions {
            let params = func.params.iter()
                .map(|p| format!("{} {}", p.param_type, p.name))
                .collect::<Vec<_>>()
                .join(", ");

            let async_modifier = if func.is_async { "async " } else { "" };
            code.push_str(&format!("\n    public {}{}{}({})\n    {{\n        // TODO: Implement\n    }}\n",
                async_modifier, func.return_type, func.name, params));
        }

        // Render method
        code.push_str("\n    public override void Render()\n    {\n");
        code.push_str("        // JSX rendering handled by runtime\n");
        code.push_str("    }\n");

        code.push_str("}\n");

        code
    }

    /// Generate templates JSON for a component
    fn generate_templates_json(&self, component: &Component) -> String {
        use serde_json::{json, to_string_pretty};

        let mut template_map = serde_json::Map::new();

        // Text and attribute templates
        for (key, template) in &component.templates {
            template_map.insert(key.clone(), json!({
                "path": template.path,
                "template": template.template,
                "bindings": template.bindings
            }));
        }

        // Loop templates
        let loop_templates: Vec<_> = component.loop_templates.iter().map(|lt| {
            json!({
                "stateKey": lt.state_key,
                "itemVar": lt.item_var,
                "indexVar": lt.index_var,
                "keyExpression": lt.key_expression
            })
        }).collect();

        // Structural templates
        let structural_templates: Vec<_> = component.structural_templates.iter().map(|st| {
            json!({
                "type": st.template_type,
                "conditionBinding": st.condition_binding
            })
        }).collect();

        // Conditional element templates
        let mut conditional_map = serde_json::Map::new();
        for (key, cet) in &component.conditional_element_templates {
            conditional_map.insert(key.clone(), json!({
                "path": cet.path,
                "conditionExpression": cet.condition_expression,
                "evaluable": cet.evaluable
            }));
        }

        // Expression templates
        let expression_templates: Vec<_> = component.expression_templates.iter().map(|et| {
            json!({
                "type": et.template_type,
                "stateKey": et.state_key,
                "binding": et.binding,
                "method": et.method,
                "args": et.args
            })
        }).collect();

        let result = json!({
            "componentName": component.name,
            "templates": template_map,
            "loopTemplates": loop_templates,
            "structuralTemplates": structural_templates,
            "conditionalElementTemplates": conditional_map,
            "expressionTemplates": expression_templates
        });

        to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string())
    }

    /// Get parent context
    fn get_parent(&self) -> Option<&ParentContext> {
        self.parent_stack.last()
    }

    /// Push parent context
    fn push_parent(&mut self, ctx: ParentContext) {
        self.parent_stack.push(ctx);
    }

    /// Pop parent context
    fn pop_parent(&mut self) {
        self.parent_stack.pop();
    }

    /// Process import declaration
    fn process_import(&mut self, import: &ImportDecl) {
        let source = String::from_utf8_lossy(import.src.value.as_bytes()).to_string();

        // Skip internal imports
        if source.starts_with("minimact") ||
           source.starts_with('.') ||
           source.starts_with('/') ||
           source.ends_with(".css") {
            return;
        }

        // Track external identifiers
        for spec in &import.specifiers {
            match spec {
                ImportSpecifier::Default(default) => {
                    self.external_imports.insert(default.local.sym.to_string());
                }
                ImportSpecifier::Named(named) => {
                    self.external_imports.insert(named.local.sym.to_string());
                }
                ImportSpecifier::Namespace(ns) => {
                    self.external_imports.insert(ns.local.sym.to_string());
                }
            }
        }
    }
}

// =============================================================================
// Specialized Visitor Implementations
// =============================================================================

/// HookExtractor - visits VarDeclarator to find hook calls
impl VisitMut for HookExtractor<'_> {
    fn visit_mut_var_declarator(&mut self, var: &mut VarDeclarator) {
        // Check if init is a hook call
        if let Some(init) = &mut var.init {
            if let Expr::Call(call) = &mut **init {
                if let Callee::Expr(callee) = &call.callee {
                    if let Expr::Ident(ident) = &**callee {
                        let callee_name = ident.sym.to_string();

                        // Push parent context
                        match &var.name {
                            Pat::Ident(id) => {
                                self.parent_stack.push(ParentContext::VarDeclarator(id.id.sym.to_string()));
                            }
                            Pat::Array(_) => {
                                // Array pattern - handled in extraction
                            }
                            _ => {}
                        }

                        // Route to appropriate hook extractor
                        match callee_name.as_str() {
                            // State hooks
                            "useState" => {
                                extract_use_state(call, &var.name, self.component);
                            }
                            "useClientState" => {
                                extract_use_client_state(call, &var.name, self.component);
                            }
                            "useProtectedState" => {
                                extract_use_protected_state(call, &var.name, self.component);
                            }
                            "useStateX" => {
                                extract_use_state_x(call, &var.name, self.component);
                            }

                            // Effect and ref hooks
                            "useEffect" => {
                                extract_use_effect(call, self.component);
                            }
                            "useRef" => {
                                extract_use_ref(call, &var.name, self.component);
                            }

                            // Content hooks
                            "useMarkdown" => {
                                extract_use_markdown(call, &var.name, self.component);
                            }
                            "useRazorMarkdown" => {
                                extract_use_razor_markdown(call, &var.name, self.component);
                            }
                            "useTemplate" => {
                                extract_use_template(call, self.component);
                            }

                            // UI state hooks
                            "useValidation" => {
                                extract_use_validation(call, &var.name, self.component);
                            }
                            "useModal" => {
                                extract_use_modal(call, &var.name, self.component);
                            }
                            "useToggle" => {
                                extract_use_toggle(call, &var.name, self.component);
                            }
                            "useDropdown" => {
                                extract_use_dropdown(call, &var.name, self.component);
                            }

                            // Pub/Sub hooks
                            "usePub" => {
                                extract_use_pub(call, &var.name, self.component);
                            }
                            "useSub" => {
                                extract_use_sub(call, &var.name, self.component);
                            }

                            // Task scheduling hooks
                            "useMicroTask" => {
                                extract_use_micro_task(call, self.component);
                            }
                            "useMacroTask" => {
                                extract_use_macro_task(call, self.component);
                            }

                            // Server communication hooks
                            "useSignalR" => {
                                extract_use_signalr(call, &var.name, self.component);
                            }
                            "useServerTask" => {
                                extract_use_server_task(call, &var.name, self.component);
                            }
                            "usePaginatedServerTask" => {
                                extract_use_paginated_server_task(call, &var.name, self.component);
                            }

                            // MVC integration hooks
                            "useMvcState" => {
                                extract_use_mvc_state(call, &var.name, self.component);
                            }
                            "useMvcViewModel" => {
                                extract_use_mvc_view_model(call, &var.name, self.component);
                            }

                            // Optimization hooks
                            "usePredictHint" => {
                                extract_use_predict_hint(call, self.component);
                            }

                            _ => {
                                // Check for custom hooks (useXxx)
                                if callee_name.starts_with("use") && callee_name.len() > 3 {
                                    if let Some(c) = callee_name.chars().nth(3) {
                                        if c.is_uppercase() {
                                            extract_custom_hook_call(call, &var.name, &callee_name, self.component);
                                        }
                                    }
                                }
                            }
                        }

                        // Pop parent context
                        if matches!(&var.name, Pat::Ident(_)) {
                            self.parent_stack.pop();
                        }
                    }
                }
            }
        }

        // Don't recurse - we only care about top-level var declarators
    }

    // Also check expression statements for useEffect without assignment
    fn visit_mut_expr_stmt(&mut self, stmt: &mut ExprStmt) {
        if let Expr::Call(call) = &mut *stmt.expr {
            if let Callee::Expr(callee) = &call.callee {
                if let Expr::Ident(ident) = &**callee {
                    if ident.sym.to_string() == "useEffect" {
                        extract_use_effect(call, self.component);
                    }
                }
            }
        }
    }
}

/// LocalVariableExtractor - extracts non-hook variable declarations
impl VisitMut for LocalVariableExtractor<'_> {
    fn visit_mut_var_decl(&mut self, var_decl: &mut VarDecl) {
        for decl in &var_decl.decls {
            // Skip if this is a hook call
            let is_hook = decl.init.as_ref().map(|init| {
                if let Expr::Call(call) = &**init {
                    if let Callee::Expr(callee) = &call.callee {
                        if let Expr::Ident(ident) = &**callee {
                            let name = ident.sym.to_string();
                            return name.starts_with("use") && name.len() > 3 &&
                                name.chars().nth(3).map(|c| c.is_uppercase()).unwrap_or(false);
                        }
                    }
                }
                false
            }).unwrap_or(false);

            if is_hook {
                continue;
            }

            // Extract as local variable
            if let Pat::Ident(ident) = &decl.name {
                let name = ident.id.sym.to_string();
                let initial_value = decl.init
                    .as_ref()
                    .map(|e| expr_to_csharp(e))
                    .unwrap_or_else(|| "null".to_string());

                self.component.local_variables.push(LocalVariable {
                    name,
                    var_type: "dynamic".to_string(),
                    initial_value,
                    is_const: var_decl.kind == VarDeclKind::Const,
                });
            }
        }

        // Don't recurse into nested scopes
    }
}

/// HelperFunctionExtractor - extracts function declarations in component body
impl VisitMut for HelperFunctionExtractor<'_> {
    fn visit_mut_fn_decl(&mut self, fn_decl: &mut FnDecl) {
        let name = fn_decl.ident.sym.to_string();

        // Skip custom hooks
        if name.starts_with("use") && name.len() > 3 {
            if let Some(c) = name.chars().nth(3) {
                if c.is_uppercase() {
                    return;
                }
            }
        }

        let params: Vec<FunctionParam> = fn_decl.function.params
            .iter()
            .map(|param| {
                let param_name = match &param.pat {
                    Pat::Ident(ident) => ident.id.sym.to_string(),
                    _ => "param".to_string(),
                };
                FunctionParam {
                    name: param_name,
                    param_type: "dynamic".to_string(),
                }
            })
            .collect();

        self.component.helper_functions.push(HelperFunction {
            name,
            params,
            return_type: "void".to_string(),
            is_async: fn_decl.function.is_async,
        });

        // Don't recurse - we don't need nested functions
    }
}

/// RenderBodyExtractor - captures the return statement's argument
impl VisitMut for RenderBodyExtractor<'_> {
    fn visit_mut_return_stmt(&mut self, ret: &mut ReturnStmt) {
        // Only capture top-level return (depth == 0)
        if self.depth == 0 {
            if let Some(arg) = &ret.arg {
                self.component.render_body = Some(arg.clone());
            }
        }
    }

    // Track function depth to only capture component's return
    fn visit_mut_function(&mut self, func: &mut Function) {
        self.depth += 1;
        func.visit_mut_children_with(self);
        self.depth -= 1;
    }

    fn visit_mut_arrow_expr(&mut self, arrow: &mut ArrowExpr) {
        self.depth += 1;
        arrow.visit_mut_children_with(self);
        self.depth -= 1;
    }
}

/// ImportExtractor - tracks external imports
impl VisitMut for ImportExtractor<'_> {
    fn visit_mut_import_decl(&mut self, import: &mut ImportDecl) {
        let source = String::from_utf8_lossy(import.src.value.as_bytes()).to_string();

        // Skip internal imports
        if source.starts_with("minimact") ||
           source.starts_with('.') ||
           source.starts_with('/') ||
           source.ends_with(".css") {
            return;
        }

        // Track identifiers
        for spec in &import.specifiers {
            match spec {
                ImportSpecifier::Default(default) => {
                    self.external_imports.insert(default.local.sym.to_string());
                }
                ImportSpecifier::Named(named) => {
                    self.external_imports.insert(named.local.sym.to_string());
                }
                ImportSpecifier::Namespace(ns) => {
                    self.external_imports.insert(ns.local.sym.to_string());
                }
            }
        }
    }
}

/// JSXTemplateExtractor - extracts text and attribute templates from JSX
impl VisitMut for JSXTemplateExtractor<'_> {
    fn visit_mut_jsx_element(&mut self, jsx: &mut JSXElement) {
        let path = self.current_path.iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(".");

        // Extract attribute templates
        for attr in &jsx.opening.attrs {
            if let JSXAttrOrSpread::JSXAttr(jsx_attr) = attr {
                if let Some(JSXAttrValue::JSXExprContainer(container)) = &jsx_attr.value {
                    if let JSXExpr::Expr(expr) = &container.expr {
                        // Extract bindings from expression
                        let bindings = extract_bindings_from_expr(expr);
                        if !bindings.is_empty() {
                            let attr_name = match &jsx_attr.name {
                                JSXAttrName::Ident(ident) => ident.sym.to_string(),
                                JSXAttrName::JSXNamespacedName(ns) => {
                                    format!("{}:{}", ns.ns.sym, ns.name.sym)
                                }
                            };

                            let template_key = format!("{}@{}", path, attr_name);
                            self.component.templates.insert(template_key, Template {
                                path: path.clone(),
                                template: generate_template_string(expr),
                                bindings,
                            });
                        }
                    }
                }
            }
        }

        // Traverse children
        for (i, child) in jsx.children.iter_mut().enumerate() {
            self.current_path.push(i);

            match child {
                JSXElementChild::JSXText(text) => {
                    // Check for text with interpolations
                    let content = String::from_utf8_lossy(text.value.as_bytes()).to_string();
                    if content.trim().is_empty() {
                        self.current_path.pop();
                        continue;
                    }
                }
                JSXElementChild::JSXExprContainer(container) => {
                    if let JSXExpr::Expr(expr) = &container.expr {
                        let bindings = extract_bindings_from_expr(expr);
                        if !bindings.is_empty() {
                            let child_path = self.current_path.iter()
                                .map(|i| i.to_string())
                                .collect::<Vec<_>>()
                                .join(".");

                            self.component.templates.insert(child_path.clone(), Template {
                                path: child_path,
                                template: generate_template_string(expr),
                                bindings,
                            });
                        }
                    }
                }
                JSXElementChild::JSXElement(child_jsx) => {
                    child_jsx.visit_mut_with(self);
                }
                _ => {}
            }

            self.current_path.pop();
        }
    }
}

/// LoopExtractor - extracts .map() patterns for loop templates
impl VisitMut for LoopExtractor<'_> {
    fn visit_mut_call_expr(&mut self, call: &mut CallExpr) {
        // Check if this is a .map() call
        if let Callee::Expr(callee) = &call.callee {
            if let Expr::Member(member) = &**callee {
                if let MemberProp::Ident(prop) = &member.prop {
                    if prop.sym.to_string() == "map" {
                        // Extract the array being mapped
                        let state_key = extract_binding_from_expr(&member.obj);

                        if let Some(state_key) = state_key {
                            // Get the callback function
                            if let Some(arg) = call.args.get(0) {
                                if let Expr::Arrow(arrow) = &*arg.expr {
                                    // Extract item and index variables
                                    let item_var = arrow.params.get(0)
                                        .and_then(|p| {
                                            if let Pat::Ident(id) = p {
                                                Some(id.id.sym.to_string())
                                            } else {
                                                None
                                            }
                                        })
                                        .unwrap_or_else(|| "item".to_string());

                                    let index_var = arrow.params.get(1)
                                        .and_then(|p| {
                                            if let Pat::Ident(id) = p {
                                                Some(id.id.sym.to_string())
                                            } else {
                                                None
                                            }
                                        });

                                    // Extract key expression from JSX
                                    let key_expression = extract_key_from_jsx(&arrow.body);

                                    self.component.loop_templates.push(LoopTemplate {
                                        state_key,
                                        item_var,
                                        index_var,
                                        key_expression: key_expression.unwrap_or_default(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        // Continue traversal
        call.visit_mut_children_with(self);
    }
}

/// StructuralExtractor - extracts conditional rendering patterns
impl VisitMut for StructuralExtractor<'_> {
    // Ternary conditional: condition ? <A /> : <B />
    fn visit_mut_cond_expr(&mut self, cond: &mut CondExpr) {
        // Check if consequent or alternate is JSX
        let cons_is_jsx = is_jsx_expr(&cond.cons);
        let alt_is_jsx = is_jsx_expr(&cond.alt);

        if cons_is_jsx || alt_is_jsx {
            let condition_binding = extract_binding_from_expr(&cond.test)
                .unwrap_or_else(|| generate_expr_string(&cond.test));

            self.component.structural_templates.push(StructuralTemplate {
                template_type: "conditional".to_string(),
                condition_binding,
            });
        }

        // Continue traversal
        cond.visit_mut_children_with(self);
    }

    // Logical AND: condition && <A />
    fn visit_mut_bin_expr(&mut self, bin: &mut BinExpr) {
        if bin.op == BinaryOp::LogicalAnd {
            // Check if right side is JSX
            if is_jsx_expr(&bin.right) {
                let condition_binding = extract_binding_from_expr(&bin.left)
                    .unwrap_or_else(|| generate_expr_string(&bin.left));

                self.component.structural_templates.push(StructuralTemplate {
                    template_type: "logical".to_string(),
                    condition_binding,
                });
            }
        }

        // Continue traversal
        bin.visit_mut_children_with(self);
    }
}

/// ExpressionExtractor - extracts expression templates (method calls, binary ops, etc.)
impl VisitMut for ExpressionExtractor<'_> {
    fn visit_mut_jsx_expr_container(&mut self, container: &mut JSXExprContainer) {
        if let JSXExpr::Expr(expr) = &container.expr {
            match &**expr {
                // Method call: price.toFixed(2)
                Expr::Call(call) => {
                    if let Callee::Expr(callee) = &call.callee {
                        if let Expr::Member(member) = &**callee {
                            if let MemberProp::Ident(method) = &member.prop {
                                let method_name = method.sym.to_string();

                                // Check if this is a supported transform
                                if is_supported_transform(&method_name) {
                                    let binding = extract_binding_from_expr(&member.obj)
                                        .unwrap_or_default();
                                    let state_key = binding.split('.').next()
                                        .unwrap_or(&binding).to_string();

                                    let args: Vec<String> = call.args.iter()
                                        .filter_map(|arg| literal_to_string(&arg.expr))
                                        .collect();

                                    self.component.expression_templates.push(ExpressionTemplate {
                                        template_type: "methodCall".to_string(),
                                        state_key,
                                        binding,
                                        method: Some(method_name),
                                        args,
                                    });
                                }
                            }
                        }
                    }
                }
                // Binary expression: count * 2 + 1
                Expr::Bin(bin) => {
                    let bindings = extract_all_bindings(expr);
                    if !bindings.is_empty() {
                        let state_key = bindings[0].split('.').next()
                            .unwrap_or(&bindings[0]).to_string();

                        self.component.expression_templates.push(ExpressionTemplate {
                            template_type: "binaryExpression".to_string(),
                            state_key,
                            binding: bindings.join(", "),
                            method: None,
                            args: Vec::new(),
                        });
                    }
                }
                // Unary expression: -count
                Expr::Unary(unary) => {
                    if let Some(binding) = extract_binding_from_expr(&unary.arg) {
                        let state_key = binding.split('.').next()
                            .unwrap_or(&binding).to_string();

                        self.component.expression_templates.push(ExpressionTemplate {
                            template_type: "unaryExpression".to_string(),
                            state_key,
                            binding,
                            method: None,
                            args: vec![unary.op.to_string()],
                        });
                    }
                }
                // Member expression: items.length
                Expr::Member(member) => {
                    if let MemberProp::Ident(prop) = &member.prop {
                        let prop_name = prop.sym.to_string();
                        if is_supported_transform(&prop_name) {
                            let binding = build_member_path(member);
                            let state_key = binding.split('.').next()
                                .unwrap_or(&binding).to_string();

                            self.component.expression_templates.push(ExpressionTemplate {
                                template_type: "memberExpression".to_string(),
                                state_key,
                                binding,
                                method: Some(prop_name),
                                args: Vec::new(),
                            });
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

// =============================================================================
// Helper Functions for Template Extraction
// =============================================================================

fn extract_bindings_from_expr(expr: &Expr) -> Vec<String> {
    let mut bindings = Vec::new();
    extract_all_bindings_inner(expr, &mut bindings);
    bindings
}

fn extract_all_bindings_inner(expr: &Expr, bindings: &mut Vec<String>) {
    match expr {
        Expr::Ident(ident) => {
            bindings.push(ident.sym.to_string());
        }
        Expr::Member(member) => {
            let path = build_member_path(member);
            bindings.push(path);
        }
        Expr::Bin(bin) => {
            extract_all_bindings_inner(&bin.left, bindings);
            extract_all_bindings_inner(&bin.right, bindings);
        }
        Expr::Unary(unary) => {
            extract_all_bindings_inner(&unary.arg, bindings);
        }
        Expr::Cond(cond) => {
            extract_all_bindings_inner(&cond.test, bindings);
            extract_all_bindings_inner(&cond.cons, bindings);
            extract_all_bindings_inner(&cond.alt, bindings);
        }
        Expr::Call(call) => {
            if let Callee::Expr(callee) = &call.callee {
                extract_all_bindings_inner(callee, bindings);
            }
        }
        _ => {}
    }
}

fn extract_all_bindings(expr: &Expr) -> Vec<String> {
    let mut bindings = Vec::new();
    extract_all_bindings_inner(expr, &mut bindings);
    bindings
}

fn extract_binding_from_expr(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Ident(ident) => Some(ident.sym.to_string()),
        Expr::Member(member) => Some(build_member_path(member)),
        _ => None,
    }
}

fn build_member_path(member: &MemberExpr) -> String {
    let mut parts = Vec::new();
    let mut current: &Expr = &member.obj;

    // Get property name
    if let MemberProp::Ident(prop) = &member.prop {
        parts.push(prop.sym.to_string());
    }

    // Walk up the member chain
    while let Expr::Member(m) = current {
        if let MemberProp::Ident(prop) = &m.prop {
            parts.push(prop.sym.to_string());
        }
        current = &m.obj;
    }

    // Get root identifier
    if let Expr::Ident(ident) = current {
        parts.push(ident.sym.to_string());
    }

    parts.reverse();
    parts.join(".")
}

fn generate_template_string(expr: &Expr) -> String {
    // Generate a template string representation
    match expr {
        Expr::Ident(ident) => format!("${{{{{}}}}}", ident.sym),
        Expr::Member(member) => format!("${{{{{}}}}}", build_member_path(member)),
        Expr::Bin(bin) => {
            let left = generate_template_string(&bin.left);
            let right = generate_template_string(&bin.right);
            format!("{} {} {}", left, bin.op, right)
        }
        _ => "?".to_string(),
    }
}

fn generate_expr_string(expr: &Expr) -> String {
    match expr {
        Expr::Ident(ident) => ident.sym.to_string(),
        Expr::Member(member) => build_member_path(member),
        Expr::Unary(unary) => {
            let arg = generate_expr_string(&unary.arg);
            format!("{}{}", unary.op, arg)
        }
        Expr::Bin(bin) => {
            let left = generate_expr_string(&bin.left);
            let right = generate_expr_string(&bin.right);
            format!("{} {} {}", left, bin.op, right)
        }
        _ => "?".to_string(),
    }
}

fn is_jsx_expr(expr: &Expr) -> bool {
    matches!(expr, Expr::JSXElement(_) | Expr::JSXFragment(_))
}

fn extract_key_from_jsx(body: &BlockStmtOrExpr) -> Option<String> {
    // Look for key attribute in JSX element
    match body {
        BlockStmtOrExpr::Expr(expr) => {
            if let Expr::JSXElement(jsx) = &**expr {
                for attr in &jsx.opening.attrs {
                    if let JSXAttrOrSpread::JSXAttr(jsx_attr) = attr {
                        if let JSXAttrName::Ident(name) = &jsx_attr.name {
                            if name.sym.to_string() == "key" {
                                if let Some(JSXAttrValue::JSXExprContainer(container)) = &jsx_attr.value {
                                    if let JSXExpr::Expr(expr) = &container.expr {
                                        return Some(generate_expr_string(expr));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }
    None
}

fn is_supported_transform(name: &str) -> bool {
    matches!(name,
        "toFixed" | "toPrecision" | "toExponential" |
        "toUpperCase" | "toLowerCase" | "trim" |
        "substring" | "substr" | "slice" |
        "length" | "join"
    )
}

fn literal_to_string(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Lit(lit) => match lit {
            Lit::Str(s) => Some(String::from_utf8_lossy(s.value.as_bytes()).to_string()),
            Lit::Num(n) => Some(n.value.to_string()),
            Lit::Bool(b) => Some(b.value.to_string()),
            _ => None,
        },
        _ => None,
    }
}

// =============================================================================
// Hook Extraction Functions
// =============================================================================

fn extract_use_state(call: &CallExpr, binding: &Pat, component: &mut Component) {
    if let Pat::Array(arr) = binding {
        let var_name = arr.elems.get(0)
            .and_then(|e| e.as_ref())
            .and_then(|p| if let Pat::Ident(id) = p { Some(id.id.sym.to_string()) } else { None })
            .unwrap_or_default();

        let setter_name = arr.elems.get(1)
            .and_then(|e| e.as_ref())
            .and_then(|p| if let Pat::Ident(id) = p { Some(id.id.sym.to_string()) } else { None })
            .unwrap_or_default();

        let initial_value = call.args.get(0)
            .map(|arg| expr_to_csharp(&arg.expr))
            .unwrap_or_else(|| "null".to_string());

        component.use_state.push(UseStateInfo {
            var_name,
            setter_name: Some(setter_name),
            initial_value,
            state_type: "dynamic".to_string(),
            is_client_state: false,
        });
    }
}

fn extract_use_effect(call: &CallExpr, component: &mut Component) {
    let dependencies = if call.args.len() > 1 {
        if let Some(arg) = call.args.get(1) {
            extract_dependency_array(&arg.expr)
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    component.use_effect.push(UseEffectInfo {
        dependencies,
        is_client_side: false,
    });
}

fn extract_use_ref(call: &CallExpr, binding: &Pat, component: &mut Component) {
    if let Pat::Ident(ident) = binding {
        let name = ident.id.sym.to_string();
        let initial_value = call.args.get(0)
            .map(|arg| expr_to_csharp(&arg.expr))
            .unwrap_or_else(|| "null".to_string());

        component.use_ref.push(UseRefInfo {
            name,
            initial_value,
        });
    }
}

fn extract_use_client_state(call: &CallExpr, binding: &Pat, component: &mut Component) {
    if let Pat::Array(arr) = binding {
        let var_name = arr.elems.get(0)
            .and_then(|e| e.as_ref())
            .and_then(|p| if let Pat::Ident(id) = p { Some(id.id.sym.to_string()) } else { None })
            .unwrap_or_default();

        let setter_name = arr.elems.get(1)
            .and_then(|e| e.as_ref())
            .and_then(|p| if let Pat::Ident(id) = p { Some(id.id.sym.to_string()) } else { None })
            .unwrap_or_default();

        let initial_value = call.args.get(0)
            .map(|arg| expr_to_csharp(&arg.expr))
            .unwrap_or_else(|| "null".to_string());

        component.use_client_state.push(UseStateInfo {
            var_name,
            setter_name: Some(setter_name),
            initial_value,
            state_type: "dynamic".to_string(),
            is_client_state: true,
        });
    }
}

fn extract_use_markdown(call: &CallExpr, binding: &Pat, component: &mut Component) {
    if let Pat::Array(arr) = binding {
        let name = arr.elems.get(0)
            .and_then(|e| e.as_ref())
            .and_then(|p| if let Pat::Ident(id) = p { Some(id.id.sym.to_string()) } else { None })
            .unwrap_or_default();

        let setter = arr.elems.get(1)
            .and_then(|e| e.as_ref())
            .and_then(|p| if let Pat::Ident(id) = p { Some(id.id.sym.to_string()) } else { None })
            .unwrap_or_default();

        let initial_value = call.args.get(0)
            .map(|arg| expr_to_csharp(&arg.expr))
            .unwrap_or_else(|| "null".to_string());

        component.use_markdown.push(UseMarkdownInfo {
            name,
            setter,
            initial_value,
        });
    }
}

fn extract_custom_hook(call: &CallExpr, binding: &Pat, hook_name: &str, component: &mut Component) {
    let instance_name = match binding {
        Pat::Ident(ident) => ident.id.sym.to_string(),
        _ => return,
    };

    let class_name = {
        let without_use = hook_name.strip_prefix("use").unwrap_or(hook_name);
        format!("{}Hook", without_use)
    };

    component.custom_hooks.push(CustomHookInstance {
        hook_name: hook_name.to_string(),
        instance_name,
        class_name,
        return_values: Vec::new(),
    });
}

fn extract_dependency_array(expr: &Expr) -> Vec<String> {
    match expr {
        Expr::Array(arr) => {
            arr.elems.iter()
                .filter_map(|elem| {
                    elem.as_ref().and_then(|e| {
                        match &*e.expr {
                            Expr::Ident(ident) => Some(ident.sym.to_string()),
                            _ => None,
                        }
                    })
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

fn expr_to_csharp(expr: &Expr) -> String {
    match expr {
        Expr::Lit(lit) => match lit {
            Lit::Str(s) => format!("\"{}\"", String::from_utf8_lossy(s.value.as_bytes())),
            Lit::Num(n) => n.value.to_string(),
            Lit::Bool(b) => b.value.to_string(),
            Lit::Null(_) => "null".to_string(),
            _ => "null".to_string(),
        },
        Expr::Ident(ident) => ident.sym.to_string(),
        Expr::Array(_) => "new List<dynamic>()".to_string(),
        Expr::Object(_) => "new Dictionary<string, dynamic>()".to_string(),
        _ => "null".to_string(),
    }
}

/// Process a program with the Minimact transformer
pub fn process_transform(mut program: Program, input_file_path: String) -> Program {
    let mut transformer = MinimactTransformer::new(input_file_path);
    program.visit_mut_with(&mut transformer);
    program
}
