# Testing Framework - Babel vs Rust/SWC Output Validation

## Overview

This testing framework ensures that the Rust/SWC transpiler generates code that produces **identical transformations** to the original Babel plugin.

## Test Architecture

```
┌──────────────────┐
│  Input Fixture   │  Counter.tsx
│  (TSX/JSX code)  │  TypedProps.tsx
└────────┬─────────┘
         │
         ├─────────────────────────────┐
         │                             │
         ▼                             ▼
┌────────────────┐           ┌─────────────────┐
│  Babel Plugin  │           │  Rust Transpiler│
│  (JavaScript)  │           │  (Generated SWC)│
└────────┬───────┘           └────────┬────────┘
         │                             │
         ▼                             ▼
┌────────────────┐           ┌─────────────────┐
│  Reference     │           │  Test Output    │
│  Output (JSON) │           │  (JSON)         │
└────────┬───────┘           └────────┬────────┘
         │                             │
         └──────────────┬──────────────┘
                        ▼
                ┌──────────────┐
                │ Diff Tool    │
                │ - Compare    │
                │ - Report     │
                └──────────────┘
```

## Files Created

### Framework Files

1. **`TEST_FRAMEWORK.md`** - Architecture and design documentation
2. **`tests/test-runner.js`** - Automated test execution and comparison
3. **`test.bat`** - Windows batch script to run tests
4. **`tests/manual-test.bat`** - Manual testing for debugging

### Output Directories

- `tests/reference-outputs/` - Reference outputs from Babel plugin
- `tests/test-outputs/` - Outputs from Rust/SWC transpiler

## Running Tests

### Automated Test Suite

```bash
# Run all tests
./test.bat

# Run with specific test
./test.bat Counter

# Regenerate reference outputs
./test.bat --regenerate
```

### Manual Testing

```bash
# Compare outputs manually
./tests/manual-test.bat

# Run Babel plugin only
node tests/test-runner.js

# Run Rust transpiler only
target/debug/babel-to-swc.exe --json
```

## Test Cases

### Existing Fixtures

1. **`fixtures/Counter.tsx`**
   - useState hooks (2 instances)
   - JSX elements (div, span, button)
   - Event handlers (onClick)
   - Simple props

2. **`fixtures/TypedProps.tsx`**
   - TypeScript type annotations
   - Props with types
   - Interface definitions

## Output Format

### Babel Plugin Output (Reference)

```json
{
  "components": [
    {
      "name": "Counter",
      "props": [],
      "hooks": [
        {
          "type": "useState",
          "stateName": "count",
          "setterName": "setCount",
          "initialValue": "0",
          "hasTypeAnnotation": false
        },
        {
          "type": "useState",
          "stateName": "message",
          "setterName": "setMessage",
          "initialValue": "\"Hello\"",
          "hasTypeAnnotation": false
        }
      ],
      "jsxElements": [
        {
          "type": "div",
          "attributes": [
            {"name": "id", "value": "counter-root", "isExpression": false}
          ],
          "hasChildren": true
        },
        {
          "type": "span",
          "attributes": [
            {"name": "id", "value": "counter-value", "isExpression": false}
          ],
          "hasChildren": true
        },
        {
          "type": "button",
          "attributes": [
            {"name": "id", "value": "increment-btn", "isExpression": false},
            {"name": "type", "value": "button", "isExpression": false},
            {"name": "onClick", "value": "<expression>", "isExpression": true}
          ],
          "hasChildren": true
        }
      ],
      "localVariables": [],
      "helperFunctions": []
    }
  ]
}
```

### Rust/SWC Output (Test)

```json
{
  "visitorMethods": [
    {
      "name": "Program",
      "transformCount": 0
    },
    {
      "name": "FunctionDeclaration",
      "transformCount": 1
    }
  ]
}
```

## Comparison Criteria

### Must Match Exactly

