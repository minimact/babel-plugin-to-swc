# RustScript Language Specification

**Version:** 0.3.0
**Status:** Draft (with Scoped Traversal)
**Target Platforms:** Babel (JavaScript) & SWC (Rust/WASM)

---

## 1. Overview

RustScript is a domain-specific language for writing AST transformation plugins that compile to both Babel (JavaScript) and SWC (Rust). It enforces a strict visitor pattern with explicit ownership semantics that map cleanly to both garbage-collected and borrow-checked runtimes.

### 1.1 Design Philosophy

- **Strict Visitor, Loose Context**: Enforces Rust-like `VisitMut` pattern
- **Immutable by Default**: All mutations must be explicit
- **No Path Magic**: No implicit parent traversal; state must be tracked explicitly
- **Unified AST**: Subset of nodes common to ESTree (Babel) and swc_ecma_ast
- **Clone-to-Own**: Explicit `.clone()` required for value extraction

### 1.2 Vector Alignment Principle

RustScript finds the intersection of JavaScript and Rust capabilities, not the union. Code that compiles must be semantically valid in both targets.

---

## 2. Lexical Structure

### 2.1 Keywords

```
plugin      fn          let         const       if          else
match       return      true        false       null        for
in          while       break       continue    struct      enum
impl        use         pub         mut         self        Self
traverse    using       writer
```

### 2.2 Reserved Keywords (Future Use)

```
async       await       trait       where       type        as
loop        mod         crate       super       dyn         static
```

### 2.3 Operators

```
// Arithmetic
+   -   *   /   %

// Comparison
==  !=  <   >   <=  >=

// Logical
&&  ||  !

// Assignment
=   +=  -=  *=  /=

// Reference/Dereference
&   *

// Member Access
.   ::

// Special
=>  ->  ..  ...
```

### 2.4 Comments

```rustscript
// Single-line comment

/*
   Multi-line comment
*/

/// Documentation comment (preserved in output)
```

### 2.5 Literals

```rustscript
// Strings (always use Str type internally)
"hello world"
"escaped \"quotes\""

// Numbers
42
3.14
0xFF
0b1010

// Booleans
true
false

// Null
null
```

---

## 3. Type System

### 3.1 Primitive Types

| RustScript | Babel (JS) | SWC (Rust) |
|------------|------------|------------|
| `Str` | `string` | `JsWord` / `Atom` |
| `i32` | `number` | `i32` |
| `f64` | `number` | `f64` |
| `bool` | `boolean` | `bool` |
| `()` | `undefined` | `()` |

### 3.2 Reference Types

```rustscript
&T          // Immutable reference
&mut T      // Mutable reference
```

**Compilation:**
- **Babel**: References are ignored; values passed directly
- **SWC**: References preserved as-is

### 3.3 Container Types

```rustscript
Vec<T>      // Dynamic array
Option<T>   // Optional value (Some/None)
HashMap<K, V>  // Key-value map
HashSet<T>  // Unique set
```

**Compilation:**

| RustScript | Babel (JS) | SWC (Rust) |
|------------|------------|------------|
| `Vec<T>` | `Array` | `Vec<T>` |
| `Option<T>` | `T \| null` | `Option<T>` |
| `HashMap<K,V>` | `Map` or `Object` | `HashMap<K,V>` |
| `HashSet<T>` | `Set` | `HashSet<T>` |

### 3.4 The CodeBuilder Type

For generating code (transpiler use case):

```rustscript
// A buffer that handles platform-specific string concatenation
type CodeBuilder;

impl CodeBuilder {
    fn new() -> CodeBuilder;
    fn append(s: Str);
    fn newline();
    fn indent();
    fn dedent();
    fn to_string() -> Str;
}
```

**Compilation:**
- **Babel**: Uses array-based string building with `.join()`
- **SWC**: Uses `String` with `push_str()`

### 3.5 AST Node Types

See Section 7 (Unified AST) for complete mapping.

```rustscript
Identifier
CallExpression
MemberExpression
BinaryExpression
// ... etc
```

---

## 4. Declarations

### 4.1 Plugin Declaration

Every RustScript file must declare a plugin:

```rustscript
plugin MyTransformer {
    // visitor methods and helpers
}
```

**Compiles to:**

```javascript
// Babel
module.exports = function({ types: t }) {
  return {
    visitor: { /* ... */ }
  };
};
```

```rust
// SWC
pub struct MyTransformer;
impl VisitMut for MyTransformer { /* ... */ }
```

### 4.2 Function Declaration

```rustscript
fn function_name(param: Type, param2: &mut Type) -> ReturnType {
    // body
}
```

**Visibility:**

```rustscript
pub fn public_function() { }    // Exported
fn private_function() { }       // Internal
```

### 4.3 Variable Declaration

