/**
 * Component Generator
 */

const t = require('@babel/types');
const { generateRenderBody } = require('./renderBody.cjs');
const { generateCSharpExpression, generateCSharpStatement, setCurrentComponent } = require('./expressions.cjs');
const { generateServerTaskMethods } = require('./serverTask.cjs');
const { generateTimelineAttributes } = require('./timelineGenerator.cjs');

/**
 * Generate C# class for a component
 */
function generateComponent(component) {
  // Set the current component context for useState setter detection
  setCurrentComponent(component);

  const lines = [];

  // Timeline attributes (for @minimact/timeline)
  if (component.timeline) {
    const timelineAttrs = generateTimelineAttributes(component.timeline);
    timelineAttrs.forEach(attr => lines.push(attr));
  }

  // Loop template attributes (for predictive rendering)
  if (component.loopTemplates && component.loopTemplates.length > 0) {
    for (const loopTemplate of component.loopTemplates) {
      const templateJson = JSON.stringify(loopTemplate)
        .replace(/"/g, '""'); // Escape quotes for C# verbatim string

      lines.push(`[LoopTemplate("${loopTemplate.stateKey}", @"${templateJson}")]`);
    }
  }

  // StateX projection attributes (for declarative state projections)
  if (component.useStateX && component.useStateX.length > 0) {
    for (let i = 0; i < component.useStateX.length; i++) {
      const stateX = component.useStateX[i];
      const stateKey = `stateX_${i}`;

      for (const target of stateX.targets) {
        const parts = [];

        // Required: stateKey and selector
        parts.push(`"${stateKey}"`);
        parts.push(`"${target.selector}"`);

        // Optional: Transform (C# lambda)
        if (target.transform) {
          parts.push(`Transform = @"${target.transform}"`);
        }

        // Optional: TransformId (registry reference)
        if (target.transformId) {
          parts.push(`TransformId = "${target.transformId}"`);
        }

        // Optional: ApplyAs mode
        if (target.applyAs && target.applyAs !== 'textContent') {
          parts.push(`ApplyAs = "${target.applyAs}"`);
        }

        // Optional: Property name
        if (target.property) {
          parts.push(`Property = "${target.property}"`);
        }

        // Optional: ApplyIf condition
        if (target.applyIf && target.applyIf.csharpCode) {
          parts.push(`ApplyIf = @"${target.applyIf.csharpCode}"`);
        }

        // Optional: Template hint
        if (target.template) {
          parts.push(`Template = "${target.template}"`);
        }

        lines.push(`[StateXTransform(${parts.join(', ')})]`);
      }
    }
  }

  // Class declaration
  lines.push('[Component]');

  const baseClass = component.useTemplate
    ? component.useTemplate.name
    : 'MinimactComponent';

  lines.push(`public partial class ${component.name} : ${baseClass}`);
  lines.push('{');

  // Template properties (from useTemplate)
  if (component.useTemplate && component.useTemplate.props) {
    for (const [propName, propValue] of Object.entries(component.useTemplate.props)) {
      // Capitalize first letter for C# property name
      const csharpPropName = propName.charAt(0).toUpperCase() + propName.slice(1);
      lines.push(`    public override string ${csharpPropName} => "${propValue}";`);
      lines.push('');
    }
  }

  // Prop fields (from function parameters)
  for (const prop of component.props) {
    lines.push(`    [Prop]`);
    lines.push(`    public ${prop.type} ${prop.name} { get; set; }`);
    lines.push('');
  }

  // State fields (useState)
  for (const state of component.useState) {
    lines.push(`    [State]`);
    lines.push(`    private ${state.type} ${state.name} = ${state.initialValue};`);
    lines.push('');
  }

  // MVC State fields (useMvcState)
  // ❌ DO NOT GENERATE [State] FIELDS FOR useMvcState!
  // MVC ViewModel already populates these values in the State dictionary.
  // Instead, generate readonly properties that access State dictionary with typed GetState<T>.
  if (component.useMvcState) {
    for (const mvcState of component.useMvcState) {
      const csharpType = mvcState.type || 'dynamic';
      lines.push(`    // MVC State property: ${mvcState.propertyName}`);
      lines.push(`    private ${csharpType} ${mvcState.name} => GetState<${csharpType}>("${mvcState.propertyName}");`);
      lines.push('');
    }
  }

  // MVC ViewModel fields (useMvcViewModel)
  if (component.useMvcViewModel) {
    for (const viewModel of component.useMvcViewModel) {
      lines.push(`    // useMvcViewModel - read-only access to entire ViewModel`);
      lines.push(`    private dynamic ${viewModel.name} = null;`);
      lines.push('');
    }
  }

  // State fields (useStateX)
  for (const stateX of component.useStateX) {
    lines.push(`    [State]`);
    lines.push(`    private ${stateX.initialValueType} ${stateX.varName} = ${stateX.initialValue};`);
    lines.push('');
  }

  // Ref fields (useRef)
  for (const ref of component.useRef) {
    lines.push(`    [Ref]`);
    lines.push(`    private object ${ref.name} = ${ref.initialValue};`);
    lines.push('');
  }

  // Markdown fields (useMarkdown)
  for (const md of component.useMarkdown) {
    lines.push(`    [Markdown]`);
    lines.push(`    [State]`);
    lines.push(`    private string ${md.name} = ${md.initialValue};`);
    lines.push('');
  }

  // Razor Markdown fields (useRazorMarkdown)
  // These are initialized in OnInitialized() after Razor syntax is evaluated
  if (component.useRazorMarkdown) {
    for (const md of component.useRazorMarkdown) {
      lines.push(`    [RazorMarkdown]`);
      lines.push(`    [State]`);
      lines.push(`    private string ${md.name} = null!;`);
      lines.push('');
    }
  }

  // Validation fields (useValidation)
  for (const validation of component.useValidation) {
    lines.push(`    [Validation]`);
    lines.push(`    private ValidationField ${validation.name} = new ValidationField`);
    lines.push(`    {`);
    lines.push(`        FieldKey = "${validation.fieldKey}",`);

    // Add validation rules
    if (validation.rules.required) {
      lines.push(`        Required = ${validation.rules.required.toString().toLowerCase()},`);
    }
    if (validation.rules.minLength) {
      lines.push(`        MinLength = ${validation.rules.minLength},`);
    }
    if (validation.rules.maxLength) {
      lines.push(`        MaxLength = ${validation.rules.maxLength},`);
    }
    if (validation.rules.pattern) {
      lines.push(`        Pattern = @"${validation.rules.pattern}",`);
    }
    if (validation.rules.message) {
      lines.push(`        Message = "${validation.rules.message}"`);
    }

    lines.push(`    };`);
    lines.push('');
  }

  // Modal fields (useModal)
  for (const modal of component.useModal) {
    lines.push(`    private ModalState ${modal.name} = new ModalState();`);
    lines.push('');
  }

  // Toggle fields (useToggle)
  for (const toggle of component.useToggle) {
    lines.push(`    [State]`);
    lines.push(`    private bool ${toggle.name} = ${toggle.initialValue};`);
    lines.push('');
  }

  // Dropdown fields (useDropdown)
  for (const dropdown of component.useDropdown) {
    lines.push(`    private DropdownState ${dropdown.name} = new DropdownState();`);
    lines.push('');
  }

  // Pub/Sub fields (usePub)
  if (component.usePub) {
    for (const pub of component.usePub) {
      const channelStr = pub.channel ? `"${pub.channel}"` : 'null';
      lines.push(`    // usePub: ${pub.name}`);
      lines.push(`    private string ${pub.name}_channel = ${channelStr};`);
      lines.push('');
    }
  }

  // Pub/Sub fields (useSub)
  if (component.useSub) {
    for (const sub of component.useSub) {
      const channelStr = sub.channel ? `"${sub.channel}"` : 'null';
      lines.push(`    // useSub: ${sub.name}`);
      lines.push(`    private string ${sub.name}_channel = ${channelStr};`);
      lines.push(`    private dynamic ${sub.name}_value = null;`);
      lines.push('');
    }
  }

  // Task scheduling fields (useMicroTask)
  if (component.useMicroTask) {
    for (let i = 0; i < component.useMicroTask.length; i++) {
      lines.push(`    // useMicroTask ${i}`);
      lines.push(`    private bool _microTaskScheduled_${i} = false;`);
      lines.push('');
    }
  }

  // Task scheduling fields (useMacroTask)
  if (component.useMacroTask) {
    for (let i = 0; i < component.useMacroTask.length; i++) {
      const task = component.useMacroTask[i];
      lines.push(`    // useMacroTask ${i} (delay: ${task.delay}ms)`);
      lines.push(`    private bool _macroTaskScheduled_${i} = false;`);
      lines.push('');
    }
  }

  // SignalR fields (useSignalR)
  if (component.useSignalR) {
    for (const signalR of component.useSignalR) {
      const hubUrlStr = signalR.hubUrl ? `"${signalR.hubUrl}"` : 'null';
      lines.push(`    // useSignalR: ${signalR.name}`);
      lines.push(`    private string ${signalR.name}_hubUrl = ${hubUrlStr};`);
      lines.push(`    private bool ${signalR.name}_connected = false;`);
      lines.push(`    private string ${signalR.name}_connectionId = null;`);
      lines.push(`    private string ${signalR.name}_error = null;`);
      lines.push('');
    }
  }

  // Predict hint fields (usePredictHint)
  if (component.usePredictHint) {
    for (let i = 0; i < component.usePredictHint.length; i++) {
      const hint = component.usePredictHint[i];
      const hintIdStr = hint.hintId ? `"${hint.hintId}"` : `"hint_${i}"`;
      lines.push(`    // usePredictHint: ${hintIdStr}`);
      lines.push(`    private string _hintId_${i} = ${hintIdStr};`);
      lines.push('');
    }
  }

  // Client-computed properties (from external libraries)
  const clientComputedVars = component.localVariables.filter(v => v.isClientComputed);
  if (clientComputedVars.length > 0) {
    lines.push('    // Client-computed properties (external libraries)');
    for (const clientVar of clientComputedVars) {
      const csharpType = inferCSharpTypeFromInit(clientVar.init);
      lines.push(`    [ClientComputed("${clientVar.name}")]`);
      lines.push(`    private ${csharpType} ${clientVar.name} => GetClientState<${csharpType}>("${clientVar.name}", default);`);
      lines.push('');
    }
  }

  // Server Task methods (useServerTask)
  const serverTaskMethods = generateServerTaskMethods(component);
  for (const line of serverTaskMethods) {
    lines.push(line);
  }

  // Render method (or RenderContent for templates)
  const renderMethodName = component.useTemplate ? 'RenderContent' : 'Render';
  lines.push(`    protected override VNode ${renderMethodName}()`);
  lines.push('    {');

  // Only add StateManager sync if NOT using a template (templates handle this themselves)
  if (!component.useTemplate) {
    lines.push('        StateManager.SyncMembersToState(this);');
    lines.push('');
  }

  // MVC State local variables - read from State dictionary
  if (component.useMvcState && component.useMvcState.length > 0) {
    lines.push('        // MVC State - read from State dictionary');
    for (const mvcState of component.useMvcState) {
      const csharpType = mvcState.type !== 'object' ? mvcState.type : 'dynamic';
      // Use propertyName (e.g., 'initialQuantity') not variable name (e.g., 'quantity')
      lines.push(`        var ${mvcState.name} = GetState<${csharpType}>("${mvcState.propertyName}");`);
    }
    lines.push('');
  }

  // Local variables (exclude client-computed ones, they're properties now)
  const regularLocalVars = component.localVariables.filter(v => !v.isClientComputed);
  for (const localVar of regularLocalVars) {
    lines.push(`        ${localVar.type} ${localVar.name} = ${localVar.initialValue};`);
  }
  if (regularLocalVars.length > 0) {
    lines.push('');
  }

  if (component.renderBody) {
    const renderCode = generateRenderBody(component.renderBody, component, 2);
    lines.push(renderCode);
  } else {
    lines.push('        return new VText("");');
  }

  lines.push('    }');

  // Effect methods (useEffect)
  let effectIndex = 0;
  for (const effect of component.useEffect) {
    lines.push('');

    // Extract dependency names from array
    const deps = [];
    if (effect.dependencies && t.isArrayExpression(effect.dependencies)) {
      for (const dep of effect.dependencies.elements) {
        if (t.isIdentifier(dep)) {
          deps.push(dep.name);
        }
      }
    }

    // Generate [OnStateChanged] for each dependency, or [OnMounted] if no dependencies
    if (deps.length === 0 && effect.dependencies && t.isArrayExpression(effect.dependencies) && effect.dependencies.elements.length === 0) {
      // Empty dependency array [] means run only on mount
      lines.push(`    [OnMounted]`);
    } else if (deps.length > 0) {
      // Run when these dependencies change
      for (const dep of deps) {
        lines.push(`    [OnStateChanged("${dep}")]`);
      }
    }
    // If no dependency array provided (undefined), no attribute needed - runs on every render

    lines.push(`    private void Effect_${effectIndex}()`);
    lines.push('    {');

    // Extract and convert effect body
    if (effect.body && t.isArrowFunctionExpression(effect.body)) {
      const body = effect.body.body;
      if (t.isBlockStatement(body)) {
        // Multi-statement effect
        for (const stmt of body.body) {
          lines.push(`        ${generateCSharpStatement(stmt)}`);
        }
      } else {
        // Single expression effect
        lines.push(`        ${generateCSharpExpression(body)};`);
      }
    }

    lines.push('    }');
    effectIndex++;
  }

  // Event handlers
  for (const handler of component.eventHandlers) {
    lines.push('');

    // Generate parameter list
    const params = handler.params || [];
    let paramList = params.length > 0
      ? params.map(p => t.isIdentifier(p) ? `dynamic ${p.name}` : 'dynamic arg')
      : [];

    // Add captured parameters from .map() context (e.g., item, index)
    const capturedParams = handler.capturedParams || [];
    if (capturedParams.length > 0) {
      paramList = paramList.concat(capturedParams.map(p => `dynamic ${p}`));
    }

    const paramStr = paramList.join(', ');

    // Event handlers must be public so SignalR hub can call them
    // Use async Task if handler contains await
    const returnType = handler.isAsync ? 'async Task' : 'void';
    lines.push(`    public ${returnType} ${handler.name}(${paramStr})`);
    lines.push('    {');

    // Check if this is a curried function error
    if (handler.isCurriedError) {
      lines.push(`        throw new InvalidOperationException(`);
      lines.push(`            "Event handler '${handler.name}' returns a function instead of executing an action. " +`);
      lines.push(`            "This is a curried function pattern (e.g., (e) => (id) => action(id)) which is invalid for event handlers. " +`);
      lines.push(`            "The returned function is never called by the event system. " +`);
      lines.push(`            "Fix: Use (e) => action(someValue) or create a properly bound handler."`);
      lines.push(`        );`);
    }
    // Generate method body
    else if (handler.body) {
      if (t.isBlockStatement(handler.body)) {
        // Block statement: { ... }
        for (const statement of handler.body.body) {
          const csharpStmt = generateCSharpStatement(statement);
          if (csharpStmt) {
            lines.push(`        ${csharpStmt}`);
          }
        }
      } else {
        // Expression body: () => expression
        const csharpExpr = generateCSharpExpression(handler.body);
        lines.push(`        ${csharpExpr};`);
      }
    }

    lines.push('    }');
  }

  // Toggle methods (useToggle)
  for (const toggle of component.useToggle) {
    lines.push('');
    lines.push(`    private void ${toggle.toggleFunc}()`);
    lines.push('    {');
    lines.push(`        ${toggle.name} = !${toggle.name};`);
    lines.push(`        SetState("${toggle.name}", ${toggle.name});`);
    lines.push('    }');
  }

  // GetClientHandlers method - returns JavaScript code for client-only event handlers
  if (component.clientHandlers && component.clientHandlers.length > 0) {
    lines.push('');
    lines.push('    /// <summary>');
    lines.push('    /// Returns JavaScript event handlers for client-side execution');
    lines.push('    /// These execute in the browser with bound hook context');
    lines.push('    /// </summary>');
    lines.push('    protected override Dictionary<string, string> GetClientHandlers()');
    lines.push('    {');
    lines.push('        return new Dictionary<string, string>');
    lines.push('        {');

    for (let i = 0; i < component.clientHandlers.length; i++) {
      const handler = component.clientHandlers[i];
      // Escape the JavaScript code for C# string literal
      const escapedJs = handler.jsCode
        .replace(/\\/g, '\\\\')  // Escape backslashes
        .replace(/"/g, '\\"')    // Escape quotes
        .replace(/\n/g, '\\n')   // Escape newlines
        .replace(/\r/g, '');     // Remove carriage returns

      const comma = i < component.clientHandlers.length - 1 ? ',' : '';
      lines.push(`            ["${handler.name}"] = @"${escapedJs}"${comma}`);
    }

    lines.push('        };');
    lines.push('    }');
  }

  // 🔥 NEW: GetClientEffects method - returns JavaScript effect callbacks
  if (component.clientEffects && component.clientEffects.length > 0) {
    lines.push('');
    lines.push('    /// <summary>');
    lines.push('    /// Returns JavaScript callbacks for useEffect hooks');
    lines.push('    /// These execute in the browser with bound hook context');
    lines.push('    /// </summary>');
    lines.push('    protected override Dictionary<string, EffectDefinition> GetClientEffects()');
    lines.push('    {');
    lines.push('        return new Dictionary<string, EffectDefinition>');
    lines.push('        {');

    for (let i = 0; i < component.clientEffects.length; i++) {
      const effect = component.clientEffects[i];

      // Escape JavaScript for C# string literal
      const escapedJs = effect.jsCode
        .replace(/\\/g, '\\\\')
        .replace(/"/g, '\\"')
        .replace(/\n/g, '\\n')
        .replace(/\r/g, '');

      // Extract dependency names from array
      const deps = [];
      if (effect.dependencies && effect.dependencies.elements) {
        for (const dep of effect.dependencies.elements) {
          if (dep && dep.name) {
            deps.push(`"${dep.name}"`);
          }
        }
      }
      const depsArray = deps.length > 0 ? deps.join(', ') : '';

      const comma = i < component.clientEffects.length - 1 ? ',' : '';

      lines.push(`            ["${effect.name}"] = new EffectDefinition`);
      lines.push(`            {`);
      lines.push(`                Callback = @"${escapedJs}",`);
      lines.push(`                Dependencies = new[] { ${depsArray} }`);
      lines.push(`            }${comma}`);
    }

    lines.push('        };');
    lines.push('    }');
  }

  // MVC State setter methods (useMvcState)
  // MVC State setter methods - REMOVED
  // These are now generated at the end of the class (after event handlers)
  // with the correct property names from the ViewModel (not variable names)

  // Pub/Sub methods (usePub)
  if (component.usePub) {
    for (const pub of component.usePub) {
      lines.push('');
      lines.push(`    // Publish to ${pub.name}_channel`);
      lines.push(`    private void ${pub.name}(dynamic value, PubSubOptions? options = null)`);
      lines.push('    {');
      lines.push(`        EventAggregator.Instance.Publish(${pub.name}_channel, value, options);`);
      lines.push('    }');
    }
  }

  // Pub/Sub methods (useSub)
  if (component.useSub) {
    for (const sub of component.useSub) {
      lines.push('');
      lines.push(`    // Subscribe to ${sub.name}_channel`);
      lines.push(`    protected override void OnInitialized()`);
      lines.push('    {');
      lines.push(`        base.OnInitialized();`);
      lines.push(`        `);
      lines.push(`        // Subscribe to ${sub.name}_channel`);
      lines.push(`        EventAggregator.Instance.Subscribe(${sub.name}_channel, (msg) => {`);
      lines.push(`            ${sub.name}_value = msg.Value;`);
      lines.push(`            SetState("${sub.name}_value", ${sub.name}_value);`);
      lines.push(`        });`);
      lines.push('    }');
    }
  }

  // SignalR methods (useSignalR)
  if (component.useSignalR) {
    for (const signalR of component.useSignalR) {
      lines.push('');
      lines.push(`    // SignalR send method for ${signalR.name}`);
      lines.push(`    // Note: useSignalR is primarily client-side.`);
      lines.push(`    // Server-side SignalR invocation can use HubContext directly if needed.`);
      lines.push(`    private async Task ${signalR.name}_send(string methodName, params object[] args)`);
      lines.push('    {');
      lines.push(`        if (HubContext != null && ConnectionId != null)`);
      lines.push(`        {`);
      lines.push(`            // Send message to specific client connection`);
      lines.push(`            await HubContext.Clients.Client(ConnectionId).SendAsync(methodName, args);`);
      lines.push(`        }`);
      lines.push('    }');
    }
  }

  // MVC State setter methods
  if (component.useMvcState) {
    for (const mvcState of component.useMvcState) {
      if (mvcState.setter) {
        const csharpType = mvcState.type !== 'object' ? mvcState.type : 'dynamic';
        lines.push('');
        lines.push(`    private void ${mvcState.setter}(${csharpType} value)`);
        lines.push('    {');
        lines.push(`        SetState("${mvcState.propertyName}", value);`);
        lines.push('    }');
      }
    }
  }

  // OnInitialized method for Razor Markdown initialization
  if (component.useRazorMarkdown && component.useRazorMarkdown.length > 0) {
    const { convertRazorMarkdownToCSharp } = require('./razorMarkdown.cjs');

    lines.push('');
    lines.push('    protected override void OnInitialized()');
    lines.push('    {');
    lines.push('        base.OnInitialized();');
    lines.push('');

    for (const md of component.useRazorMarkdown) {
      // Convert Razor markdown to C# string interpolation
      const csharpMarkdown = convertRazorMarkdownToCSharp(md.initialValue);
      lines.push(`        ${md.name} = ${csharpMarkdown};`);
    }

    lines.push('    }');
  }

  // Helper functions (function declarations in component body)
  if (component.helperFunctions && component.helperFunctions.length > 0) {
    for (const func of component.helperFunctions) {
      // Skip custom hooks (they're generated as separate [Hook] classes)
      if (func.name && func.name.startsWith('use') && func.params && func.params.length > 0) {
        const firstParam = func.params[0];
        if (firstParam.name === 'namespace') {
          continue; // This is a custom hook, skip it
        }
      }

      lines.push('');

      const returnType = func.isAsync
        ? (func.returnType === 'void' ? 'async Task' : `async Task<${func.returnType}>`)
        : func.returnType;

      const params = (func.params || []).map(p => `${p.type} ${p.name}`).join(', ');

      lines.push(`    private ${returnType} ${func.name}(${params})`);
      lines.push('    {');

      // Generate function body
      if (func.body && t.isBlockStatement(func.body)) {
        for (const statement of func.body.body) {
          const stmtCode = generateCSharpStatement(statement, 2);
          lines.push(stmtCode);
        }
      }

      lines.push('    }');
    }
  }

  // Helper functions (standalone functions referenced by component)
  if (component.topLevelHelperFunctions && component.topLevelHelperFunctions.length > 0) {
    for (const helper of component.topLevelHelperFunctions) {
      // Skip custom hooks (they're generated as separate [Hook] classes)
      if (helper.name && helper.name.startsWith('use') && helper.node && helper.node.params && helper.node.params.length > 0) {
        const firstParam = helper.node.params[0];
        if (firstParam && firstParam.name === 'namespace') {
          continue; // This is a custom hook, skip it
        }
      }

      lines.push('');
      lines.push(`    // Helper function: ${helper.name}`);

      // Generate the function signature
      const func = helper.node;
      const params = (func.params || []).map(p => {
        // Get parameter type from TypeScript annotation
        let paramType = 'dynamic';
        if (p.typeAnnotation && p.typeAnnotation.typeAnnotation) {
          paramType = tsTypeToCSharpType(p.typeAnnotation.typeAnnotation);
        }
        return `${paramType} ${p.name}`;
      }).join(', ');

      // Get return type from TypeScript annotation
      let returnType = 'dynamic';
      if (func.returnType && func.returnType.typeAnnotation) {
        returnType = tsTypeToCSharpType(func.returnType.typeAnnotation);
      }

      lines.push(`    private static ${returnType} ${helper.name}(${params})`);
      lines.push('    {');

      // Generate function body
      if (t.isBlockStatement(func.body)) {
        for (const statement of func.body.body) {
          const csharpStmt = generateCSharpStatement(statement);
          if (csharpStmt) {
            lines.push(`        ${csharpStmt}`);
          }
        }
      } else {
        // Expression body (arrow function)
        const csharpExpr = generateCSharpExpression(func.body);
        lines.push(`        return ${csharpExpr};`);
      }

      lines.push('    }');
    }
  }

  lines.push('}');

  return lines;
}

/**
 * Infer C# type from JavaScript AST node (for client-computed variables)
 */
function inferCSharpTypeFromInit(node) {
  if (!node) return 'dynamic';

  // Array types
  if (t.isArrayExpression(node)) {
    return 'List<dynamic>';
  }

  // Call expressions - try to infer from method name
  if (t.isCallExpression(node)) {
    const callee = node.callee;

    if (t.isMemberExpression(callee) && t.isIdentifier(callee.property)) {
      const method = callee.property.name;

      // Common array methods return arrays
      if (['map', 'filter', 'sort', 'sortBy', 'orderBy', 'slice', 'concat'].includes(method)) {
        return 'List<dynamic>';
      }

      // Aggregation methods return numbers
      if (['reduce', 'sum', 'sumBy', 'mean', 'meanBy', 'average', 'count', 'size'].includes(method)) {
        return 'double';
      }

      // Find methods return single item
      if (['find', 'minBy', 'maxBy', 'first', 'last'].includes(method)) {
        return 'dynamic';
      }

      // String methods
      if (['format', 'toString', 'join'].includes(method)) {
        return 'string';
      }
    }

    // Direct function calls (moment(), _.chain(), etc.)
    return 'dynamic';
  }

  // String operations
  if (t.isTemplateLiteral(node) || t.isStringLiteral(node)) {
    return 'string';
  }

  // Numbers
  if (t.isNumericLiteral(node)) {
    return 'double';
  }

  // Booleans
  if (t.isBooleanLiteral(node)) {
    return 'bool';
  }

  // Binary expressions - try to infer from operation
  if (t.isBinaryExpression(node)) {
    if (['+', '-', '*', '/', '%'].includes(node.operator)) {
      return 'double';
    }
    if (['==', '===', '!=', '!==', '<', '>', '<=', '>='].includes(node.operator)) {
      return 'bool';
    }
  }

  // Logical expressions
  if (t.isLogicalExpression(node)) {
    return 'bool';
  }

  // Default to dynamic
  return 'dynamic';
}

module.exports = {
  generateComponent,
  inferCSharpTypeFromInit
};
