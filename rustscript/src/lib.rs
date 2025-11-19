//! RustScript Compiler
//!
//! A language that compiles to both Babel (JavaScript) and SWC (Rust) plugins.

pub mod lexer;
pub mod parser;
pub mod semantic;
pub mod codegen;
pub mod mapping;
// pub mod error;
// pub mod prelude;

pub use lexer::{Lexer, Token, TokenKind, Span};
pub use parser::{Parser, Program, ParseError};
pub use semantic::{analyze, SemanticError, SemanticResult};
pub use codegen::{generate, Target, GeneratedCode};
pub use mapping::{
    NodeMapping, FieldMapping, HelperMapping, PatternMapping,
    get_node_mapping, get_field_mapping, get_helper_for_field, get_pattern_check,
};