```rustscript
let name = value;           // Immutable binding
let mut name = value;       // Mutable binding
const NAME = value;         // Compile-time constant
```

**Clone Requirement:**

```rustscript
// Extracting from a reference requires explicit clone
let name = node.name.clone();   // Required
let name = node.name;           // ERROR: implicit borrow
```

**Compilation:**

```javascript
// Babel: .clone() is stripped
const name = node.name;
```

```rust
// SWC: .clone() preserved
let name = node.name.clone();
```

### 4.4 Struct Declaration

```rustscript
struct ComponentInfo {
    name: Str,
    props: Vec<Prop>,
    has_state: bool,
}
```

**Compiles to:**

```javascript
// Babel: Plain object shape (documentation only)
// Runtime: { name: string, props: Array, has_state: boolean }
```

```rust
// SWC
#[derive(Clone, Debug)]
pub struct ComponentInfo {
    pub name: String,
    pub props: Vec<Prop>,
    pub has_state: bool,
}
```

### 4.5 Enum Declaration

```rustscript
enum HookType {
    State,
    Effect,
    Ref,
    Custom(Str),
}
```

**Compiles to:**

```javascript
// Babel: Tagged union pattern
// { type: "State" } | { type: "Effect" } | { type: "Custom", value: string }
```

```rust
// SWC
pub enum HookType {
    State,
    Effect,
    Ref,
    Custom(String),
}
```

---

## 5. Visitor Methods

### 5.1 Visitor Function Signature

```rustscript
plugin MyPlugin {
    fn visit_<node_type>(node: &mut NodeType, ctx: &Context) {
        // transformation logic
    }
}
```

**Naming Convention:**
- `visit_identifier` → visits `Identifier` nodes
- `visit_call_expression` → visits `CallExpression` nodes
- Snake_case function name maps to PascalCase node type

### 5.2 Context Object

The `Context` provides limited scope analysis available in both engines:

```rustscript
ctx.scope              // Current scope information
ctx.filename           // Source filename
ctx.generate_uid(hint) // Generate unique identifier
```

### 5.3 Node Replacement (Statement Lowering)

Direct assignment to the visitor argument triggers replacement:

```rustscript
fn visit_call_expression(node: &mut CallExpression, ctx: &Context) {
    // This is "statement lowering" - not regular assignment
    *node = CallExpression {
        callee: Identifier::new("newName"),
        arguments: vec![],
    };
}
```

**Compiles to:**

```javascript
// Babel: Lowered to path.replaceWith()
path.replaceWith(t.callExpression(
    t.identifier("newName"),
    []
));
```

```rust
// SWC: Direct assignment
*node = CallExpr {
    callee: Callee::Expr(Box::new(Expr::Ident(Ident::new("newName".into(), DUMMY_SP)))),
    args: vec![],
    ..Default::default()
};
```

### 5.4 Node Removal

In Babel, this is `path.remove()`. In SWC, this requires replacing with a NoOp or filtering. RustScript standardizes this:

```rustscript
fn visit_expression_statement(node: &mut ExpressionStatement, ctx: &Context) {
    if should_remove(node) {
        *node = Statement::noop(); // Compiles to path.remove() or Stmt::Empty
    }
}
```

**Compiles to:**

```javascript
// Babel
if (shouldRemove(node)) {
    path.remove();
}
```

```rust
// SWC
if should_remove(node) {
    *node = Stmt::Empty(EmptyStmt { span: DUMMY_SP });
}
```

### 5.5 List/Sibling Manipulation

Inserting siblings is difficult in SWC's VisitMut. RustScript solves this by exposing a `flat_map_in_place` helper:

```rustscript
fn visit_block_statement(node: &mut BlockStatement, ctx: &Context) {
    // "stmts" is a special view of the block's body
    node.stmts.flat_map_in_place(|stmt| {
        if stmt.is_return() {
            // Replace 1 statement with 2 (Injection)
            vec![
                Statement::expression(log_call()),
                stmt.clone()
            ]
        } else {
            // Keep as-is
            vec![stmt.clone()]
        }
    });
}
```

**Compiles to:**

```javascript
// Babel: Iterates path.get('body') and uses path.replaceWithMultiple()
const body = path.get('body');
const newBody = [];
for (const stmt of body) {
    if (t.isReturnStatement(stmt.node)) {
        newBody.push(t.expressionStatement(logCall()));
        newBody.push(stmt.node);
    } else {
        newBody.push(stmt.node);
    }
}
path.node.body = newBody;
```

```rust
// SWC: Uses flat_map inside visit_mut_block_stmt
let new_stmts: Vec<Stmt> = node.stmts.iter().flat_map(|stmt| {
    if matches!(stmt, Stmt::Return(_)) {
        vec![
            Stmt::Expr(ExprStmt { expr: Box::new(log_call()), span: DUMMY_SP }),
            stmt.clone()
        ]
    } else {
        vec![stmt.clone()]
    }
}).collect();
node.stmts = new_stmts;
```

