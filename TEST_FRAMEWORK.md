# Babel-to-SWC Test Framework

## Architecture

The test framework validates that the generated Rust/SWC code produces **identical transformations** to the original Babel plugin.

## Test Flow

```
┌─────────────────────┐
│ Test Input (TSX)    │
│ fixtures/Counter.tsx│
└──────────┬──────────┘
           │
           ├──────────────────────────────────┐
           │                                  │
           ▼                                  ▼
┌─────────────────────┐            ┌─────────────────────┐
│ Babel Plugin        │            │ Generated Rust/SWC  │
│ (JavaScript)        │            │ (from transpiler)   │
└──────────┬──────────┘            └──────────┬──────────┘
           │                                  │
           ▼                                  ▼
┌─────────────────────┐            ┌─────────────────────┐
│ Reference Output    │            │ Test Output         │
│ (JSON/AST)          │            │ (JSON/AST)          │
└──────────┬──────────┘            └──────────┬──────────┘
           │                                  │
           └──────────────┬───────────────────┘
                          ▼
                 ┌─────────────────┐
                 │ Diff Analyzer   │
                 │ - Compare ASTs  │
                 │ - Report diffs  │
                 └─────────────────┘
```

## Test Cases

### Phase 1: Basic Transformations
1. **Simple Component** - Function component with no hooks
2. **useState Hook** - Component with useState
3. **Props Extraction** - Destructured props
4. **JSX Elements** - Basic JSX rendering

### Phase 2: Complex Transformations
5. **Multiple Hooks** - useState, useEffect, useRef
6. **Conditional Rendering** - Ternaries, logical &&
7. **Event Handlers** - onClick, onChange
8. **Helper Functions** - Declared inside component

### Phase 3: Advanced Features
9. **Custom Hooks** - useCustomHook patterns
10. **TypeScript Types** - Type annotations
11. **Template Strings** - String interpolation
12. **Array Methods** - .map(), .filter()

## Output Format

Both Babel and Rust/SWC outputs should produce:

```json
{
  "components": [
    {
      "name": "Counter",
      "hooks": [
        {
          "type": "useState",
          "stateName": "count",
          "setterName": "setCount",
          "initialValue": "0"
        }
      ],
      "props": [
        {
          "name": "initialCount",
          "type": "number"
        }
      ],
      "jsxElements": [
        {
          "type": "div",
          "attributes": [
            {"name": "className", "value": "counter"}
          ]
        }
      ]
    }
  ]
}
```

## Diff Detection

The diff tool should identify:

1. **Missing Transforms** - Babel detected something, Rust didn't
2. **Extra Transforms** - Rust detected something, Babel didn't
3. **Incorrect Values** - Same transform, different values
4. **Type Mismatches** - Wrong hook type, wrong node type
5. **Order Differences** - Same items, different order (may be acceptable)

## Test Execution

```bash
# Run all tests
./test.bat

# Run specific test
./test.bat Counter

# Generate new reference outputs
./test.bat --regenerate

# Show diffs only
./test.bat --diff-only
```

## Success Criteria

- ✅ **100% transform coverage** - All Babel transforms detected by Rust
- ✅ **Zero false positives** - No extra transforms from Rust
- ✅ **Exact value matching** - Values match byte-for-byte
- ✅ **Type accuracy** - Correct AST node types
