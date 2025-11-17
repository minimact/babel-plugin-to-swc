# Babel Plugin Patterns

This document describes the core patterns used in the simplified Babel plugin that should be transpiled to SWC/Rust equivalents.

## 1. Plugin Structure

### Babel Pattern
```javascript
module.exports = function(babel) {
  return {
    name: 'plugin-name',
    visitor: { /* visitors */ }
  };
};
```

### SWC Equivalent (Target)
```rust
use swc_core::ecma::{
    ast::*,
    visit::{VisitMut, VisitMutWith},
};

pub struct TransformVisitor;

impl VisitMut for TransformVisitor {
    // Visit methods here
}
```

## 2. Visitor Pattern

### Babel: Program Visitor with Enter/Exit
```javascript
Program: {
  enter(path, state) {
    state.components = [];
  },
  exit(path, state) {
    console.log('Done:', state.components);
  }
}
```

### SWC Equivalent
```rust
impl VisitMut for TransformVisitor {
    fn visit_mut_module(&mut self, module: &mut Module) {
        // Enter logic here
        self.components = Vec::new();

        // Visit children
        module.visit_mut_children_with(self);

        // Exit logic here
        println!("Done: {:?}", self.components);
    }
}
```

## 3. Specific Node Visitors

### Babel: FunctionDeclaration
```javascript
FunctionDeclaration(path, state) {
  const name = path.node.id.name;
  processComponent(path, state, name);
}
```

### SWC Equivalent
```rust
fn visit_mut_fn_decl(&mut self, func: &mut FnDecl) {
    let name = func.ident.sym.to_string();
    self.process_component(&name, &func.function);

    func.visit_mut_children_with(self);
}
```

### Babel: ArrowFunctionExpression
```javascript
ArrowFunctionExpression(path, state) {
  if (path.parent.type === 'VariableDeclarator') {
    const name = path.parent.id.name;
    processComponent(path, state, name);
  }
}
```

### SWC Equivalent
```rust
fn visit_mut_var_declarator(&mut self, var: &mut VarDeclarator) {
    if let Pat::Ident(ident) = &var.name {
        let name = ident.id.sym.to_string();

        if let Some(init) = &var.init {
            if let Expr::Arrow(arrow) = &**init {
                // Check if component (starts with uppercase)
                if name.chars().next().unwrap().is_uppercase() {
                    self.process_component(&name, arrow);
                }
            }
        }
    }

    var.visit_mut_children_with(self);
}
```

## 4. Nested Traversal

### Babel: path.traverse()
```javascript
path.traverse({
  CallExpression(callPath) {
    if (t.isIdentifier(callPath.node.callee, { name: 'useState' })) {
      // Extract hook
    }
  },
  JSXElement(jsxPath) {
    // Track JSX element
  }
});
```

### SWC Equivalent
```rust
struct HookExtractor {
    hooks: Vec<Hook>,
}

impl Visit for HookExtractor {
    fn visit_call_expr(&mut self, call: &CallExpr) {
        if let Callee::Expr(expr) = &call.callee {
            if let Expr::Ident(ident) = &**expr {
                if &*ident.sym == "useState" {
                    // Extract hook
                }
            }
        }
        call.visit_children_with(self);
    }

    fn visit_jsx_element(&mut self, jsx: &JSXElement) {
        // Track JSX element
        jsx.visit_children_with(self);
    }
}
```

## 5. Type Checking

### Babel: Using @babel/types
```javascript
const t = require('@babel/types');

if (t.isIdentifier(node.callee, { name: 'useState' })) {
  // ...
}

if (t.isArrayPattern(parent.id)) {
  // ...
}
```

### SWC Equivalent
```rust
// Pattern matching in Rust
match &call.callee {
    Callee::Expr(expr) => {
        if let Expr::Ident(ident) = &**expr {
            if &*ident.sym == "useState" {
                // ...
            }
        }
    }
    _ => {}
}

if let Pat::Array(array_pat) = &parent {
    // ...
}
```

## 6. State Management

### Babel: Using state parameter
```javascript
visitor: {
  Program: {
    enter(path, state) {
      state.components = [];
    }
  },
  FunctionDeclaration(path, state) {
    state.components.push(/* ... */);
  }
}
```

### SWC Equivalent
```rust
pub struct TransformVisitor {
    components: Vec<Component>,
}

impl TransformVisitor {
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
        }
    }
}

impl VisitMut for TransformVisitor {
    fn visit_mut_fn_decl(&mut self, func: &mut FnDecl) {
        self.components.push(/* ... */);
        func.visit_mut_children_with(self);
    }
}
```

## 7. Data Extraction Patterns

### Babel: Extract useState
```javascript
function extractUseState(callPath) {
  const parent = callPath.parent;

  if (t.isVariableDeclarator(parent) && t.isArrayPattern(parent.id)) {
    const [stateId, setStateId] = parent.id.elements;
    const initialValue = callPath.node.arguments[0];

    return {
      stateName: stateId.name,
      setterName: setStateId.name,
      initialValue: initialValue
    };
  }
}
```

### SWC Equivalent
```rust
fn extract_use_state(&self, call: &CallExpr, parent: &VarDeclarator) -> Option<UseStateHook> {
    if let Pat::Array(array_pat) = &parent.name {
        let state_name = match &array_pat.elems[0] {
            Some(Pat::Ident(id)) => id.id.sym.to_string(),
            _ => return None,
        };

        let setter_name = match &array_pat.elems[1] {
            Some(Pat::Ident(id)) => id.id.sym.to_string(),
            _ => return None,
        };

        let initial_value = call.args.get(0)?;

        Some(UseStateHook {
            state_name,
            setter_name,
            initial_value: initial_value.expr.clone(),
        })
    } else {
        None
    }
}
```

## Key Differences

| Aspect | Babel | SWC |
|--------|-------|-----|
| **Language** | JavaScript | Rust |
| **Type System** | Dynamic, uses runtime checks | Static, uses pattern matching |
| **Visitor Trait** | Plain object with methods | Trait implementation (VisitMut/Visit) |
| **State** | Passed as parameter | Stored in struct fields |
| **Type Checking** | `t.isXXX()` functions | Pattern matching with `match`/`if let` |
| **Traversal** | `path.traverse()` | Separate visitor struct + trait |
| **Children** | Automatic | Must call `visit_mut_children_with()` |
