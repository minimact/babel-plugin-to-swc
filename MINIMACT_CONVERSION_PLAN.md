# Minimact Babel Plugin → RustScript Conversion Plan

## Overview

Converting ~10,000 lines of modular Babel plugin code (127 files) to RustScript.

**Goal**: Port babel-plugin-minimact from example/babel-plugin-minimact to RustScript syntax.

## Analysis Summary

### Plugin Purpose
Transforms React/JSX components to C# Minimact components with:
- Hook tracking (useState, useEffect, useRef, custom hooks)
- Template extraction for hot reload
- Client/server state splitting
- C# code generation
- Timeline/animation support
- Plugin system integration

### Architecture (6 major categories)
1. **Entry Point** - Visitor coordination
2. **Extractors** - Pull data from AST (hooks, props, templates, etc.)
3. **Analyzers** - Analyze and classify AST patterns
4. **Generators** - Generate C# output code
5. **Transpilers** - TypeScript → C#/Rust conversion
6. **Utils** - Helper functions

---

## Conversion Strategy

### Phase 1: Foundation (Steps 1-3)
Build basic infrastructure to support the plugin

### Phase 2: Core Extraction (Steps 4-8)
Implement data extraction from AST

### Phase 3: Code Generation (Steps 9-13)
Implement C# code generation

### Phase 4: Advanced Features (Steps 14-18)
Implement template extraction and hot reload

### Phase 5: Integration (Steps 19-20)
Wire everything together and test

---

## Detailed Steps

### STEP 1: Create RustScript plugin skeleton
**Complexity**: Low
**Files**: 1 new file
**Lines**: ~50

**Tasks**:
- Create `rustscript/tests/minimact/minimact.rsc`
- Define plugin structure:
  ```rust
  plugin MinimactPlugin {
      // State will go here
  }
  ```
- Add empty visitor methods for FunctionDeclaration, Program

**Dependencies**: None
**Output**: Compiles successfully with empty visitor

---

### STEP 2: Port utils/helpers.cjs
**Complexity**: Low
**Files**: utils/helpers.cjs → helper functions in plugin
**Lines**: ~100

**Tasks**:
- Port `getComponentName()` - Extract component name from function/arrow function
- Port `escapeCSharpString()` - String escaping for C# generation
- Add as helper functions in plugin body

**Dependencies**: None
**Output**: Helper functions compile and work in isolation

---

### STEP 3: Port types/typeConversion.cjs
**Complexity**: Medium
**Files**: types/typeConversion.cjs → type conversion functions
**Lines**: ~150

**Tasks**:
- Port `tsTypeToCSharpType()` - Convert TypeScript types to C# types
  - Map: string → String, number → double, boolean → bool, etc.
- Port `inferType()` - Infer C# type from AST nodes
- Create type mapping table/match expression

**Dependencies**: Step 2 (helpers)
**Output**: Type conversion functions work

**Test**: Create small test that converts `number[]` → `List<double>`

---

### STEP 4: Port extractors/props.cjs
**Complexity**: Medium
**Files**: extractors/props.cjs → prop extraction logic
**Lines**: ~80

**Tasks**:
- Extract props from function parameters
- Handle destructured props: `function Comp({ prop1, prop2 })`
- Handle single prop object: `function Comp(props)`
- Extract TypeScript type annotations if present

**Dependencies**: Step 3 (type conversion)
**Output**: Can extract props from function signature

**Test**: `function Foo({ name: string, age: number })` → props array

---

### STEP 5: Port extractors/hooks.cjs (basic hooks)
**Complexity**: High
**Files**: extractors/hooks.cjs → hook extraction
**Lines**: ~300

**Tasks**:
- Port `extractHook()` - Main hook dispatcher
- Port `extractUseState()` - Extract useState calls
- Port `extractUseEffect()` - Extract useEffect calls
- Port `extractUseRef()` - Extract useRef calls
- Store hook data in component structure

**Dependencies**: Step 3 (type conversion)
**Output**: Can extract useState, useEffect, useRef from component

**Test**: Component with `const [count, setCount] = useState(0)` → hook data

---

### STEP 6: Port extractors/localVariables.cjs
**Complexity**: Medium
**Files**: extractors/localVariables.cjs → local variable extraction
**Lines**: ~100

**Tasks**:
- Extract `const`, `let`, `var` declarations
- Track variable names and types
- Store in component structure

**Dependencies**: Step 3 (type conversion)
**Output**: Can extract local variables from function body

