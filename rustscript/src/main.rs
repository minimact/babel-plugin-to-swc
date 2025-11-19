//! RustScript Compiler CLI

use clap::{Parser as ClapParser, Subcommand};
use std::fs;
use std::path::PathBuf;

use rustscript::{Lexer, Parser, analyze};

#[derive(ClapParser)]
#[command(name = "rustscript")]
#[command(about = "RustScript compiler - compile to Babel and SWC plugins")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Tokenize a RustScript file (for debugging)
    Lex {
        /// Input file
        file: PathBuf,
    },
    /// Parse a RustScript file (for debugging)
    Parse {
        /// Input file
        file: PathBuf,
    },
    /// Check a RustScript file for errors
    Check {
        /// Input file
        file: PathBuf,
    },
    /// Build a RustScript project
    Build {
        /// Target platform
        #[arg(short, long, default_value = "both")]
        target: String,
    },
}

fn main() {
    let cli = <Cli as ClapParser>::parse();

    match cli.command {
        Commands::Lex { file } => {
            let source = match fs::read_to_string(&file) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error reading file: {}", e);
                    std::process::exit(1);
                }
            };

            let mut lexer = Lexer::new(&source);
            let tokens = lexer.tokenize();

            println!("Tokens for {:?}:", file);
            println!("{:-<60}", "");
            for token in &tokens {
                println!(
                    "{:>4}:{:<3} {:?}",
                    token.span.line,
                    token.span.column,
                    token.kind
                );
            }
            println!("{:-<60}", "");
            println!("Total tokens: {}", tokens.len());
        }
        Commands::Parse { file } => {
            let source = match fs::read_to_string(&file) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error reading file: {}", e);
                    std::process::exit(1);
                }
            };

            let mut lexer = Lexer::new(&source);
            let tokens = lexer.tokenize();
            let mut parser = Parser::new(tokens);

            match parser.parse() {
                Ok(program) => {
                    println!("Successfully parsed {:?}", file);
                    println!("{:-<60}", "");
                    println!("{:#?}", program);
                }
                Err(e) => {
                    eprintln!("Parse error at {}:{}: {}", e.span.line, e.span.column, e.message);
                    std::process::exit(1);
                }
            }
        }
        Commands::Check { file } => {
            let source = match fs::read_to_string(&file) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error reading file: {}", e);
                    std::process::exit(1);
                }
            };

            let mut lexer = Lexer::new(&source);
            let tokens = lexer.tokenize();
            let mut parser = Parser::new(tokens);

            let program = match parser.parse() {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Parse error at {}:{}: {}", e.span.line, e.span.column, e.message);
                    std::process::exit(1);
                }
            };

            let result = analyze(&program);

            // Print errors
            for error in &result.errors {
                eprintln!(
                    "error[{}]: {} at {}:{}",
                    error.code, error.message, error.span.line, error.span.column
                );
                if let Some(ref hint) = error.hint {
                    eprintln!("  help: {}", hint);
                }
            }

            // Print warnings
            for warning in &result.warnings {
                eprintln!(
                    "warning[{}]: {} at {}:{}",
                    warning.code, warning.message, warning.span.line, warning.span.column
                );
                if let Some(ref hint) = warning.hint {
                    eprintln!("  help: {}", hint);
                }
            }

            if result.errors.is_empty() {
                println!("Check passed: {:?}", file);
                if !result.warnings.is_empty() {
                    println!("  {} warning(s)", result.warnings.len());
                }
            } else {
                eprintln!("Check failed: {} error(s)", result.errors.len());
                std::process::exit(1);
            }
        }
        Commands::Build { target: _ } => {
            eprintln!("Build not implemented yet");
            std::process::exit(1);
        }
    }
}
