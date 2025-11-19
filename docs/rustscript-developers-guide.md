# RustScript Developer's Guide

**Version:** 0.3.0
**Last Updated:** 2024

A comprehensive guide for writing AST transformation plugins in RustScript that compile to both Babel (JavaScript) and SWC (Rust).

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [Getting Started](#2-getting-started)
3. [Language Basics](#3-language-basics)
4. [The Unified AST](#4-the-unified-ast)
5. [Writing Plugins](#5-writing-plugins)
6. [Writing Writers (Transpilers)](#6-writing-writers-transpilers)
7. [Pattern Matching](#7-pattern-matching)
8. [Scoped Traversal](#8-scoped-traversal)
9. [Type System](#9-type-system)
10. [Best Practices](#10-best-practices)
11. [Platform Differences](#11-platform-differences)
12. [Troubleshooting](#12-troubleshooting)
13. [API Reference](#13-api-reference)

---

## 1. Introduction

### What is RustScript?

RustScript is a domain-specific language designed for writing AST (Abstract Syntax Tree) transformation plugins. It compiles to both:

- **Babel plugins** (JavaScript) for the Node.js ecosystem
- **SWC plugins** (Rust/WASM) for high-performance compilation

### Why RustScript?

Writing the same transformation logic twice—once in JavaScript for Babel and once in Rust for SWC—is error-prone and time-consuming. RustScript solves this by:

1. **Write Once, Run Anywhere**: Single source compiles to both targets
2. **Unified AST**: Abstract away ESTree vs swc_ecma_ast differences
3. **Type Safety**: Rust-inspired ownership model catches errors early
4. **Performance**: Generated SWC plugins run at native speed

### The Vector Alignment Principle

RustScript operates on the principle of "vector alignment"—finding the intersection of JavaScript and Rust capabilities. Features that work on one platform but not the other are either:

- Abstracted into unified constructs
- Explicitly marked as platform-specific
- Prohibited to ensure correctness

---

## 2. Getting Started

### Installation

```bash
# Build the RustScript compiler
cd rustscript
cargo build --release

# Add to PATH (optional)
export PATH="$PATH:/path/to/rustscript/target/release"
```

### Your First Plugin

Create a file `hello.rsc`:

```rustscript
/// A simple plugin that logs function names
plugin HelloPlugin {
    fn visit_function_declaration(node: &mut FunctionDeclaration, ctx: &Context) {
        let name = node.id.name.clone();
        // In a real plugin, you'd transform the AST here
        node.visit_children(self);
    }
}
```

### Building

```bash
# Build for both targets
rustscript build hello.rsc

# Build for specific target
rustscript build hello.rsc --target babel
rustscript build hello.rsc --target swc

# Specify output directory
rustscript build hello.rsc -o dist
```

### Output Structure

```
dist/
├── index.js    # Babel plugin
└── lib.rs      # SWC plugin
```

---

## 3. Language Basics

### Comments

```rustscript
// Single-line comment

/// Documentation comment (included in output)

/* Multi-line
   comment */
```

### Variables

```rustscript
// Immutable binding
let x = 5;
let name = "hello";

// Mutable binding
let mut count = 0;
count += 1;

// Constants (compile-time)
const MAX_DEPTH = 10;
```

### Types

#### Primitive Types

```rustscript
let flag: bool = true;
let count: i32 = 42;
let index: u32 = 0;
let ratio: f64 = 3.14;
```

#### String Type

```rustscript
// Str is the unified string type
// Compiles to: String (JS) / JsWord (SWC)
let name: Str = "hello";

// String operations
let upper = name.to_uppercase();
let len = name.len();
let starts = name.starts_with("he");
```

#### Container Types

```rustscript
// Vector
let mut items: Vec<Str> = vec![];
items.push("first");
items.insert(0, "zeroth");

// HashMap
let mut map: HashMap<Str, i32> = HashMap::new();
map.insert("key", 42);

// Option
let maybe: Option<Str> = Some("value");
if let Some(val) = maybe {
    // use val
}
```

### Control Flow

```rustscript
// If-else
if condition {
    // ...
} else if other {
    // ...
} else {
    // ...
}

// Match
match value {
    1 => handle_one(),
    2 | 3 => handle_two_or_three(),
    _ => handle_other(),
}

// Loops
for item in &items {
    // ...
}

while condition {
    // ...
}

loop {
    if done {
        break;
    }
}
```

### Functions

```rustscript
// Basic function
fn add(a: i32, b: i32) -> i32 {
    return a + b;
}

// Public function (exported)
pub fn helper(name: &Str) -> bool {
    return name.len() > 0;
}

// Function with mutable parameter
fn transform(node: &mut Expr) {
    // modify node
}
```

---

## 4. The Unified AST

### Node Types

RustScript uses unified node type names that map to both platforms:

| RustScript | Babel (ESTree) | SWC |
|------------|----------------|-----|
| `Program` | `Program` | `Module` |
| `FunctionDeclaration` | `FunctionDeclaration` | `FnDecl` |
| `VariableDeclaration` | `VariableDeclaration` | `VarDecl` |
| `Identifier` | `Identifier` | `Ident` |
| `CallExpression` | `CallExpression` | `CallExpr` |
| `MemberExpression` | `MemberExpression` | `MemberExpr` |
| `BinaryExpression` | `BinaryExpression` | `BinExpr` |
| `StringLiteral` | `StringLiteral` | `Str` |
| `NumericLiteral` | `NumericLiteral` | `Number` |

### Field Access

Field names are also unified:

```rustscript
// Identifier
let name = ident.name;  // .name (Babel) / .sym (SWC)

// MemberExpression
let obj = member.object;     // .object (Babel) / .obj (SWC)
let prop = member.property;  // .property (Babel) / .prop (SWC)

// CallExpression
let callee = call.callee;       // same
let args = call.arguments;      // .arguments (Babel) / .args (SWC)

// FunctionDeclaration
let id = func.id;
let params = func.params;
let body = func.body;
```

### Creating Nodes

```rustscript
// Create an identifier
let id = Identifier {
    name: "myVar",
};

// Create a call expression
let call = CallExpression {
    callee: Identifier { name: "console" },
    arguments: vec![
        StringLiteral { value: "Hello" },
    ],
};

// Create a member expression
let member = MemberExpression {
    object: Identifier { name: "console" },
    property: Identifier { name: "log" },
};
```

---

## 5. Writing Plugins

### Plugin Structure

```rustscript
plugin MyPlugin {
    // State (becomes struct fields)
    struct State {
        count: i32,
        found_items: Vec<Str>,
    }

    // Visitor methods
    fn visit_call_expression(node: &mut CallExpression, ctx: &Context) {
        // Transform the node
        node.visit_children(self);
    }

    // Helper functions
    fn is_console_log(node: &CallExpression) -> bool {
        // ...
    }
}
```

### Visitor Methods

Visitor methods follow the naming convention `visit_<node_type>`:

```rustscript
fn visit_program(node: &mut Program, ctx: &Context) { }
fn visit_function_declaration(node: &mut FunctionDeclaration, ctx: &Context) { }
fn visit_call_expression(node: &mut CallExpression, ctx: &Context) { }
fn visit_identifier(node: &mut Identifier, ctx: &Context) { }
// ... etc
```

### The Context Object

The `ctx` parameter provides traversal context:

```rustscript
fn visit_identifier(node: &mut Identifier, ctx: &Context) {
    // Get the filename being processed
    let file = ctx.filename;

    // Get parent information (when available)
    // Note: Platform-specific behavior
}
```

### Traversal Control

```rustscript
fn visit_function_declaration(node: &mut FunctionDeclaration, ctx: &Context) {
    // Process children AFTER this node (post-order)
    // Do your transformation first
    transform_function(node);

    // Then visit children
    node.visit_children(self);
}

fn visit_call_expression(node: &mut CallExpression, ctx: &Context) {
    // Process children BEFORE this node (pre-order)
    node.visit_children(self);

    // Then do your transformation
    transform_call(node);
}

fn visit_block_statement(node: &mut BlockStatement, ctx: &Context) {
    // Skip children entirely
    // Don't call visit_children
}
```

### Node Replacement (Statement Lowering)

Replace a node using pointer assignment:

```rustscript
fn visit_call_expression(node: &mut CallExpression, ctx: &Context) {
    if is_deprecated_api(node) {
        // Replace with new API call
        *node = CallExpression {
            callee: Identifier { name: "newApi" },
            arguments: node.arguments.clone(),
        };
    }
}
```

This compiles to:
- **Babel**: `path.replaceWith(t.callExpression(...))`
- **SWC**: Direct assignment `*node = CallExpr { ... }`

### Removing Nodes

```rustscript
fn visit_expression_statement(node: &mut ExpressionStatement, ctx: &Context) {
    if should_remove(node) {
        // Mark for removal
        node.remove();
    }
}
```

---

## 6. Writing Writers (Transpilers)

Writers are read-only visitors for transpilation (generating different output):

```rustscript
writer ReactToOrleans {
    builder: CodeBuilder,

    fn init() -> Self {
        Self { builder: CodeBuilder::new() }
    }

    fn visit_function_declaration(node: &FunctionDeclaration, ctx: &Context) {
        // Generate C# class
        self.builder.append("public class ");
        self.builder.append(node.id.name.clone());
        self.builder.append(" : Grain {\n");

        self.builder.indent();
        node.visit_children(self);
        self.builder.dedent();

        self.builder.append("}\n");
    }

    fn finish(self) -> Str {
        self.builder.to_string()
    }
}
```

### CodeBuilder API

```rustscript
let mut builder = CodeBuilder::new();

// Append text
builder.append("text");
builder.append_line("line with newline");

// Indentation
builder.indent();   // Increase indent level
builder.dedent();   // Decrease indent level

// Get result
let output = builder.to_string();
```

---

## 7. Pattern Matching

### The `matches!` Macro

Type-check nodes without extracting values:

```rustscript
if matches!(node, Identifier) {
    // node is an Identifier
}

if matches!(node.callee, MemberExpression) {
    // callee is a MemberExpression
}
```

### Nested Pattern Matching

```rustscript
if matches!(node.callee, MemberExpression {
    object: Identifier { name: "console" },
    property: Identifier { name: "log" }
}) {
    // This is console.log
}
```

### Compiled Output

**Babel:**
```javascript
if (
    t.isMemberExpression(node.callee) &&
    t.isIdentifier(node.callee.object, { name: "console" }) &&
    t.isIdentifier(node.callee.property, { name: "log" })
) {
    // matched
}
```

**SWC:**
```rust
if let Expr::Member(member) = &node.callee {
    if let Expr::Ident(obj) = &*member.obj {
        if &*obj.sym == "console" {
            if let MemberProp::Ident(prop) = &member.prop {
                if &*prop.sym == "log" {
                    // matched
                }
            }
        }
    }
}
```

### Flow-Sensitive Typing

When you use `matches!` in a condition, the variable is automatically narrowed:

```rustscript
fn visit_expression(node: &mut Expr, ctx: &Context) {
    // node is Expr (enum)

    if matches!(node, CallExpression) {
        // Inside this block, 'node' is narrowed to CallExpression
        // You can access .callee, .arguments directly
        let callee = node.callee;
    }

    // Outside the block, node is back to Expr
}
```

---

## 8. Scoped Traversal

### Inline Traversal

Define a one-off visitor for a subtree:

```rustscript
fn visit_function_declaration(func: &mut FunctionDeclaration, ctx: &Context) {
    // Count returns in this function only
    traverse(func.body) {
        let count = 0;

        fn visit_return_statement(ret: &mut ReturnStatement, ctx: &Context) {
            self.count += 1;
        }
    }

    // 'count' is now available here if needed
}
```

### Delegated Traversal

Apply another plugin/visitor:

```rustscript
plugin Cleanup {
    fn visit_identifier(node: &mut Identifier, ctx: &Context) {
        // cleanup logic
    }
}

plugin Main {
    fn visit_function(node: &mut Function, ctx: &Context) {
        if node.is_async {
            // Apply Cleanup to this subtree
            traverse(node) using Cleanup;
        }
    }
}
```

### Manual Iteration

Selectively visit children:

```rustscript
fn visit_block_statement(node: &mut BlockStatement, ctx: &Context) {
    // Don't call node.visit_children(self)

    for stmt in &mut node.stmts {
        if needs_special_handling(stmt) {
            traverse(stmt) using SpecialVisitor;
        } else {
            stmt.visit_with(self);
        }
    }
}
```

---

## 9. Type System

### Ownership and Borrowing

RustScript enforces Rust-like ownership rules:

```rustscript
// Immutable borrow
fn read_name(node: &Identifier) -> Str {
    return node.name.clone();  // Must clone to own the value
}

// Mutable borrow
fn update_name(node: &mut Identifier) {
    node.name = "newName";
}
```

### Clone-to-Own

When extracting values from borrowed references, you must explicitly clone:

```rustscript
fn visit_identifier(node: &mut Identifier, ctx: &Context) {
    // ERROR: Cannot move out of borrowed content
    // let name = node.name;

    // CORRECT: Clone to own
    let name = node.name.clone();
}
```

### Type Inference

Types are inferred when possible:

```rustscript
let x = 5;           // i32
let s = "hello";     // Str
let v = vec![];      // Vec<_> (needs context)
let v: Vec<Str> = vec![];  // Explicit when needed
```

---

## 10. Best Practices

### 1. Prefer Immutable Bindings

```rustscript
// Good
let name = node.name.clone();

// Only use mut when necessary
let mut count = 0;
```

### 2. Clone Explicitly

```rustscript
// Good - clear ownership
let args = node.arguments.clone();

// Avoid - unclear ownership
let args = node.arguments;  // May or may not work
```

### 3. Use Helper Functions

```rustscript
plugin MyPlugin {
    fn visit_call_expression(node: &mut CallExpression, ctx: &Context) {
        if Self::is_console_log(node) {
            Self::transform_console_log(node);
        }
    }

    fn is_console_log(node: &CallExpression) -> bool {
        matches!(node.callee, MemberExpression {
            object: Identifier { name: "console" },
            property: Identifier { name: "log" }
        })
    }

    fn transform_console_log(node: &mut CallExpression) {
        // ...
    }
}
```

### 4. Handle All Cases

```rustscript
fn get_name(expr: &Expr) -> Option<Str> {
    if matches!(expr, Identifier) {
        return Some(expr.name.clone());
    }
    return None;  // Don't forget the None case
}
```

### 5. Document Your Code

```rustscript
/// Transform console.log calls to custom logger
///
/// Example:
///   console.log("hello") → logger.info("hello")
fn visit_call_expression(node: &mut CallExpression, ctx: &Context) {
    // ...
}
```

---

## 11. Platform Differences

### String Interning

**Babel:** Strings are regular JavaScript strings
**SWC:** Strings are interned as `JsWord`/`Atom`

RustScript abstracts this with the `Str` type, but be aware of performance implications when creating many strings in SWC.

### AST Structure

**Babel (ESTree):** Flat hierarchy, everything is a "Node"
**SWC:** Strict enum/struct hierarchy (`Expr`, `Stmt`, `Decl`, etc.)

RustScript's `matches!` macro handles this, but you may need to think about which enum variant you're working with.

### Box Wrapping

SWC uses `Box<T>` for recursive types. RustScript handles this automatically, but generated code may include dereferences.

### MemberProp vs Expression

In Babel, `member.property` is an `Expression`.
In SWC, `member.prop` is a `MemberProp` enum.

RustScript handles this in pattern matching, but be aware when debugging generated code.

---

## 12. Troubleshooting

### Common Errors

#### "Cannot move out of borrowed content"

```rustscript
// Wrong
let name = node.name;

// Right
let name = node.name.clone();
```

#### "Type mismatch"

Check that you're using the correct type. The compiler will tell you what it expected vs what it found.

#### "Pattern doesn't match"

Make sure your `matches!` pattern uses the correct node type name (RustScript names, not platform-specific).

### Debugging Generated Code

1. Build with verbose output to see intermediate steps
2. Check the generated `index.js` and `lib.rs` files
3. For Babel, add `console.log` statements
4. For SWC, add `dbg!()` macros

### Performance Issues

1. Avoid cloning large node trees unnecessarily
2. Use `traverse` for targeted subtree processing
3. Short-circuit expensive checks early

---

## 13. API Reference

### Built-in Functions

```rustscript
// String operations
str.len() -> usize
str.is_empty() -> bool
str.starts_with(prefix: &Str) -> bool
str.ends_with(suffix: &Str) -> bool
str.contains(substr: &Str) -> bool
str.to_uppercase() -> Str
str.to_lowercase() -> Str

// Vector operations
vec.len() -> usize
vec.is_empty() -> bool
vec.push(item: T)
vec.pop() -> Option<T>
vec.insert(index: usize, item: T)
vec.remove(index: usize) -> T
vec.get(index: usize) -> Option<&T>

// HashMap operations
map.insert(key: K, value: V)
map.get(key: &K) -> Option<&V>
map.contains_key(key: &K) -> bool
map.remove(key: &K) -> Option<V>
```

### Context API

```rustscript
ctx.filename: Str           // Current file being processed
ctx.source: Option<Str>     // Source code (if available)
```

### Node Methods

```rustscript
node.visit_children(self)   // Visit all children
node.visit_with(visitor)    // Visit with specific visitor
node.remove()               // Mark for removal
```

### CodeBuilder API

```rustscript
CodeBuilder::new() -> CodeBuilder
builder.append(text: &Str)
builder.append_line(text: &Str)
builder.indent()
builder.dedent()
builder.to_string() -> Str
```

---

## Appendix: Full Example

```rustscript
/// Transform console.log to a custom logger
///
/// Converts:
///   console.log("message")
/// To:
///   Logger.info("message")

plugin ConsoleToLogger {
    struct State {
        transformations: i32,
    }

    fn visit_call_expression(node: &mut CallExpression, ctx: &Context) {
        if Self::is_console_log(node) {
            Self::transform_to_logger(node);
            self.state.transformations += 1;
        }

        node.visit_children(self);
    }

    fn is_console_log(node: &CallExpression) -> bool {
        matches!(node.callee, MemberExpression {
            object: Identifier { name: "console" },
            property: Identifier { name: "log" }
        })
    }

    fn transform_to_logger(node: &mut CallExpression) {
        // Replace console.log with Logger.info
        *node = CallExpression {
            callee: MemberExpression {
                object: Identifier { name: "Logger" },
                property: Identifier { name: "info" },
            },
            arguments: node.arguments.clone(),
        };
    }
}
```

Build and use:

```bash
# Build the plugin
rustscript build console-to-logger.rsc -o dist

# Use with Babel
npx babel src --plugins ./dist/index.js

# Use with SWC (after compiling lib.rs to WASM)
npx swc src --plugin ./dist/plugin.wasm
```

---

## Further Reading

- [RustScript Language Specification](rustscript-specification.md)
- [Enhancement Plan](rustscript-enhancement-plan.md)
- [Compiler Implementation](rustscript-compiler-implementation.md)
- [Babel Plugin Handbook](https://github.com/jamiebuilds/babel-handbook)
- [SWC Plugin Documentation](https://swc.rs/docs/plugin/ecmascript/getting-started)
