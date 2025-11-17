# C# Generator Implementation Plan

## 🎯 Goal
Add C# Blazor code generation to the generated SWC plugin so it outputs `.cs` files, not just JSON metadata.

## 📊 Current State

### What We Have ✅
- Generated SWC plugin extracts component metadata:
  - Component name
  - Hooks (useState with state/setter names, initial values)
  - JSX elements (type, attributes)
  - Props
- JSON output matching Babel plugin format

### What's Missing ❌
- C# file generation
- C# component class generation
- C# expression translation (JSX → C#)
- File writing to `.cs` output

## 📁 Babel Plugin Architecture (Reference)

The original `babel-plugin-minimact` has this structure:

```
babel-plugin-minimact/
├── index.cjs                          # Main plugin entry
└── src/
    └── generators/
        ├── csharpFile.cjs             # File-level generator
        │   └── generateCSharpFile()   # Generates complete .cs file
        ├── component.cjs              # Component class generator
        │   └── generateComponent()    # Generates component class
        ├── expressions.cjs            # Expression translator
        │   └── generateCSharpExpression() # JSX → C# expression
        ├── jsx.cjs                    # JSX element generator
        ├── hooks.cjs                  # Hook translator (useState → C# properties)
        └── plugin.cjs                 # Plugin system support
```

### Flow in Babel Plugin:
1. Extract metadata (hooks, JSX, props) ✅ **WE HAVE THIS**
2. Call `generateCSharpFile(components)` ❌ **NEED TO ADD**
3. Write to `{ComponentName}.cs` ❌ **NEED TO ADD**

## 🏗️ Implementation Plan

### Phase 1: Basic C# File Generation

**Goal:** Generate a minimal working C# class file

**Files to Create:**
- `generated-plugin/src/csharp_generator.rs` - Main C# generation module

**Output Format:**
```csharp
using Minimact.AspNetCore.Core;
using Minimact.AspNetCore.Extensions;
using MinimactHelpers = Minimact.AspNetCore.Core.Minimact;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;

namespace Minimact.Components;

public class Counter : MinimactComponent
{
    // Properties for state
    private int count = 0;
    private string message = "Hello";

    // Render method
    protected override MinimactNode Render()
    {
        return MinimactHelpers.Element("div", new { id = "counter-root" },
            MinimactHelpers.Element("span", new { id = "counter-value" }, count),
            MinimactHelpers.Element("span", new { id = "message" }, message),
            MinimactHelpers.Element("button", new {
                id = "increment-btn",
                type = "button",
                onclick = (Action)(() => { count++; })
            }, "Increment")
        );
    }
}
```

**Steps:**
1. ✅ Create `csharp_generator.rs` module
2. ✅ Implement `generate_csharp_file(components: &[Component]) -> String`
3. ✅ Generate using statements
4. ✅ Generate namespace
5. ✅ Generate component class with:
   - State properties from hooks
   - Render method stub
6. ✅ Write to file in `main.rs`

### Phase 2: Hook Translation

**Goal:** Convert `useState` hooks to C# properties

**Mapping:**
```javascript
const [count, setCount] = useState(0);
const [message, setMessage] = useState("Hello");
```
↓
```csharp
private int count = 0;
private string message = "Hello";
```

**Steps:**
1. Parse hook initial value to determine C# type:
   - `"0"` → `int count = 0`
   - `"\"Hello\""` → `string message = "Hello"`
   - `"true"` → `bool flag = true`
   - `"[]"` → `List<object> items = new()`
2. Generate property declarations
3. Handle setter logic (for now, just direct assignment)

### Phase 3: JSX to C# Translation

**Goal:** Convert JSX elements to `MinimactHelpers.Element()` calls

**Mapping:**
```jsx
<div id="counter-root">
  <span id="counter-value">{count}</span>
  <button onClick={() => setCount(count + 1)}>Increment</button>
</div>
```
↓
```csharp
MinimactHelpers.Element("div", new { id = "counter-root" },
    MinimactHelpers.Element("span", new { id = "counter-value" }, count),
    MinimactHelpers.Element("button", new {
        onclick = (Action)(() => { count++; })
    }, "Increment")
)
```

**Steps:**
1. Traverse JSX elements from metadata
2. Generate `Element()` calls with:
   - Tag name
   - Attributes as anonymous object `new { ... }`
   - Children (nested recursively)