### 5.6 Property Mutation (Forbidden)

Direct property mutation is NOT allowed:

```rustscript
// ERROR: Cannot mutate properties directly
node.name = "newName";
```

**Rationale:** Babel's scope tracker may not be notified. Always replace the whole node or use clone-and-rebuild pattern.

### 5.7 Traversal Control

```rustscript
fn visit_call_expression(node: &mut CallExpression, ctx: &Context) {
    // Process children first (post-order)
    node.visit_children(self);

    // Or skip children entirely
    // (don't call visit_children)
}
```

### 5.8 Scoped Traversal (`traverse`)

RustScript allows interrupting the current visitor to perform a scoped traversal on a specific node using a different set of rules. This bridges the impedance mismatch between Babel's `path.traverse` (graph walk) and SWC's `visit_mut_with` (recursive function call).

#### 5.8.1 Inline Traversal

Use `traverse(node) { ... }` to define a one-off visitor for a subtree:

```rustscript
fn visit_function_declaration(func: &mut FunctionDeclaration, ctx: &Context) {
    for stmt in &mut func.body.stmts {
        if stmt.is_if_statement() {
            // Spawn a nested visitor for just this statement
            traverse(stmt) {
                // Local state (becomes struct fields in Rust, object properties in JS)
                let found_returns = 0;

                fn visit_return_statement(ret: &mut ReturnStatement, ctx: &Context) {
                    ret.argument = None;
                    self.found_returns += 1;
                }
            }
        }
    }
}
```

**Compiles to:**

```javascript
// Babel
const __nestedVisitor = {
    state: { found_returns: 0 },
    ReturnStatement(path) {
        const ret = path.node;
        ret.argument = null;
        this.state.found_returns++;
    },
};
stmt.traverse(__nestedVisitor);
```

```rust
// SWC (with hoisted struct)
struct __InlineVisitor_0 {
    found_returns: i32,
}

impl VisitMut for __InlineVisitor_0 {
    fn visit_mut_return_stmt(&mut self, ret: &mut ReturnStmt) {
        ret.arg = None;
        self.found_returns += 1;
    }
}

// At usage site:
let mut __visitor = __InlineVisitor_0 { found_returns: 0 };
stmt.visit_mut_with(&mut __visitor);
```

**Capture Rules:**
- **Immutable**: Ambient variables from the parent scope can be read (cloned into the new visitor)
- **Mutable**: Ambient variables cannot be mutated from inside `traverse` (Rust borrow checker). Pass data via initial state instead.

#### 5.8.2 Delegated Traversal (`using`)

Use `traverse(node) using VisitorName` to apply a separately defined plugin/visitor:

```rustscript
plugin CleanUp {
    fn visit_identifier(n: &mut Identifier, ctx: &Context) {
        // cleanup logic
    }
}

plugin Main {
    fn visit_function(node: &mut Function, ctx: &Context) {
        if node.is_async {
            // Route this subtree through the CleanUp visitor
            traverse(node) using CleanUp;
        }
    }
}
```

**Compiles to:**

```javascript
// Babel
node.traverse(CleanUp);
```

```rust
// SWC
let mut __visitor = CleanUp::default();
node.visit_mut_with(&mut __visitor);
```

#### 5.8.3 Manual Iteration Pattern

This pattern is required when you want to selectively visit children:

```rustscript
fn visit_block_statement(node: &mut BlockStatement, ctx: &Context) {
    // 1. Do NOT call node.visit_children(self);

    // 2. Manually iterate
    for stmt in &mut node.stmts {
        if needs_special_handling(stmt) {
            traverse(stmt) using SpecialVisitor;
        } else {
            // Continue with current visitor
            stmt.visit_with(self);
        }
    }
}
```

#### 5.8.4 The "Root Node" Guarantee

RustScript guarantees that the visitor runs on the node passed to `traverse`, not just its children:

- **SWC**: Maps to `node.visit_mut_with(&mut visitor)`
- **Babel**: Maps to `path.traverse(visitor)` + manual visit of `path.node`

This ensures consistent behavior across both targets.

---

## 6. Pattern Matching

### 6.1 The `matches!` Macro

Pattern matching that works on both targets:

```rustscript
if matches!(node.callee, MemberExpression {
    object: Identifier { name: "console" },
    property: Identifier { name: "log" }
}) {
    // matched
}
```

**Compiles to:**

```javascript
// Babel
if (
    t.isMemberExpression(node.callee) &&
    t.isIdentifier(node.callee.object, { name: "console" }) &&
    t.isIdentifier(node.callee.property, { name: "log" })
) {
    // matched
}
```

