use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Deserialize, Serialize)]
pub struct PluginMetadata {
    pub version: String,
    pub plugin: PluginInfo,
    #[serde(default)]
    pub structs: HashMap<String, StructDef>,
    #[serde(default)]
    pub functions: HashMap<String, FunctionDef>,
    #[serde(default)]
    pub babel_to_swc_mappings: HashMap<String, String>,
    pub visitor_context: Option<VisitorContext>,
    #[serde(default)]
    pub code_generation_hints: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PluginInfo {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StructDef {
    pub description: Option<String>,
    pub fields: HashMap<String, FieldDef>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FieldDef {
    #[serde(rename = "type")]
    pub field_type: String,
    pub required: Option<bool>,
    pub default: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FunctionDef {
    pub description: Option<String>,
    pub params: Vec<ParamDef>,
    pub returns: String,
    pub uses_string_builder: Option<bool>,
    pub babel_specific: Option<bool>,
    pub swc_implementation: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ParamDef {
    pub name: String,
    #[serde(rename = "type")]
    pub param_type: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VisitorContext {
    pub needs_parent_tracking: bool,
    pub parent_types: Vec<String>,
}

impl PluginMetadata {
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    }

    pub fn get_function_signature(&self, func_name: &str) -> Option<&FunctionDef> {
        self.functions.get(func_name)
    }

    pub fn get_return_type(&self, func_name: &str) -> Option<&str> {
        self.functions.get(func_name).map(|f| f.returns.as_str())
    }

    pub fn translate_babel_pattern(&self, pattern: &str) -> Option<&str> {
        self.babel_to_swc_mappings.get(pattern).map(|s| s.as_str())
    }

    pub fn get_struct(&self, struct_name: &str) -> Option<&StructDef> {
        self.structs.get(struct_name)
    }
}
