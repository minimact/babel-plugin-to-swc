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
        // JSX elements detected
        // TODO: Generate JSX translation
        return null;
    }
}