```rust
// SWC
if let Callee::Expr(expr) = &node.callee {
    if let Expr::Member(member) = &**expr {
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
}
```

### 6.2 Match Expression

```rustscript
match node.operator {
    "+" => handle_add(),
    "-" => handle_sub(),
    "*" | "/" => handle_mul_div(),
    _ => handle_default(),
}
```

### 6.3 If-Let Pattern

```rustscript
if let Some(name) = get_identifier_name(node) {
    // use name
}
```

**Compiles to:**

```javascript
// Babel
const name = getIdentifierName(node);
if (name != null) {
    // use name
}
```

```rust
// SWC
if let Some(name) = get_identifier_name(node) {
    // use name
}
```

---

## 7. Unified AST (U-AST)

### 7.1 Node Mapping Table

| RustScript | Babel (ESTree) | SWC |
|------------|----------------|-----|
| `Identifier` | `t.Identifier` | `Ident` |
| `CallExpression` | `t.CallExpression` | `CallExpr` |
| `MemberExpression` | `t.MemberExpression` | `MemberExpr` |
| `BinaryExpression` | `t.BinaryExpression` | `BinExpr` |
| `UnaryExpression` | `t.UnaryExpression` | `UnaryExpr` |
| `StringLiteral` | `t.StringLiteral` | `Str` |
| `NumericLiteral` | `t.NumericLiteral` | `Number` |
| `BooleanLiteral` | `t.BooleanLiteral` | `Bool` |
| `NullLiteral` | `t.NullLiteral` | `Null` |
| `ArrayExpression` | `t.ArrayExpression` | `ArrayLit` |
| `ObjectExpression` | `t.ObjectExpression` | `ObjectLit` |
| `ArrowFunctionExpression` | `t.ArrowFunctionExpression` | `ArrowExpr` |
| `FunctionDeclaration` | `t.FunctionDeclaration` | `FnDecl` |
| `VariableDeclaration` | `t.VariableDeclaration` | `VarDecl` |
| `IfStatement` | `t.IfStatement` | `IfStmt` |
| `ReturnStatement` | `t.ReturnStatement` | `ReturnStmt` |
| `BlockStatement` | `t.BlockStatement` | `BlockStmt` |
| `ExpressionStatement` | `t.ExpressionStatement` | `ExprStmt` |
| `JSXElement` | `t.JSXElement` | `JSXElement` |
| `JSXAttribute` | `t.JSXAttribute` | `JSXAttr` |
| `JSXExpressionContainer` | `t.JSXExpressionContainer` | `JSXExprContainer` |
| `JSXText` | `t.JSXText` | `JSXText` |

### 7.2 TypeScript Node Mapping

| RustScript | Babel (ESTree) | SWC |
|------------|----------------|-----|
| `TSInterfaceDeclaration` | `TSInterfaceDeclaration` | `TsInterfaceDecl` |
| `TSPropertySignature` | `TSPropertySignature` | `TsPropertySignature` |
| `TSMethodSignature` | `TSMethodSignature` | `TsMethodSignature` |
| `TSTypeReference` | `TSTypeReference` | `TsTypeRef` |
| `TSTypeAnnotation` | `TSTypeAnnotation` | `TsTypeAnn` |
| `TSTypeAliasDeclaration` | `TSTypeAliasDeclaration` | `TsTypeAliasDecl` |

**Note:** TypeScript nodes use the `TS` prefix in RustScript to distinguish them from JavaScript nodes. When accessing fields on TypeScript nodes, be aware that:
- `key` on `TSPropertySignature` is `Box<Expr>` in SWC (requires pattern matching to extract identifier name)
- `type_args` on `CallExpression` provides access to generic type arguments like `useState<string>`

### 7.3 Node Construction

```rustscript
// Creating nodes
let id = Identifier::new("myVar");
let call = CallExpression {
    callee: id,
    arguments: vec![StringLiteral::new("arg")],
};
```

**Compiles to:**

```javascript
// Babel
const id = t.identifier("myVar");
const call = t.callExpression(id, [t.stringLiteral("arg")]);
```

```rust
// SWC
let id = Ident::new("myVar".into(), DUMMY_SP);
let call = CallExpr {
    callee: Callee::Expr(Box::new(Expr::Ident(id))),
    args: vec![ExprOrSpread {
        expr: Box::new(Expr::Lit(Lit::Str(Str::from("arg")))),
        spread: None,
    }],
    ..Default::default()
};
```

### 7.4 Node Type Checking

```rustscript
// Type checking
if node.is_identifier() { }
if node.is_call_expression() { }
```

**Compiles to:**

```javascript
// Babel
if (t.isIdentifier(node)) { }
if (t.isCallExpression(node)) { }
```

```rust
// SWC
if matches!(node, Expr::Ident(_)) { }
if matches!(node, Expr::Call(_)) { }
```

