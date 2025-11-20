# Minimact Transpiler (RustScript)

TSX to C# transpiler written in RustScript. Compiles to both Babel and SWC plugins.

## Structure

```
minimact-transpiler-rustscript/
├── src/
│   └── main.rsc          # Main writer transpiler
├── tests/
│   └── Counter.tsx       # Test component
└── README.md
```

## Building

```bash
# Build for Babel
cd ../rustscript
cargo run -- build ../minimact-transpiler-rustscript/src/main.rsc -o ../minimact-transpiler-rustscript/dist -t babel

# Build for SWC
cargo run -- build ../minimact-transpiler-rustscript/src/main.rsc -o ../minimact-transpiler-rustscript/dist -t swc

# Build for both
cargo run -- build ../minimact-transpiler-rustscript/src/main.rsc -o ../minimact-transpiler-rustscript/dist -t both
```

## Features

### Implemented
- [x] Component detection (PascalCase functions)
- [x] useState extraction
- [x] useEffect extraction
- [x] useRef extraction
- [x] Basic C# class generation
- [x] State field generation with [State] attribute

### TODO
- [ ] JSX to VNode generation
- [ ] Event handler extraction
- [ ] Template extraction
- [ ] TypeScript interface support
- [ ] Custom hook support
- [ ] Multi-file output (.cs, .templates.json, .hooks.json)

## Expected Output

For `Counter.tsx`:

```csharp
using Minimact.Core;
using Minimact.VDom;
using System.Collections.Generic;

public class Counter : MinimactComponent
{
    [State] private int count = 0;
    [State] private bool isEnabled = true;

    protected override VNode Render()
    {
        return new VElement("div",
            new Dictionary<string, object> { ["className"] = "counter" },
            new VElement("h2", null, new VText(label)),
            new VElement("span", null, new VText(count.ToString())),
            new VElement("button",
                new Dictionary<string, object> {
                    ["onClick"] = HandleIncrement,
                    ["disabled"] = !isEnabled
                },
                new VText("+")),
            new VElement("button",
                new Dictionary<string, object> {
                    ["onClick"] = HandleDecrement,
                    ["disabled"] = !isEnabled
                },
                new VText("-"))
        );
    }

    private void HandleIncrement()
    {
        count = count + 1;
    }

    private void HandleDecrement()
    {
        count = count - 1;
    }
}
```
