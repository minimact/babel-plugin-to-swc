# Babel Plugin to RustScript Conversion Guide

A practical guide for manually converting babel-plugin-minimact JavaScript code to RustScript.

## Table of Contents

1. [Basic Structure](#basic-structure)
2. [Type Conversions](#type-conversions)
3. [Node Access Patterns](#node-access-patterns)
4. [Visitor Methods](#visitor-methods)
5. [Pattern Matching](#pattern-matching)
6. [Collections & Iteration](#collections--iteration)
7. [String Operations](#string-operations)
8. [State Management](#state-management)
9. [Helper Functions](#helper-functions)
10. [File I/O](#file-io)
11. [Common Patterns Reference](#common-patterns-reference)

---

## Basic Structure

### Module Exports/Imports

**Babel (JavaScript):**
```javascript
// helpers.cjs
function escapeCSharpString(str) {
  return str.replace(/\\/g, '\\\\');
}

function getComponentName(path) {
  return path.node.id ? path.node.id.name : null;
}

module.exports = {
  escapeCSharpString,
  getComponentName,
};
```

**RustScript:**
```rustscript
// helpers.rsc
pub fn escape_csharp_string(s: &Str) -> Str {
    s.replace("\\", "\\\\")
}

pub fn get_component_name(node: &FunctionDeclaration) -> Option<Str> {
    if let Some(ref id) = node.id {
        return Some(id.name.clone());
    }
    None
}
```

**Key Changes:**
- `function` → `pub fn` (for exports)
- `module.exports = {}` → just use `pub fn`
- camelCase → snake_case for function names
- `null` → `None` (use `Option<T>`)
- Add explicit types to parameters and return values

### Using Modules

**Babel (JavaScript):**
```javascript
const { getComponentName } = require('./utils/helpers.cjs');
const { tsTypeToCSharpType } = require('./types/typeConversion.cjs');

// Use them
const name = getComponentName(path);
const csharpType = tsTypeToCSharpType(typeAnnotation);
```

**RustScript:**
```rustscript
use "./utils/helpers.rsc" { get_component_name };
use "./types/typeConversion.rsc" { ts_type_to_csharp_type };

// Use them
let name = get_component_name(node);
let csharp_type = ts_type_to_csharp_type(type_annotation);
```

**Key Changes:**
- `require()` → `use` statement
- Destructuring imports work the same way
- Path must include `.rsc` extension

---

## Type Conversions

### JavaScript → RustScript Types

| JavaScript | RustScript | Notes |
|------------|------------|-------|
| `string` | `Str` | RustScript's platform-agnostic string |
| `number` | `i32` or `f64` | Use `i32` for integers, `f64` for floats |
| `boolean` | `bool` | Same keyword |
| `null`, `undefined` | `None` | Use `Option<T>` for nullable values |
| `Array<T>` | `Vec<T>` | Dynamic array |
| `Object` or `Map` | `HashMap<K, V>` | Key-value pairs |
| `Set` | `HashSet<T>` | Unique values |
| `{ key: value }` | `struct` | Define custom structs |

### Variable Declarations

**Babel (JavaScript):**
```javascript
const name = "Component";           // Immutable
let count = 0;                      // Mutable
const items = [];                   // Immutable reference, mutable contents
items.push("item");

const result = computeValue();      // Can be null
if (result !== null) {
  console.log(result);
}
```

**RustScript:**
```rustscript
let name = "Component";             // Immutable
let mut count = 0;                  // Mutable
let mut items = vec![];             // Mutable
items.push("item");

let result = compute_value();       // Option<T>
if let Some(value) = result {
    // use value
}
```

**Key Changes:**
- `const` → `let` (immutable by default)
- `let` (mutable) → `let mut`
- `null` checks → `if let Some(...)` pattern

---

## Node Access Patterns

### Accessing Node Properties

**Babel (JavaScript):**
```javascript
// Direct property access
const name = node.name;
const id = node.id;
const params = node.params;

// Nested access
const calleeName = node.callee.name;

// Optional chaining
const typeName = node.typeAnnotation?.typeAnnotation?.typeName?.name;
```

**RustScript:**
```rustscript
// Direct property access - must clone owned values
let name = node.name.clone();

// Borrowing references (for temporary use)
let id = &node.id;
let params = &node.params;

// Nested access with Option unwrapping
let callee_name = if let Some(ref callee) = node.callee {
    if let Identifier(ref id) = callee {
        Some(id.name.clone())
    } else {
        None
    }
} else {
    None
};

// Optional chaining - use nested if-let
if let Some(ref type_ann) = node.type_annotation {
    if let Some(ref inner) = type_ann.type_annotation {
        if let Some(ref type_name) = inner.type_name {
            let name = type_name.name.clone();
        }
    }
}
```

**Key Changes:**
- Must `.clone()` to own values (RustScript requires explicit cloning)
- Use `&` for borrowing references when you don't need ownership
- Optional chaining → nested `if let Some(ref ...)` patterns
- Use `ref` in patterns to borrow instead of move

### Checking Node Types

**Babel (JavaScript):**
```javascript
const t = require('@babel/types');

if (t.isIdentifier(node)) {
  console.log(node.name);
}

if (t.isCallExpression(node.init)) {
  const callee = node.init.callee;
}

if (t.isMemberExpression(node) &&
    t.isIdentifier(node.object, { name: "console" })) {
  // ...
}
```

**RustScript:**
```rustscript
// Simple type check
if matches!(node, Identifier) {
    let name = node.name.clone();
}

// Check nested type
if let Some(ref init) = node.init {
    if matches!(init, CallExpression) {
        let callee = &init.callee;
    }
}

// Pattern matching with field checks
if let Expression::MemberExpression(ref member) = node {
    if let Expression::Identifier(ref obj) = member.object {
        if obj.name == "console" {
            // ...
        }
    }
}
```

**Key Changes:**
- `t.isIdentifier()` → `matches!(node, Identifier)`
- `t.isCallExpression()` → `matches!(node, CallExpression)`
- Nested checks require pattern matching with `if let`
- Field value checks done after extracting with pattern match

---

## Visitor Methods

### Basic Visitor

**Babel (JavaScript):**
```javascript
module.exports = function(babel) {
  const t = babel.types;

  return {
    visitor: {
      FunctionDeclaration(path) {
        const name = path.node.id.name;
        console.log(`Found function: ${name}`);
      },

      CallExpression(path) {
        if (t.isIdentifier(path.node.callee, { name: 'useState' })) {
          // Handle useState
        }
      }
    }
  };
};
```

**RustScript:**
```rustscript
plugin MinimactPlugin {
    fn visit_function_declaration(node: &mut FunctionDeclaration, ctx: &Context) {
        if let Some(ref id) = node.id {
            let name = id.name.clone();
            // Found function: {name}
        }
    }

    fn visit_call_expression(node: &mut CallExpression, ctx: &Context) {
        if let Expression::Identifier(ref callee) = node.callee {
            if callee.name == "useState" {
                // Handle useState
            }
        }
    }
}
```

**Key Changes:**
- `visitor: { ... }` → `plugin PluginName { ... }`
- `MethodName(path)` → `fn visit_method_name(node: &mut NodeType, ctx: &Context)`
- `path.node` → `node` (direct access)
- PascalCase → snake_case for method names
- No `babel.types` needed - types are built-in

### Visitor with State

**Babel (JavaScript):**
```javascript
module.exports = function() {
  return {
    visitor: {
      Program: {
        enter(path, state) {
          state.components = [];
          state.hooks = [];
        },

        exit(path, state) {
          console.log(`Found ${state.components.length} components`);
        }
      },

      FunctionDeclaration(path, state) {
        const name = path.node.id.name;
        state.components.push({ name });
      }
    }
  };
};
```

**RustScript:**
```rustscript
plugin MinimactPlugin {
    struct State {
        components: Vec<ComponentInfo>,
        hooks: Vec<HookInfo>,
    }

    struct ComponentInfo {
        name: Str,
    }

    struct HookInfo {
        name: Str,
    }

    fn visit_program_enter(node: &mut Program, ctx: &Context) {
        self.state.components = vec![];
        self.state.hooks = vec![];
    }

    fn visit_program_exit(node: &mut Program, ctx: &Context) {
        let count = self.state.components.len();
        // Found {count} components
    }

    fn visit_function_declaration(node: &mut FunctionDeclaration, ctx: &Context) {
        if let Some(ref id) = node.id {
            let name = id.name.clone();
            self.state.components.push(ComponentInfo { name });
        }
    }
}
```

**Key Changes:**
- State stored in `struct State` inside plugin
- Access state with `self.state.field_name`
- Program entry/exit → `visit_program_enter` / `visit_program_exit`
- Define custom structs for complex data

---

## Pattern Matching

### Destructuring Arrays

**Babel (JavaScript):**
```javascript
// const [count, setCount] = useState(0);
if (t.isArrayPattern(decl.id)) {
  const elements = decl.id.elements;
  const [valueId, setterId] = elements;

  if (valueId && setterId) {
    const valueName = valueId.name;
    const setterName = setterId.name;
  }
}
```

**RustScript:**
```rustscript
// const [count, setCount] = useState(0);
if let Pattern::ArrayPat(ref arr) = decl.id {
    if arr.elements.len() >= 2 {
        if let Some(ref value_elem) = arr.elements[0] {
            if let Pattern::Ident(ref value_name) = value_elem {
                // Got value name
            }
        }

        if let Some(ref setter_elem) = arr.elements[1] {
            if let Pattern::Ident(ref setter_name) = setter_elem {
                // Got setter name
            }
        }
    }
}
```

**Key Changes:**
- Array destructuring needs index access
- Each element is `Option<Pattern>` so needs unwrapping
- Extract identifier name with nested pattern match

### Destructuring Objects

**Babel (JavaScript):**
```javascript
// function Component({ name, age }) { }
if (t.isObjectPattern(params[0])) {
  for (const prop of params[0].properties) {
    if (t.isObjectProperty(prop)) {
      const propName = prop.key.name;
      const propType = prop.typeAnnotation?.typeAnnotation;
    }
  }
}
```

**RustScript:**
```rustscript
// function Component({ name, age }) { }
if let Some(ref first_param) = params.get(0) {
    if let Pattern::ObjectPat(ref obj) = first_param {
        for prop in &obj.properties {
            if let ObjectPatternProp::KeyValue(ref kv) = prop {
                if let Pattern::Ident(ref prop_name) = kv.key {
                    // Got prop name

                    if let Some(ref type_ann) = kv.type_annotation {
                        // Got type annotation
                    }
                }
            }
        }
    }
}
```

**Key Changes:**
- Object properties need iteration
- Each property is an enum variant
- Extract key with pattern matching

---

## Collections & Iteration

### Arrays/Vectors

**Babel (JavaScript):**
```javascript
const items = [];
items.push("item1");
items.push("item2");

const count = items.length;
const first = items[0];

for (const item of items) {
  console.log(item);
}

const filtered = items.filter(item => item.startsWith("item"));
const mapped = items.map(item => item.toUpperCase());
```

**RustScript:**
```rustscript
let mut items = vec![];
items.push("item1");
items.push("item2");

let count = items.len();
let first = &items[0];

for item in &items {
    // use item
}

let filtered: Vec<Str> = items.iter()
    .filter(|item| item.starts_with("item"))
    .map(|s| s.clone())
    .collect();

let mapped: Vec<Str> = items.iter()
    .map(|item| item.to_uppercase())
    .collect();
```

**Key Changes:**
- `[]` → `vec![]`
- `.length` → `.len()`
- `for...of` → `for item in &items`
- `.filter()/.map()` → `.iter().filter().map().collect()`
- Must call `.collect()` to materialize results

### Objects/HashMaps

**Babel (JavaScript):**
```javascript
const map = {};
map["key1"] = "value1";
map["key2"] = "value2";

const hasKey = "key1" in map;
const value = map["key1"];

for (const key in map) {
  console.log(key, map[key]);
}
```

**RustScript:**
```rustscript
let mut map = HashMap::new();
map.insert("key1", "value1");
map.insert("key2", "value2");

let has_key = map.contains_key("key1");
let value = map.get("key1");  // Returns Option<&V>

for (key, value) in &map {
    // use key and value
}
```

**Key Changes:**
- `{}` → `HashMap::new()`
- `obj[key]` → `map.get(key)` (returns `Option`)
- `"key" in obj` → `map.contains_key("key")`
- Iteration gives tuples `(key, value)`

### Sets

**Babel (JavaScript):**
```javascript
const set = new Set();
set.add("item1");
set.add("item2");

const hasItem = set.has("item1");
const size = set.size;

for (const item of set) {
  console.log(item);
}
```

**RustScript:**
```rustscript
let mut set = HashSet::new();
set.insert("item1");
set.insert("item2");

let has_item = set.contains("item1");
let size = set.len();

for item in &set {
    // use item
}
```

**Key Changes:**
- `new Set()` → `HashSet::new()`
- `.add()` → `.insert()`
- `.has()` → `.contains()`
- `.size` → `.len()`

---

## String Operations

### String Building

**Babel (JavaScript):**
```javascript
let code = "";
code += "public class ";
code += componentName;
code += " {\n";

// Template literals
const msg = `Hello, ${name}!`;
const csharp = `public ${type} ${name} { get; set; }`;
```

**RustScript:**
```rustscript
let mut code = String::new();
code.push_str("public class ");
code.push_str(&component_name);
code.push_str(" {\n");

// format! macro
let msg = format!("Hello, {}!", name);
let csharp = format!("public {} {} {{ get; set; }}", type_name, name);
```

**Key Changes:**
- String concatenation → `.push_str()`
- Template literals → `format!()` macro
- `${}` → `{}`
- Must use `&` to borrow strings when concatenating

### String Methods

**Babel (JavaScript):**
```javascript
const name = "useState";

if (name.startsWith("use")) { }
if (name.endsWith("State")) { }
if (name.includes("State")) { }

const upper = name.toUpperCase();
const lower = name.toLowerCase();

const replaced = name.replace("State", "Effect");
const parts = name.split("_");
const joined = parts.join("-");
```

**RustScript:**
```rustscript
let name = "useState";

if name.starts_with("use") { }
if name.ends_with("State") { }
if name.contains("State") { }

let upper = name.to_uppercase();
let lower = name.to_lowercase();

let replaced = name.replace("State", "Effect");
let parts: Vec<&str> = name.split("_").collect();
let joined = parts.join("-");
```

**Key Changes:**
- `.startsWith()` → `.starts_with()`
- `.endsWith()` → `.ends_with()`
- `.includes()` → `.contains()`
- `.toUpperCase()` → `.to_uppercase()`
- `.toLowerCase()` → `.to_lowercase()`
- `.split()` needs `.collect()` to get Vec

### String Escaping

**Babel (JavaScript):**
```javascript
function escapeCSharpString(str) {
  return str
    .replace(/\\/g, '\\\\')
    .replace(/"/g, '\\"')
    .replace(/\n/g, '\\n');
}
```

**RustScript:**
```rustscript
pub fn escape_csharp_string(s: &Str) -> Str {
    s.replace("\\", "\\\\")
     .replace("\"", "\\\"")
     .replace("\n", "\\n")
}
```

**Key Changes:**
- No regex syntax - use string literals
- Method chaining works the same

---

## State Management

### Tracking State Across Visits

**Babel (JavaScript):**
```javascript
module.exports = function() {
  return {
    visitor: {
      Program(path, state) {
        state.file.components = [];
        state.file.currentComponent = null;
      },

      FunctionDeclaration(path, state) {
        const name = path.node.id.name;
        state.file.currentComponent = { name, hooks: [] };
      },

      CallExpression(path, state) {
        if (state.file.currentComponent) {
          const hook = extractHook(path.node);
          state.file.currentComponent.hooks.push(hook);
        }
      },

      'FunctionDeclaration:exit'(path, state) {
        if (state.file.currentComponent) {
          state.file.components.push(state.file.currentComponent);
          state.file.currentComponent = null;
        }
      }
    }
  };
};
```

**RustScript:**
```rustscript
plugin MinimactPlugin {
    struct State {
        components: Vec<ComponentInfo>,
        current_component: Option<ComponentInfo>,
    }

    struct ComponentInfo {
        name: Str,
        hooks: Vec<HookInfo>,
    }

    struct HookInfo {
        name: Str,
    }

    fn visit_program_enter(node: &mut Program, ctx: &Context) {
        self.state.components = vec![];
        self.state.current_component = None;
    }

    fn visit_function_declaration(node: &mut FunctionDeclaration, ctx: &Context) {
        if let Some(ref id) = node.id {
            let name = id.name.clone();
            self.state.current_component = Some(ComponentInfo {
                name,
                hooks: vec![],
            });
        }

        // Continue visiting children
        node.visit_children(self);
    }

    fn visit_call_expression(node: &mut CallExpression, ctx: &Context) {
        if let Some(ref mut component) = self.state.current_component {
            let hook = extract_hook(node);
            component.hooks.push(hook);
        }
    }

    fn visit_function_declaration_exit(node: &mut FunctionDeclaration, ctx: &Context) {
        if let Some(component) = self.state.current_component.take() {
            self.state.components.push(component);
        }
    }
}
```

**Key Changes:**
- State defined in `struct State { ... }`
- Access with `self.state.field`
- Exit methods: `visit_method_name_exit`
- Use `Option::take()` to move out of Option

---

## Helper Functions

### Simple Helpers

**Babel (JavaScript):**
```javascript
function isComponentName(name) {
  if (!name) return false;
  return name[0] === name[0].toUpperCase();
}

function getHookType(hookName) {
  if (hookName === 'useState') return 'state';
  if (hookName === 'useEffect') return 'effect';
  return 'custom';
}
```

**RustScript:**
```rustscript
pub fn is_component_name(name: &Str) -> bool {
    if name.is_empty() {
        return false;
    }

    if let Some(first) = name.chars().next() {
        return first.is_uppercase();
    }

    false
}

pub fn get_hook_type(hook_name: &Str) -> Str {
    if hook_name == "useState" {
        return "state";
    }
    if hook_name == "useEffect" {
        return "effect";
    }
    "custom"
}
```

**Key Changes:**
- Check empty strings with `.is_empty()`
- Get first char with `.chars().next()`
- Return early or use last expression as return value

### Helpers with Complex Logic

**Babel (JavaScript):**
```javascript
function extractStateVariables(node) {
  const result = [];

  if (!t.isArrayPattern(node.id)) {
    return result;
  }

  const [valueId, setterId] = node.id.elements;
  if (!valueId || !setterId) {
    return result;
  }

  result.push({
    varName: valueId.name,
    setterName: setterId.name,
  });

  return result;
}
```

**RustScript:**
```rustscript
pub struct StateVariable {
    pub var_name: Str,
    pub setter_name: Str,
}

pub fn extract_state_variables(node: &VariableDeclarator) -> Vec<StateVariable> {
    let mut result = vec![];

    if let Pattern::ArrayPat(ref arr) = node.id {
        if arr.elements.len() >= 2 {
            let value_name = if let Some(ref elem) = arr.elements[0] {
                if let Pattern::Ident(ref name) = elem {
                    Some(name.clone())
                } else {
                    None
                }
            } else {
                None
            };

            let setter_name = if let Some(ref elem) = arr.elements[1] {
                if let Pattern::Ident(ref name) = elem {
                    Some(name.clone())
                } else {
                    None
                }
            } else {
                None
            };

            if let (Some(var_name), Some(setter)) = (value_name, setter_name) {
                result.push(StateVariable {
                    var_name,
                    setter_name: setter,
                });
            }
        }
    }

    result
}
```

**Key Changes:**
- Define struct for return type
- Use nested `if let` for unwrapping
- Build result incrementally
- Return empty vec if early return needed

---

## File I/O

### Writing Files

**Babel (JavaScript):**
```javascript
const fs = require('fs');
const path = require('path');

// Write C# file
const csFilePath = path.join(outputDir, `${componentName}.cs`);
fs.writeFileSync(csFilePath, csharpCode);

// Write JSON file
const jsonFilePath = path.join(outputDir, `${componentName}.templates.json`);
fs.writeFileSync(jsonFilePath, JSON.stringify(templates, null, 2));
```

**RustScript:**
```rustscript
use fs;
use json;

// Write C# file
let cs_file_path = format!("{}/{}.cs", output_dir, component_name);
fs::write_file(&cs_file_path, &csharp_code)?;

// Write JSON file
let json_file_path = format!("{}/{}.templates.json", output_dir, component_name);
let json_string = json::stringify(templates);
fs::write_file(&json_file_path, &json_string)?;
```

**Key Changes:**
- `require('fs')` → `use fs;`
- `fs.writeFileSync()` → `fs::write_file()`
- `path.join()` → `format!()` with path separator
- `JSON.stringify()` → `json::stringify()`
- Use `?` operator for error handling

### Reading Files

**Babel (JavaScript):**
```javascript
const fs = require('fs');

if (fs.existsSync(filePath)) {
  const content = fs.readFileSync(filePath, 'utf-8');
  const lines = content.split('\n');
}
```

**RustScript:**
```rustscript
use fs;

if fs::file_exists(&file_path) {
    if let Ok(content) = fs::read_file(&file_path) {
        let lines: Vec<&str> = content.split('\n').collect();
    }
}
```

**Key Changes:**
- `fs.existsSync()` → `fs::file_exists()`
- `fs.readFileSync()` → `fs::read_file()` (returns `Result`)
- Use `if let Ok(...)` to handle Result

---

## Common Patterns Reference

### Pattern 1: Extract Hook Call

**Babel:**
```javascript
function extractUseState(path) {
  if (!t.isCallExpression(path.node.init)) return null;

  const callee = path.node.init.callee;
  if (!t.isIdentifier(callee, { name: 'useState' })) return null;

  const [valueId, setterId] = path.node.id.elements;
  const initialValue = path.node.init.arguments[0];

  return {
    varName: valueId.name,
    setterName: setterId.name,
    initialValue: generate(initialValue).code,
  };
}
```

**RustScript:**
```rustscript
pub struct UseStateInfo {
    pub var_name: Str,
    pub setter_name: Str,
    pub initial_value: Str,
}

pub fn extract_use_state(decl: &VariableDeclarator) -> Option<UseStateInfo> {
    // Check if init is a call expression
    let init = decl.init.as_ref()?;
    if !matches!(init, Expression::CallExpression) {
        return None;
    }

    let call = if let Expression::CallExpression(ref c) = init {
        c
    } else {
        return None;
    };

    // Check if callee is "useState"
    if let Expression::Identifier(ref id) = call.callee {
        if id.name != "useState" {
            return None;
        }
    } else {
        return None;
    }

    // Extract array pattern
    if let Pattern::ArrayPat(ref arr) = decl.id {
        if arr.elements.len() < 2 {
            return None;
        }

        let var_name = if let Some(Pattern::Ident(ref name)) = arr.elements[0] {
            name.clone()
        } else {
            return None;
        };

        let setter_name = if let Some(Pattern::Ident(ref name)) = arr.elements[1] {
            name.clone()
        } else {
            return None;
        };

        let initial_value = if !call.arguments.is_empty() {
            generate_expression(&call.arguments[0])
        } else {
            "undefined"
        };

        return Some(UseStateInfo {
            var_name,
            setter_name,
            initial_value,
        });
    }

    None
}
```

### Pattern 2: Build Member Expression Path

**Babel:**
```javascript
function buildMemberPath(object, path) {
  const parts = path.split('.');
  let current = t.identifier(object);

  for (const part of parts) {
    current = t.memberExpression(current, t.identifier(part));
  }

  return current;
}
```

**RustScript:**
```rustscript
pub fn build_member_path(object: &Str, path: &Str) -> Expression {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = Expression::Identifier(Identifier {
        name: object.clone(),
        span: DUMMY_SP,
    });

    for part in parts {
        current = Expression::MemberExpression(MemberExpression {
            object: Box::new(current),
            property: Box::new(Expression::Identifier(Identifier {
                name: part.to_string(),
                span: DUMMY_SP,
            })),
            computed: false,
            span: DUMMY_SP,
        });
    }

    current
}
```

### Pattern 3: Type Conversion Map

**Babel:**
```javascript
function tsTypeToCSharpType(tsType) {
  const typeMap = {
    'string': 'string',
    'number': 'double',
    'boolean': 'bool',
    'any': 'dynamic',
  };

  if (tsType.type === 'TSStringKeyword') {
    return typeMap['string'];
  }
  if (tsType.type === 'TSNumberKeyword') {
    return typeMap['number'];
  }

  return 'dynamic';
}
```

**RustScript:**
```rustscript
pub fn ts_type_to_csharp_type(ts_type: &TSType) -> Str {
    match ts_type {
        TSType::TSStringKeyword => "string",
        TSType::TSNumberKeyword => "double",
        TSType::TSBooleanKeyword => "bool",
        TSType::TSAnyKeyword => "dynamic",
        TSType::TSArrayType(ref arr) => {
            let elem_type = ts_type_to_csharp_type(&arr.element_type);
            format!("List<{}>", elem_type)
        }
        _ => "dynamic",
    }
}
```

### Pattern 4: Generate C# Code

**Babel:**
```javascript
function generateCSharpClass(component) {
  let code = '';

  code += `public class ${component.name} : MinimactComponent {\n`;

  // Generate state fields
  for (const state of component.useState) {
    code += `    private ${state.type} _${state.varName};\n`;
  }

  code += `\n`;

  // Generate render method
  code += `    public override VNode Render() {\n`;
  code += `        return ${component.renderBody};\n`;
  code += `    }\n`;

  code += `}\n`;

  return code;
}
```

**RustScript:**
```rustscript
pub fn generate_csharp_class(component: &ComponentInfo) -> Str {
    let mut code = String::new();

    code.push_str(&format!("public class {} : MinimactComponent {{\n", component.name));

    // Generate state fields
    for state in &component.use_state {
        code.push_str(&format!("    private {} _{};\n", state.type_name, state.var_name));
    }

    code.push_str("\n");

    // Generate render method
    code.push_str("    public override VNode Render() {\n");
    code.push_str(&format!("        return {};\n", component.render_body));
    code.push_str("    }\n");

    code.push_str("}\n");

    code
}
```

---

## Quick Reference: Common Conversions

| Babel | RustScript |
|-------|------------|
| `const x = ...` | `let x = ...` |
| `let x = ...` | `let mut x = ...` |
| `if (x != null)` | `if let Some(x) = ...` |
| `arr.push(x)` | `arr.push(x)` (same) |
| `arr.length` | `arr.len()` |
| `obj[key]` | `map.get(key)` |
| `"key" in obj` | `map.contains_key("key")` |
| `for (const x of arr)` | `for x in &arr` |
| `arr.map(x => ...)` | `arr.iter().map(\|x\| ...).collect()` |
| `str.startsWith("x")` | `str.starts_with("x")` |
| `str.includes("x")` | `str.contains("x")` |
| `\`Hello ${x}\`` | `format!("Hello {}", x)` |
| `t.isIdentifier(n)` | `matches!(n, Identifier)` |
| `path.node.name` | `node.name.clone()` |
| `return null` | `return None` |
| `throw new Error()` | `return Err(...)` |

---

## Conversion Workflow

1. **Start with helpers** - Convert utility functions first (helpers.cjs)
2. **Define structs** - Create data structures for component info, hooks, etc.
3. **Convert extractors** - Port extraction logic for props, hooks, etc.
4. **Convert generators** - Port C# code generation
5. **Wire plugin** - Create main plugin that orchestrates everything
6. **Test incrementally** - Test each module as you convert it

---

## Tips

- **Use the RustScript spec** - Refer to `docs/rustscript-specification.md` for details
- **Pattern match liberally** - Use `if let` and `match` for unwrapping
- **Clone when needed** - Don't fight the ownership system, just `.clone()`
- **Test as you go** - Compile after each function to catch errors early
- **Keep it simple** - Don't try to be too clever, straightforward code is best
- **Ask for help** - If stuck, check existing RustScript examples in `rustscript/tests/`

---

Good luck with the conversion! 🚀