---

## 8. String Handling

### 8.1 The `Str` Type

All strings in RustScript use the `Str` type:

```rustscript
let name: Str = "hello";
```

### 8.2 String Comparison

```rustscript
if node.name == "console" {
    // works on both targets
}
```

**Compiles to:**

```javascript
// Babel
if (node.name === "console") { }
```

```rust
// SWC: JsWord implements PartialEq<&str>
if &*node.sym == "console" { }
```

### 8.3 String Methods

| RustScript | Babel (JS) | SWC (Rust) |
|------------|------------|------------|
| `s.starts_with("x")` | `s.startsWith("x")` | `s.starts_with("x")` |
| `s.ends_with("x")` | `s.endsWith("x")` | `s.ends_with("x")` |
| `s.contains("x")` | `s.includes("x")` | `s.contains("x")` |
| `s.len()` | `s.length` | `s.len()` |
| `s.is_empty()` | `s.length === 0` | `s.is_empty()` |
| `s.to_uppercase()` | `s.toUpperCase()` | `s.to_uppercase()` |
| `s.to_lowercase()` | `s.toLowerCase()` | `s.to_lowercase()` |

### 8.4 String Formatting

```rustscript
let msg = format!("Hello, {}!", name);
```

**Compiles to:**

```javascript
// Babel
const msg = `Hello, ${name}!`;
```

```rust
// SWC
let msg = format!("Hello, {}!", name);
```

---

## 9. Option Handling

### 9.1 Creating Options

```rustscript
let some_value = Some(42);
let no_value: Option<i32> = None;
```

### 9.2 Unwrapping

```rustscript
// Safe unwrap with default
let value = opt.unwrap_or(default);
let value = opt.unwrap_or_else(|| compute_default());

// Conditional unwrap
if let Some(v) = opt {
    // use v
}

// Map transformation
let mapped = opt.map(|v| v + 1);

// Chain options
let result = opt.and_then(|v| other_option(v));
```

**Compiles to:**

```javascript
// Babel
const value = opt ?? default;
const value = opt ?? computeDefault();

if (opt != null) {
    const v = opt;
    // use v
}

const mapped = opt != null ? opt + 1 : null;
const result = opt != null ? otherOption(opt) : null;
```

```rust
// SWC: Direct Rust Option methods
let value = opt.unwrap_or(default);
let value = opt.unwrap_or_else(|| compute_default());

if let Some(v) = opt {
    // use v
}

let mapped = opt.map(|v| v + 1);
let result = opt.and_then(|v| other_option(v));
```

---

## 10. Collections

### 10.1 Vec Operations

```rustscript
let mut items: Vec<i32> = vec![];

items.push(1);
items.push(2);

let first = items.get(0);           // Option<&i32>
let len = items.len();
let is_empty = items.is_empty();

for item in &items {
    // iterate
}

let doubled: Vec<i32> = items.iter().map(|x| x * 2).collect();
```

### 10.2 HashMap Operations

```rustscript
let mut map: HashMap<Str, i32> = HashMap::new();

map.insert("key", 42);
let value = map.get("key");         // Option<&i32>
let has_key = map.contains_key("key");

for (key, value) in &map {
    // iterate
}
```

### 10.3 HashSet Operations

```rustscript
let mut set: HashSet<Str> = HashSet::new();

set.insert("item");
let has_item = set.contains("item");
```

---

## 11. Control Flow

### 11.1 If/Else

```rustscript
if condition {
    // then
} else if other_condition {
    // else if
} else {
    // else
}
```

### 11.2 Match

```rustscript
match value {
    Pattern1 => expression1,
    Pattern2 | Pattern3 => expression2,
    _ => default_expression,
}
```

### 11.3 Loops

```rustscript
// For-in loop
for item in collection {
    // body
}

// While loop
while condition {
    // body
}

// Loop with break
loop {
    if done {
        break;
    }
}
```

### 11.4 Early Return

```rustscript
fn process(node: &Node) -> Option<Str> {
    if !node.is_valid() {
        return None;
    }

    // continue processing
    Some(node.name.clone())
}
```

---

## 12. Standard Library (Prelude)

### 12.1 Automatically Imported

```rustscript
// These are always available
Option, Some, None
Vec, vec!
HashMap, HashSet
Str
format!
matches!
```

### 12.2 AST Utilities

```rustscript
/// Check if identifier matches a name
fn is_identifier_named(node: &Identifier, name: &Str) -> bool;

/// Get identifier name safely
fn get_identifier_name(expr: &Expression) -> Option<Str>;

/// Check if node is a specific literal value
fn is_literal_value<T>(node: &Expression, value: T) -> bool;
```

### 12.3 String Utilities