1. **Component Count** - Same number of components detected
2. **Component Names** - Exact name matching
3. **Hook Count** - Same number of hooks
4. **Hook Types** - useState, useEffect, etc.
5. **Hook Names** - State variable and setter names
6. **Initial Values** - Hook initial values
7. **Props Count** - Number of props
8. **Props Names** - Prop names and types
9. **JSX Element Count** - Number of JSX elements
10. **JSX Element Types** - div, span, button, etc.
11. **Attribute Count** - Number of attributes per element
12. **Attribute Names** - Exact attribute names
13. **Attribute Values** - Exact values or expression markers

### May Differ (Acceptable)

1. **Order** - Components, hooks, props in different order (if semantically equivalent)
2. **Internal IDs** - Generated IDs or keys
3. **Formatting** - Whitespace, quotes in strings
4. **Comments** - Documentation comments

## Diff Types Reported

```javascript
{
  "type": "component-count",
  "babel": 1,
  "rust": 0,
  "severity": "error"
}

{
  "type": "missing-hook",
  "component": "Counter",
  "hook": "useState",
  "babel": { "stateName": "count", ... },
  "rust": null,
  "severity": "error"
}

{
  "type": "hook-value",
  "component": "Counter",
  "property": "initialValue",
  "babel": "0",
  "rust": "undefined",
  "severity": "warning"
}
```

## Success Criteria

✅ **100% Transform Coverage**
- All Babel-detected transforms must be detected by Rust

✅ **Zero False Positives**
- Rust should not detect transforms that Babel doesn't

✅ **Exact Value Matching**
- Hook names, initial values, prop names must match exactly

✅ **Type Accuracy**
- Correct hook types (useState vs useEffect)
- Correct JSX element types (div vs span)

## Current Status

### Implemented ✅

- [x] Test framework architecture
- [x] Test runner script (`test-runner.js`)
- [x] Babel plugin metadata export
- [x] Rust JSON output mode
- [x] Diff comparison logic
- [x] Manual test script
- [x] Automated test batch file

### TODO 🚧

- [ ] Implement full JSON output from Rust (currently only visitor counts)
- [ ] Add component/hook/prop extraction to Rust output
- [ ] Run full test suite on all fixtures
- [ ] Add visual diff reporter (HTML output)
- [ ] Add CI/CD integration
- [ ] Add performance benchmarks

## Next Steps

1. **Enhance Rust JSON Output**
   - Extract full component data (hooks, props, JSX)
   - Match Babel output structure exactly

2. **Run Initial Tests**
   - Execute test suite on Counter.tsx
   - Identify gaps in detection

3. **Iterate Until Parity**
   - Fix detection gaps
   - Ensure 100% match on all fixtures

4. **Add More Test Cases**
   - Complex hooks (useEffect, custom hooks)
   - Conditional rendering
   - List mapping (.map())
   - Event handlers

## Example Test Run

```bash
$ ./test.bat

═══════════════════════════════════════════════
  Building Rust Transpiler
═══════════════════════════════════════════════
[Build output...]

═══════════════════════════════════════════════
  Running Test Suite
═══════════════════════════════════════════════

Found 2 test fixtures

==================================================
TEST: Counter
==================================================

[Babel] Processing Counter.tsx...
[Babel] Reference saved to tests/reference-outputs/Counter.json
[Rust/SWC] Processing Counter.tsx...
[Rust/SWC] Output saved to tests/test-outputs/Counter.json

[Compare] Analyzing Counter...
❌ Counter: FAILED (3 differences)
   - hook-count: {"babel":2,"rust":0}
   - jsx-element-count: {"babel":4,"rust":0}
   - component-name: {"babel":"Counter","rust":""}

==================================================
TEST: TypedProps
==================================================
[Similar output...]

═══════════════════════════════════════════════
  TEST SUMMARY
═══════════════════════════════════════════════

Total Tests: 2
✅ Passed: 0
❌ Failed: 2

Failed Tests:
  - Counter (3 diffs)
  - TypedProps (2 diffs)
```

## Contributing

When adding new test cases:

1. Add fixture to `fixtures/` directory
2. Ensure fixture has clear, testable patterns
3. Run test suite to generate reference output
4. Verify reference output is correct
5. Fix Rust transpiler if diffs detected
6. Re-run until 100% match