3. Handle expressions in attributes (e.g., `onClick` → C# lambda)
4. Handle text content vs expression children

### Phase 4: Expression Translation

**Goal:** Translate JavaScript expressions to C# expressions

**Mappings:**
| JavaScript | C# |
|------------|-----|
| `count` | `count` |
| `message` | `message` |
| `count + 1` | `count + 1` |
| `() => setCount(count + 1)` | `(Action)(() => { count++; })` |
| `{expression}` | `expression` (unwrap) |

**Steps:**
1. Detect expression types
2. Translate operators (mostly 1:1)
3. Handle function calls
4. Handle arrow functions → C# lambdas

### Phase 5: File Writing

**Goal:** Write generated C# to `.cs` files

**Steps:**
1. Update `main.rs` to call C# generator
2. Determine output path (same dir as input with `.cs` extension)
3. Write file using `std::fs::write`
4. Print success message

## 🎯 Minimal MVP (Start Here!)

**Goal:** Generate a simple Counter.cs file from Counter.tsx

**What to implement first:**
1. ✅ `csharp_generator.rs` with basic structure
2. ✅ Generate using statements + namespace
3. ✅ Generate class declaration
4. ✅ Convert `useState` hooks to properties (simple types only: int, string, bool)
5. ✅ Generate empty `Render()` method (returns null for now)
6. ✅ Write to `Counter.cs`

**Expected Output (MVP):**
```csharp
using Minimact.AspNetCore.Core;
using Minimact.AspNetCore.Extensions;
using MinimactHelpers = Minimact.AspNetCore.Core.Minimact;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;

namespace Minimact.Components;

public class Counter : MinimactComponent
{
    private int count = 0;
    private string message = "Hello";

    protected override MinimactNode Render()
    {
        // TODO: Generate JSX translation
        return null;
    }
}
```

## 📝 Implementation Checklist

### MVP (Phase 1 + 2)
- [ ] Create `src/csharp_generator.rs`
- [ ] Add module to `lib.rs`: `mod csharp_generator; pub use csharp_generator::*;`
- [ ] Implement `generate_csharp_file(components: &[Component]) -> String`
  - [ ] Generate using statements
  - [ ] Generate namespace
  - [ ] Generate class declaration
  - [ ] Implement `generate_state_properties(hooks: &[Hook]) -> Vec<String>`
  - [ ] Generate empty Render() method
- [ ] Update `main.rs` to call C# generator and write file
- [ ] Test on Counter.tsx

### Full Implementation (Phase 3 + 4 + 5)
- [ ] Implement JSX translation
  - [ ] `generate_render_method(jsx_elements: &[JsxElementData]) -> String`
  - [ ] Handle nested elements
  - [ ] Handle attributes
  - [ ] Handle children
- [ ] Implement expression translation
  - [ ] State variable references
  - [ ] Arithmetic operators
  - [ ] Event handlers (onClick → lambdas)
- [ ] Full integration test
- [ ] Compare output with Babel plugin's `.cs` output

## 🧪 Testing Strategy

1. **Unit Tests:** Test each generator function individually
   - `generate_state_properties()`
   - `infer_csharp_type()`
   - `generate_render_method()`

2. **Integration Test:**
   - Run on `Counter.tsx`
   - Compare output with Babel plugin's Counter.cs
   - Verify C# compiles with `dotnet build`

3. **Regression Test:**
   - Run on TypedProps.tsx
   - Verify more complex scenarios

## 🚀 Success Criteria

✅ **MVP Success:**
- Generated `Counter.cs` file exists
- Contains correct using statements
- Contains correct namespace
- Contains Counter class extending MinimactComponent
- Contains state properties from useState hooks with correct types and initial values
- Contains Render() method (even if empty)

✅ **Full Success:**
- Generated `Counter.cs` compiles with `dotnet`
- Render() method returns correct JSX translation
- Event handlers work (onClick translated to C# lambda)
- Matches Babel plugin output structure

## 📚 Reference Files

**To Study:**
1. `example/babel-plugin-minimact/src/generators/csharpFile.cjs`
2. `example/babel-plugin-minimact/src/generators/component.cjs`
3. `example/babel-plugin-minimact/src/generators/hooks.cjs`
4. `example/babel-plugin-minimact/src/generators/jsx.cjs`

**Output Examples:**
- Run Babel plugin on Counter.tsx and save the `.cs` output
- Use as reference for what the Rust version should generate

---

## 🏁 Let's Start!

**First Step:** Create `src/csharp_generator.rs` with MVP implementation