```rustscript
/// Convert to PascalCase
fn to_pascal_case(s: &Str) -> Str;

/// Convert to camelCase
fn to_camel_case(s: &Str) -> Str;

/// Convert to snake_case
fn to_snake_case(s: &Str) -> Str;

/// Escape string for code generation
fn escape_string(s: &Str) -> Str;
```

---

## 13. Error Handling

### 13.1 Result Type

```rustscript
fn parse_value(s: &Str) -> Result<i32, Str> {
    // ...
}
```

### 13.2 The `?` Operator

```rustscript
fn process() -> Result<(), Str> {
    let value = parse_value("42")?;  // Early return on error
    Ok(())
}
```

**Note:** In Babel output, this compiles to explicit error checking with throws.

---

## 14. Compilation Model

### 14.1 File Structure

```
my-plugin/
  src/
    lib.rs          # Main RustScript source
    helpers.rs      # Helper functions
  dist/
    index.js        # Babel output
    plugin.wasm     # SWC WASM output
  Cargo.toml        # Generated for Rust build
  package.json      # Generated for npm
```

### 14.2 Build Command

```bash
rustscript build

# Options
rustscript build --target babel    # JS only
rustscript build --target swc      # Rust/WASM only
rustscript build --target both     # Default: both targets
```

### 14.3 Compilation Phases

1. **Parse**: RustScript source → AST
2. **Type Check**: Validate types and ownership
3. **Lower**: Resolve macros, statement lowering
4. **Emit JS**: Generate Babel plugin
5. **Emit Rust**: Generate SWC plugin
6. **Bundle**: Package for distribution

---

## 15. Example: Complete Plugin

```rustscript
/// Plugin that transforms React hooks for analysis
plugin HookAnalyzer {

    // State tracked across the visitor
    struct State {
        hooks: Vec<HookInfo>,
        current_component: Option<Str>,
    }

    struct HookInfo {
        name: Str,
        hook_type: Str,
        component: Str,
    }

    fn visit_function_declaration(node: &mut FunctionDeclaration, ctx: &Context) {
        // Check if this is a component (PascalCase name)
        let name = node.id.name.clone();
        if is_component_name(&name) {
            self.state.current_component = Some(name);
        }

        // Visit children
        node.visit_children(self);

        // Clear component context
        self.state.current_component = None;
    }

    fn visit_call_expression(node: &mut CallExpression, ctx: &Context) {
        // Check for hook calls
        if let Some(component) = &self.state.current_component {
            if let Some(name) = get_callee_name(&node.callee) {
                if name.starts_with("use") {
                    self.state.hooks.push(HookInfo {
                        name: name.clone(),
                        hook_type: categorize_hook(&name),
                        component: component.clone(),
                    });
                }
            }
        }

        // Visit children
        node.visit_children(self);
    }
}

// Helper functions
fn is_component_name(name: &Str) -> bool {
    if name.is_empty() {
        return false;
    }
    let first_char = name.chars().next().unwrap();
    first_char.is_uppercase()
}

fn get_callee_name(callee: &Expression) -> Option<Str> {
    if let Expression::Identifier(id) = callee {
        Some(id.name.clone())
    } else {
        None
    }
}

fn categorize_hook(name: &Str) -> Str {
    match name.as_str() {
        "useState" | "useReducer" => "state".into(),
        "useEffect" | "useLayoutEffect" => "effect".into(),
        "useRef" => "ref".into(),
        "useMemo" | "useCallback" => "memoization".into(),
        _ => "custom".into(),
    }
}
```

---

## 16. The Writer API (For Transpilers)