---

### STEP 7: Port extractors/eventHandlers.cjs
**Complexity**: Medium
**Files**: extractors/eventHandlers.cjs → event handler extraction
**Lines**: ~50

**Tasks**:
- Detect event handler functions (onClick, onChange, etc.)
- Extract handler function bodies
- Store in component structure

**Dependencies**: None
**Output**: Can detect onClick={() => ...} patterns

---

### STEP 8: Port analyzers (detection, classification, dependencies)
**Complexity**: Medium
**Files**: analyzers/*.cjs → analysis functions
**Lines**: ~200

**Tasks**:
- Port `classifyNode()` - Classify AST nodes (static/dynamic/hybrid)
- Port `hasSpreadProps()` - Detect spread operators
- Port `hasDynamicChildren()` - Detect dynamic children
- Port `hasComplexProps()` - Detect complex prop patterns
- Port `analyzeDependencies()` - Track state dependencies per JSX node

**Dependencies**: Steps 5, 6
**Output**: Can classify and analyze JSX nodes

---

### STEP 9: Port generators/expressions.cjs (basic)
**Complexity**: High
**Files**: generators/expressions/*.cjs → expression generation
**Lines**: ~500

**Tasks**:
- Port `generateCSharpExpression()` - Main expression generator
- Handle literals (strings, numbers, booleans, null)
- Handle identifiers (variable references)
- Handle binary operators (+, -, *, /, ==, !=, etc.)
- Handle member expressions (obj.prop, arr[0])
- Handle call expressions (func(), obj.method())

**Dependencies**: Step 3 (type conversion)
**Output**: Can generate C# code for basic expressions

**Test**: `count + 1` → `count + 1` (C#)

---

### STEP 10: Port generators/expressions.cjs (advanced)
**Complexity**: High
**Files**: generators/expressions/*.cjs (continued)
**Lines**: ~400

**Tasks**:
- Port array methods (.map, .filter, .find, etc.) → LINQ
- Port string methods (.toUpperCase, .slice, etc.)
- Port template literals → string interpolation
- Port arrow functions → C# lambdas
- Handle conditional expressions (ternary)
- Handle logical operators (&&, ||)

**Dependencies**: Step 9
**Output**: Can generate C# for complex expressions

**Test**: `items.map(x => x.name)` → `items.Select(x => x.Name).ToList()`

---

### STEP 11: Port generators/jsx.cjs
**Complexity**: High
**Files**: generators/jsx.cjs → JSX to VNode generation
**Lines**: ~300

**Tasks**:
- Port `generateJSXElement()` - Convert JSX element to VNode
- Port `generateChildren()` - Process JSX children
- Port `generateFragment()` - Handle React fragments
- Generate VNode constructor calls
- Handle JSX attributes → VNode props

**Dependencies**: Steps 9, 10 (expression generation)
**Output**: Can convert `<div>Hello</div>` → `new VNode("div", null, "Hello")`

---

### STEP 12: Port generators/renderBody.cjs
**Complexity**: Medium
**Files**: generators/renderBody.cjs → Render method generation
**Lines**: ~150

**Tasks**:
- Port `generateRenderBody()` - Generate Render() method
- Combine local variables + return statement
- Format as C# method body

**Dependencies**: Steps 9, 10, 11
**Output**: Can generate complete Render() method

---

### STEP 13: Port generators/component.cjs and csharpFile.cjs
**Complexity**: High
**Files**: generators/component.cjs, csharpFile.cjs → C# class generation
**Lines**: ~400

**Tasks**:
- Port `generateComponent()` - Generate component class structure
- Port `generateCSharpFile()` - Generate complete C# file with usings
- Generate fields (state variables)
- Generate properties (props)
- Generate methods (event handlers, Render)
- Generate class boilerplate

**Dependencies**: Steps 5, 11, 12
**Output**: Can generate complete C# component class

**Test**: Simple Counter component → complete Counter.cs file

---

### STEP 14: Port extractors/templates.cjs
**Complexity**: Very High
**Files**: extractors/templates/*.cjs → template extraction
**Lines**: ~800

**Tasks**:
- Port `extractTemplates()` - Extract text templates from JSX
- Port `extractAttributeTemplates()` - Extract attribute templates
- Port `extractTemplateLiteral()` - Handle template literals
- Port `extractBinding()` - Extract state bindings
- Port `buildMemberPath()` - Build property paths
- Generate hex keys for each template

**Dependencies**: Steps 8, 11
**Output**: Can extract templates with state bindings

**Test**: `<div>{count}</div>` → template with binding to "count"

---

### STEP 15: Port extractors/loopTemplates.cjs
**Complexity**: Very High
**Files**: extractors/loopTemplates/*.cjs → .map() template extraction
**Lines**: ~700

**Tasks**:
- Port `extractLoopTemplates()` - Find .map() calls
- Port `extractLoopTemplate()` - Extract template from map callback
- Port `extractArrayBinding()` - Detect array state variable
- Port `extractKeyBinding()` - Extract key attribute
- Port `extractElementTemplate()` - Extract JSX element template
- Handle nested loops

**Dependencies**: Step 14
**Output**: Can extract predictive rendering templates from .map()

**Test**: `items.map(item => <div key={item.id}>{item.name}</div>)` → loop template

---

### STEP 16: Port extractors/structuralTemplates.cjs
**Complexity**: High
**Files**: extractors/structuralTemplates/*.cjs → conditional rendering
**Lines**: ~500

**Tasks**:
- Port `extractStructuralTemplates()` - Find conditional rendering
- Port `extractConditionalStructuralTemplate()` - Ternary operators
- Port `extractLogicalAndTemplate()` - Logical && patterns
- Port `extractElementOrFragmentTemplate()` - Extract templates from branches

**Dependencies**: Step 14
**Output**: Can extract templates from `condition ? <A/> : <B/>`

---

### STEP 17: Port extractors/conditionalElementTemplates.cjs
**Complexity**: Very High
**Files**: extractors/conditionalElementTemplates/*.cjs → enhanced conditionals
**Lines**: ~600

**Tasks**:
- Port enhanced conditional template extraction
- Port `isConditionEvaluableClientSide()` - Detect client-evaluable conditions
- Port `extractBindingsFromCondition()` - Extract bindings from conditions
- Handle complex conditional logic

**Dependencies**: Step 16
**Output**: Enhanced conditional template extraction with client-side evaluation detection

---

### STEP 18: Port extractors/expressionTemplates.cjs
**Complexity**: Very High
**Files**: extractors/expressionTemplates/*.cjs → computed expressions
**Lines**: ~700

**Tasks**:
- Port `extractExpressionTemplates()` - Extract computed value templates
- Port `extractMethodCallTemplate()` - Method call templates
- Port `extractBinaryExpressionTemplate()` - Binary operator templates
- Port `extractUnaryExpressionTemplate()` - Unary operator templates
- Port `generateExpressionString()` - Generate expression string

**Dependencies**: Step 14
**Output**: Can extract templates for `{count + 1}`, `{user.name.toUpperCase()}`

---

### STEP 19: Port processComponent.cjs (main orchestrator)
**Complexity**: Very High
**Files**: processComponent.cjs → component processing logic
**Lines**: ~500

**Tasks**:
- Port `processComponent()` - Main component processor
- Orchestrate all extractors and analyzers
- Build component data structure
- Coordinate template extraction
- Handle custom hooks
- Track external imports

**Dependencies**: All previous steps
**Output**: Complete component processing pipeline

---

### STEP 20: Port index.cjs (plugin entry point)
**Complexity**: High
**Files**: index.cjs → main plugin entry
**Lines**: ~350

**Tasks**:
- Port visitor methods (Program, FunctionDeclaration, etc.)
- Port hex path assignment
- Port .tsx.keys file generation
- Port structural change detection
- Port hot reload logic
- Wire up all generators

**Dependencies**: All previous steps
**Output**: Complete working plugin

**Test**: Full component with hooks, templates, conditionals, loops → C# + .templates.json

---

## Optional Advanced Steps (Not Core)

### STEP 21: Port analyzers/hookAnalyzer.cjs + hookDetector.cjs
**Purpose**: Custom hook support
**Lines**: ~400
**When**: If custom hooks are needed

### STEP 22: Port analyzers/timelineAnalyzer.cjs + generators/timelineGenerator.cjs
**Purpose**: Animation timeline support
**Lines**: ~300
**When**: If timeline feature is needed

### STEP 23: Port analyzers/analyzePluginUsage.cjs + generators/plugin.cjs
**Purpose**: Plugin system integration
**Lines**: ~200
**When**: If plugin system is needed

### STEP 24: Port transpilers/typescriptToCSharp.cjs + typescriptToRust.cjs
**Purpose**: Full TypeScript transpilation
**Lines**: ~600
**When**: If full TypeScript support is needed

### STEP 25: Port utils/hexPath.cjs + pathAssignment.cjs
**Purpose**: Hot reload path tracking
**Lines**: ~200
**When**: If hot reload is needed

### STEP 26: Port extractors/hookSignature.cjs
**Purpose**: Hook change detection
**Lines**: ~150
**When**: If hook change detection is needed

---

## Testing Strategy

### Per-Step Testing
After each step, create a small RustScript test file that exercises the new functionality.

Example for Step 5:
```rust
// tests/minimact/step5_hooks.rsc
plugin Step5Test {
  visitor {
    FunctionDeclaration(path) {
      // Test extracting useState
    }
  }
}
```

### Integration Testing
After major phases (1, 2, 3, 4), test with progressively complex components:

1. **Phase 1**: Empty component that compiles
2. **Phase 2**: Component with props and useState
3. **Phase 3**: Component that generates basic C# class
4. **Phase 4**: Component with templates and hot reload

### Final Testing
Use actual React component from examples/ as test case:
- `example/babel-plugin-minimact/examples/BlogPost.expected.cs`

---

## Estimated Effort

| Phase | Steps | Lines | Complexity | Estimated Time |
|-------|-------|-------|------------|----------------|
| Phase 1 | 1-3 | ~300 | Low-Medium | 2-3 hours |
| Phase 2 | 4-8 | ~730 | Medium-High | 6-8 hours |
| Phase 3 | 9-13 | ~1750 | High | 10-12 hours |
| Phase 4 | 14-18 | ~3300 | Very High | 15-20 hours |
| Phase 5 | 19-20 | ~850 | Very High | 8-10 hours |
| **TOTAL (Core)** | **1-20** | **~6930** | **High** | **41-53 hours** |
| Optional | 21-26 | ~1950 | Medium-High | 10-15 hours |
| **GRAND TOTAL** | **1-26** | **~8880** | **Very High** | **51-68 hours** |

---

## RustScript Challenges

### Known Limitations
1. **No file I/O in RustScript** - Can't write .cs, .templates.json, .tsx.keys files
   - **Solution**: Return metadata from plugin, let build system write files
2. **No require()** - Can't import Node.js modules
   - **Solution**: All logic must be self-contained in RustScript
3. **Limited string manipulation** - May need to implement C# codegen carefully
4. **No AST rewriting with recast** - Can't preserve formatting
   - **Solution**: Generate from scratch, accept different formatting

### RustScript Advantages
1. **Type safety** - Catch errors at compile time
2. **Pattern matching** - Better for AST traversal than if/else chains
3. **Single plugin file** - No module system complexity
4. **SWC compatibility** - Can target SWC for performance

---

## Success Criteria

### Minimum Viable Product (Steps 1-13)
- ✅ Can process basic React component
- ✅ Extracts props, useState, useEffect
- ✅ Generates valid C# class with Render() method
- ✅ Compiles in RustScript with no errors

### Full Feature Set (Steps 1-20)
- ✅ All basic features above
- ✅ Extracts templates for hot reload
- ✅ Handles .map() loops with predictive rendering
- ✅ Handles conditional rendering (ternary, &&)
- ✅ Generates .templates.json metadata
- ✅ Detects structural changes

### Optional Advanced (Steps 21-26)
- Custom hooks
- Animation timelines
- Plugin system
- Full TypeScript transpilation
- Hot reload with hex paths
- Hook change detection

---

## Next Steps

1. **Start with Step 1**: Create plugin skeleton
2. **Work sequentially**: Each step builds on previous
3. **Test incrementally**: Verify each step before moving on
4. **Adjust as needed**: RustScript may require different approaches
5. **Document issues**: Note any RustScript limitations encountered

---

## Questions to Resolve

1. **File output strategy**: How to return C# code and JSON metadata from RustScript?
2. **Module organization**: Single .rsc file or multiple files?
3. **Testing approach**: Use existing test infrastructure or create new?
4. **Scope**: Start with MVP (Steps 1-13) or full feature set (1-20)?

---

## Conclusion

This is a large conversion project (~7,000-9,000 lines of code). The phased approach allows for incremental progress and testing. Starting with the foundation (Steps 1-3) and basic extraction (Steps 4-8) will provide early feedback on RustScript's capabilities and limitations for this use case.

**Recommended approach**: Start with MVP (Steps 1-13) to validate feasibility, then expand to full feature set if successful.
