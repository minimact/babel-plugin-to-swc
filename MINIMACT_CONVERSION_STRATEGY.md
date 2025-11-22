# babel-plugin-minimact → RustScript Conversion Strategy

## Current Situation

**Source**: `example/babel-plugin-minimact/` - A modular Babel plugin that transpiles React/JSX to C# Minimact components
- **Size**: ~10,000 lines across 127 files
- **Complexity**: Very high - hooks, templates, hot reload, C# codegen
- **Architecture**: Modular (extractors, analyzers, generators, utils)

**Target**: RustScript - A DSL that compiles to both Babel and SWC plugins

## Critical Constraints

### What RustScript Needs (Not Yet Implemented)
1. **Module System** - Currently all code must be in one file
2. **File I/O API** - How to write `.cs`, `.templates.json`, `.tsx.keys` files
3. **Impl blocks** - Methods on custom structs (partially working)

### What babel-plugin-minimact Needs
1. **127 modules** organized into logical units
2. **File output** - Writes multiple files (C#, JSON, etc.)
3. **Complex state management** - Tracks components, hooks, templates across visitor

## Strategy: Build What We Need While Converting

Instead of converting the existing plugin directly, we'll use it as a **reference implementation** while building RustScript features incrementally.

---

## Phase 1: Design RustScript Module System (FIRST PRIORITY)

Before converting anything, we need a module system.

### Step 1.1: Design Module Syntax

**Proposal**:
```rustscript
// File: rustscript/minimact/main.rsc
use "./utils/helpers.rsc" as helpers;
use "./extractors/hooks.rsc" as hooks;

plugin MinimactPlugin {
    fn visit_function_declaration(node: &mut FunctionDeclaration, ctx: &Context) {
        let name = helpers::get_component_name(node);
        let hook_data = hooks::extract_hooks(node);
    }
}
```

**Questions to answer**:
- How do `use` statements work? Relative paths? Module names?
- How are exports declared? `pub fn`?
- Does each `.rsc` file compile independently or as a unit?
- How do Babel and SWC handle multi-file plugins?

### Step 1.2: Implement Module System Parser

Update `rustscript/src/parser/mod.rs` to handle:
- `use` statements at top of file
- Module path resolution
- Export/import semantics

### Step 1.3: Implement Module System Codegen

Update codegen to:
- **Babel**: Use `require()` for imports
- **SWC**: Use Rust `mod` and `use` statements

### Step 1.4: Test Module System

Create simple multi-file plugin:
```
test-modules/
  main.rsc       - plugin declaration
  helpers.rsc    - helper functions
```

**Success criteria**: Compiles to working Babel plugin with require()

---

## Phase 2: Design File I/O API

### Step 2.1: Define File I/O Syntax

**Proposal**:
```rustscript
use std::fs;

plugin MinimactPlugin {
    fn visit_program_exit(node: &Program, ctx: &Context) {
        let csharp_code = generate_csharp(self.state.components);

        // Write C# file
        fs::write_file("Component.cs", csharp_code)?;

        // Write JSON metadata
        let json = to_json(self.state.templates);
        fs::write_file("Component.templates.json", json)?;
    }
}
```

**Questions**:
- What's the API? `fs::write_file(path, content)`?
- How does it work in Babel? Node.js `fs` module?
- How does it work in SWC? Rust `std::fs`?
- Error handling? `Result` return type?

### Step 2.2: Implement File I/O in Codegen

- **Babel**: Map to `require('fs').writeFileSync()`
- **SWC**: Map to `std::fs::write()`

### Step 2.3: Test File I/O

Create plugin that writes a simple file during transformation.

---

## Phase 3: Start Conversion (Simple Subset First)

Don't try to convert everything. Start with a **minimal viable transpiler**.

### Step 3.1: Core Infrastructure (Helpers)

Convert: `utils/helpers.cjs`
- `getComponentName()`
- `escapeCSharpString()`

**File**: `rustscript/minimact/utils/helpers.rsc`
**Lines**: ~100

### Step 3.2: Type Conversion

Convert: `types/typeConversion.cjs`
- `tsTypeToCSharpType()`
- Map TypeScript types → C# types

**File**: `rustscript/minimact/types/conversion.rsc`
**Lines**: ~150

### Step 3.3: Basic Prop Extraction

Convert: `extractors/props.cjs`
- Extract props from function parameters

**File**: `rustscript/minimact/extractors/props.rsc`
**Lines**: ~80

### Step 3.4: Basic Hook Extraction (useState only)

Convert: `extractors/hooks.cjs` (useState portion only)
- Detect `useState` calls
- Extract var name, setter name, initial value

**File**: `rustscript/minimact/extractors/useState.rsc`
**Lines**: ~50

### Step 3.5: Simple C# Generator

Convert: `generators/component.cjs` (basic structure only)
- Generate C# class skeleton
- Generate fields for state
- Generate simple Render() method

**File**: `rustscript/minimact/generators/component.rsc`
**Lines**: ~200

### Step 3.6: Main Plugin

Tie everything together:
- Visit `FunctionDeclaration`
- Call extractors
- Call generators
- Write `.cs` file

**File**: `rustscript/minimact/main.rsc`
**Lines**: ~150

### Step 3.7: Test Minimal Transpiler

**Input** (simple React component):
```jsx
function Counter({ initial }) {
  const [count, setCount] = useState(initial);
  return <div>{count}</div>;
}
```

**Expected Output** (`Counter.cs`):
```csharp
public class Counter : MinimactComponent {
    [Prop] public dynamic Initial { get; set; }
    private int _count;

    public override void Render() {
        // Basic render
    }
}
```

**Success Criteria**: Generates valid C# class with props and state

---

## Phase 4: Expand Feature Set (Incrementally)

Once the minimal transpiler works, add features one at a time:

### Feature 1: useEffect Hook
- Convert `extractors/hooks.cjs` (useEffect portion)
- Update C# generator to emit effect methods

### Feature 2: JSX to VNode
- Convert `generators/jsx.cjs`
- Generate `Render()` method body with VNode construction

### Feature 3: Template Extraction
- Convert `extractors/templates.cjs`
- Generate `.templates.json` files

### Feature 4: Loop Templates
- Convert `extractors/loopTemplates.cjs`
- Handle `.map()` patterns

### Feature 5: Conditional Rendering
- Convert `extractors/structuralTemplates.cjs`
- Handle ternary and `&&` operators

... (continue incrementally)

---

## Phase 5: Complete Feature Parity (Long-term)

Eventually work toward full feature set:
- Custom hooks
- Timeline animations
- Plugin system
- Hot reload
- All 127 modules converted

---

## Implementation Order

### Week 1: Foundation
1. ✅ Design module system syntax
2. ✅ Implement module parser
3. ✅ Implement module codegen
4. ✅ Test multi-file plugin
5. ✅ Design file I/O API
6. ✅ Implement file I/O codegen
7. ✅ Test file writing

### Week 2-3: Minimal Transpiler
8. ✅ Convert helpers module
9. ✅ Convert type conversion module
10. ✅ Convert props extractor
11. ✅ Convert useState extractor
12. ✅ Convert basic C# generator
13. ✅ Create main plugin
14. ✅ Test with simple Counter component

### Week 4+: Incremental Features
15. Add useEffect support
16. Add JSX→VNode generation
17. Add template extraction
18. ... (continue based on priority)

---

## Module Organization (Target Structure)

```
rustscript/minimact/
  main.rsc                          # Plugin entry point

  utils/
    helpers.rsc                     # getComponentName, escapeCSharpString
    hexPath.rsc                     # Hex path generation
    pathAssignment.rsc              # JSX path assignment

  types/
    conversion.rsc                  # TS → C# type mapping

  extractors/
    props.rsc                       # Prop extraction
    useState.rsc                    # useState extraction
    useEffect.rsc                   # useEffect extraction
    useRef.rsc                      # useRef extraction
    hooks.rsc                       # Hook dispatcher
    localVariables.rsc              # Local var extraction
    eventHandlers.rsc               # Event handler extraction
    templates.rsc                   # Template extraction
    loopTemplates.rsc               # Loop template extraction
    structuralTemplates.rsc         # Conditional template extraction
    expressionTemplates.rsc         # Expression template extraction

  analyzers/
    classification.rsc              # Node classification
    dependencies.rsc                # Dependency analysis
    detection.rsc                   # Pattern detection
    hookAnalyzer.rsc                # Hook analysis
    timelineAnalyzer.rsc            # Timeline analysis

  generators/
    component.rsc                   # Component class generation
    csharpFile.rsc                  # Complete C# file
    jsx.rsc                         # JSX → VNode
    expressions.rsc                 # Expression generation
    renderBody.rsc                  # Render method body
    runtimeHelpers.rsc              # Runtime helper calls

  transpilers/
    typescriptToCSharp.rsc          # Full TS → C# transpiler
```

~30-40 modules (much more manageable than 127!)

---

## Success Metrics

### Phase 1 Complete:
- ✅ Multi-file RustScript plugins work
- ✅ Can write files from plugin
- ✅ Compiles to Babel and SWC

### Phase 3 Complete (Minimal Transpiler):
- ✅ Can transpile simple Counter component
- ✅ Generates valid C# class
- ✅ Extracts props and useState
- ✅ Generated code compiles in C#

### Phase 4 Complete (Useful Transpiler):
- ✅ All common hooks (useState, useEffect, useRef)
- ✅ JSX → VNode generation works
- ✅ Template extraction works
- ✅ Can transpile real-world components (e.g., BlogPost example)

### Phase 5 Complete (Full Feature Parity):
- ✅ All babel-plugin-minimact features ported
- ✅ Passes all minimact test cases
- ✅ Hot reload support
- ✅ Custom hooks support
- ✅ Timeline support

---

## Questions for User

Before starting, clarify:

1. **Module system priority**: Should we implement this FIRST before any conversion?
2. **File I/O API**: What syntax do you prefer? `fs::write_file()` or something else?
3. **Scope**: Start with minimal transpiler (Phase 3) or need full feature set immediately?
4. **Testing**: Should we create test cases for each module as we convert?
5. **Target structure**: Does the proposed module organization look good?

---

## Recommended First Steps

1. **TODAY**: Implement module system (parser + codegen)
2. **THIS WEEK**: Implement file I/O API
3. **NEXT WEEK**: Start minimal transpiler (helpers + props + useState)
4. **WEEK 3**: Complete minimal transpiler and test with Counter
5. **WEEK 4+**: Add features incrementally

This gives us a solid foundation and a working (if limited) transpiler quickly, then we can expand from there.

---

## Notes

- The original 127-file structure is too granular for RustScript
- We can consolidate related functionality into larger modules
- Focus on **getting something working** before achieving full parity
- Use babel-plugin-minimact as **reference**, not source of truth
- Test incrementally - don't write 10k lines before first test