This section supports "Read-Only" visitors used for transpilation rather than transformation (e.g., TSX to C#).

### 16.1 Writer Plugin Declaration

```rustscript
writer TsxToCSharp {
    // Internal state
    builder: CodeBuilder,

    // Constructor
    fn init() -> Self {
        Self { builder: CodeBuilder::new() }
    }

    // Read-Only Visitor (Note: `node` is immutable &T, not &mut T)
    fn visit_jsx_element(node: &JSXElement, ctx: &Context) {
        let tag = node.opening_element.name.get_name();

        self.builder.append("new ");
        self.builder.append(tag);
        self.builder.append("({");

        // Recurse manually or use helpers
        self.visit_jsx_attributes(&node.opening_element.attributes);

        self.builder.append("})");
    }

    // Final Output
    fn finish(self) -> Str {
        self.builder.to_string()
    }
}
```

**Compiles to:**

```javascript
// Babel: Wraps the visitor to accumulate a string
module.exports = function() {
  const builder = new CodeBuilder();
  return {
    visitor: {
      JSXElement(path) {
         const node = path.node;
         const tag = node.openingElement.name.name;

         builder.append("new ");
         builder.append(tag);
         builder.append("({");

         // ... attribute handling ...

         builder.append("})");
      }
    },
    post(file) {
       file.metadata.output = builder.toString();
    }
  }
}
```

```rust
// SWC: Uses Visit (read-only) instead of VisitMut
struct TsxToCSharp {
    builder: CodeBuilder,
}

impl TsxToCSharp {
    fn new() -> Self {
        Self { builder: CodeBuilder::new() }
    }

    fn finish(self) -> String {
        self.builder.to_string()
    }
}

impl Visit for TsxToCSharp {
    fn visit_jsx_element(&mut self, n: &JSXElement) {
        let tag = get_jsx_element_name(&n.opening.name);

        self.builder.append("new ");
        self.builder.append(&tag);
        self.builder.append("({");

        // ... attribute handling ...

        self.builder.append("})");
    }
}
```

### 16.2 Writer vs Plugin

| Aspect | `plugin` | `writer` |
|--------|----------|----------|
| Visitor Type | `VisitMut` (mutable) | `Visit` (read-only) |
| Purpose | Transform AST | Generate output |
| Node Access | `&mut T` | `&T` |
| Output | Modified AST | String/CodeBuilder |

### 16.3 Example: React to Orleans Grain

```rustscript
writer ReactToOrleans {
    builder: CodeBuilder,

    fn init() -> Self {
        Self { builder: CodeBuilder::new() }
    }

    fn visit_function_declaration(node: &FunctionDeclaration, ctx: &Context) {
        self.builder.append("public class ");
        self.builder.append(node.id.name.clone());
        self.builder.append(" : Grain, I");
        self.builder.append(node.id.name.clone());
        self.builder.append(" {\n");

        self.builder.indent();
        // Visit children (logic to convert hooks to state fields)
        node.visit_children(self);
        self.builder.dedent();

        self.builder.append("}\n");
    }

    fn visit_call_expression(node: &CallExpression, ctx: &Context) {
        // Check for useState hooks
        if let Some(name) = get_callee_name(&node.callee) {
            if name == "useState" {
                self.emit_state_field(node);
                return;
            }
        }

        node.visit_children(self);
    }

    fn emit_state_field(&mut self, node: &CallExpression) {
        // Extract state name from parent (const [x, setX] = useState(...))
        // This would use ctx or tracked state
        self.builder.append("private ");
        self.builder.append("dynamic"); // infer type
        self.builder.append(" _state;\n");
    }

    fn finish(self) -> Str {
        self.builder.to_string()
    }
}
```

---

## 17. Context & Scoping

### 17.1 The Scope Cost

Because SWC does not track scope by default, accessing `ctx.scope` triggers a pre-pass analysis in the SWC target.

```rustscript
fn visit_identifier(node: &mut Identifier, ctx: &Context) {
    // WARNING: This call is cheap in Babel but expensive in SWC (requires O(n) pre-pass)
    if ctx.scope.has_binding(&node.name) {
        // ...
    }
}
```

### 17.2 Scope Methods

| Method | Description | Babel Cost | SWC Cost |
|--------|-------------|------------|----------|
| `has_binding(name)` | Check if name is bound in scope | O(1) | O(n) pre-pass |
| `generate_uid(hint)` | Generate unique identifier | O(1) | O(n) tracking |
| `get_binding(name)` | Get binding info | O(1) | O(n) pre-pass |

### 17.3 Avoiding Scope Lookups

For performance in SWC, track bindings manually when possible:

```rustscript
plugin OptimizedVisitor {
    // Track bindings ourselves
    bindings: HashSet<Str>,

    fn visit_variable_declarator(node: &mut VariableDeclarator, ctx: &Context) {
        if let Pattern::Identifier(id) = &node.id {
            self.bindings.insert(id.name.clone());
        }
        node.visit_children(self);
    }

    fn visit_identifier(node: &mut Identifier, ctx: &Context) {
        // Use our tracked bindings instead of ctx.scope
        if self.bindings.contains(&node.name) {
            // ...
        }
    }
}
```

---

## 18. Limitations

### 18.1 Not Supported

- Async/await (different semantics between targets)
- External library imports (no cross-platform guarantee)
- Direct DOM/Node.js APIs
- Regex literals (use string-based matching instead)
- Closures that capture mutable state (borrow checker issues)

### 16.2 Platform-Specific Escape Hatches

For cases requiring platform-specific code:

```rustscript
#[cfg(target = "babel")]
fn babel_specific() {
    // Only compiled to Babel
}

#[cfg(target = "swc")]
fn swc_specific() {
    // Only compiled to SWC
}
```

**Warning:** Use sparingly. This breaks the "write once" guarantee.

---

## 17. Future Considerations

### 17.1 Planned Features

- Module system for code organization
- Generic functions
- Derive macros for common patterns
- LSP support for editor integration
- Source maps for debugging

### 17.2 Ecosystem

- Package registry for shared RustScript libraries
- Testing framework with dual-target test runner
- Benchmark suite comparing Babel vs SWC performance

---

## Appendix A: Grammar (EBNF)

```ebnf
program         = plugin_decl ;
plugin_decl     = "plugin" IDENT "{" plugin_body "}" ;
plugin_body     = { struct_decl | fn_decl | impl_block } ;

struct_decl     = "struct" IDENT "{" struct_fields "}" ;
struct_fields   = { IDENT ":" type "," } ;

fn_decl         = ["pub"] "fn" IDENT "(" params ")" ["->" type] block ;
params          = [ param { "," param } ] ;
param           = IDENT ":" type ;

type            = primitive_type | reference_type | container_type | IDENT ;
primitive_type  = "Str" | "i32" | "f64" | "bool" | "()" ;
reference_type  = "&" ["mut"] type ;
container_type  = ("Vec" | "Option" | "HashMap" | "HashSet") "<" type_args ">" ;
type_args       = type { "," type } ;

block           = "{" { statement } "}" ;
statement       = let_stmt | expr_stmt | if_stmt | match_stmt | for_stmt
                | while_stmt | return_stmt | break_stmt | continue_stmt ;

let_stmt        = "let" ["mut"] IDENT [":" type] "=" expr ";" ;
expr_stmt       = expr ";" ;
if_stmt         = "if" expr block { "else" "if" expr block } [ "else" block ] ;
match_stmt      = "match" expr "{" { match_arm } "}" ;
match_arm       = pattern "=>" expr "," ;
for_stmt        = "for" IDENT "in" expr block ;
while_stmt      = "while" expr block ;
return_stmt     = "return" [expr] ";" ;

expr            = assignment | logical_or ;
assignment      = (deref | member) "=" expr | logical_or ;
logical_or      = logical_and { "||" logical_and } ;
logical_and     = equality { "&&" equality } ;
equality        = comparison { ("==" | "!=") comparison } ;
comparison      = term { ("<" | ">" | "<=" | ">=") term } ;
term            = factor { ("+" | "-") factor } ;
factor          = unary { ("*" | "/" | "%") unary } ;
unary           = ("!" | "-" | "*" | "&" ["mut"]) unary | call ;
call            = primary { "(" args ")" | "." IDENT | "[" expr "]" } ;
primary         = IDENT | literal | "(" expr ")" | struct_init | vec_init ;

literal         = STRING | NUMBER | "true" | "false" | "null" ;
struct_init     = IDENT "{" { IDENT ":" expr "," } "}" ;
vec_init        = "vec!" "[" [ expr { "," expr } ] "]" ;
args            = [ expr { "," expr } ] ;
pattern         = literal | IDENT | "_" | struct_pattern ;
deref           = "*" IDENT ;
member          = expr "." IDENT ;
```

---

## Appendix B: Compilation Examples

### B.1 Simple Function

**RustScript:**
```rustscript
fn is_hook_name(name: &Str) -> bool {
    name.starts_with("use") && name.len() > 3
}
```

**Babel Output:**
```javascript
function isHookName(name) {
    return name.startsWith("use") && name.length > 3;
}
```

**SWC Output:**
```rust
fn is_hook_name(name: &str) -> bool {
    name.starts_with("use") && name.len() > 3
}
```

### B.2 Pattern Matching

**RustScript:**
```rustscript
fn get_string_value(node: &Expression) -> Option<Str> {
    if let Expression::StringLiteral(lit) = node {
        Some(lit.value.clone())
    } else {
        None
    }
}
```

**Babel Output:**
```javascript
function getStringValue(node) {
    if (t.isStringLiteral(node)) {
        return node.value;
    } else {
        return null;
    }
}
```

**SWC Output:**
```rust
fn get_string_value(node: &Expr) -> Option<String> {
    if let Expr::Lit(Lit::Str(lit)) = node {
        Some(lit.value.to_string())
    } else {
        None
    }
}
```

---

## Appendix C: Error Messages

### C.1 Ownership Errors

```
error[RS001]: implicit borrow not allowed
  --> src/lib.rs:10:15
   |
10 |     let name = node.name;
   |               ^^^^^^^^^^ help: use explicit clone: `node.name.clone()`
   |
   = note: RustScript requires explicit .clone() to extract values from references
```

### C.2 Mutation Errors

```
error[RS002]: direct property mutation not allowed
  --> src/lib.rs:15:5
   |
15 |     node.name = "new";
   |     ^^^^^^^^^^^^^^^^^
   |
   = note: replace the entire node instead of mutating properties
   = help: use `*node = NodeType { ... }` pattern
```

### C.3 Type Errors

```
error[RS003]: type mismatch
  --> src/lib.rs:20:20
   |
20 |     let count: Str = 42;
   |                      ^^ expected `Str`, found `i32`
```

---

*End of Specification*
