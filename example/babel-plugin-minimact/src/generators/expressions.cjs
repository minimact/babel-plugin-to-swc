/**
 * Expression Generators
 */

const t = require('@babel/types');
const { escapeCSharpString } = require('../utils/helpers.cjs');
const { analyzeDependencies } = require('../analyzers/dependencies.cjs');
const { classifyNode } = require('../analyzers/classification.cjs');
const { generateRuntimeHelperForJSXNode } = require('./runtimeHelpers.cjs');
const { generateJSXElement } = require('./jsx.cjs');
const { getPathFromNode } = require('../utils/pathAssignment.cjs');

// Module-level variable to store current component context
// This allows useState setter detection without threading component through all calls
let currentComponent = null;

/**
 * Convert C# string truthy checks to proper boolean expressions
 * Wraps non-boolean expressions with MinimactHelpers.ToBool()
 */
function convertStringToBool(csharpTest, originalTestNode) {
  // If it's already a boolean expression, return as-is
  if (t.isBinaryExpression(originalTestNode) ||
      t.isLogicalExpression(originalTestNode) ||
      t.isUnaryExpression(originalTestNode, { operator: '!' })) {
    return csharpTest;
  }

  // For any other expression, wrap with ToBool() helper
  // This will handle strings, numbers, objects, etc. at runtime
  return `MinimactHelpers.ToBool(${csharpTest})`;
}

/**
 * Generate expression for use in boolean context (conditionals, logical operators)
 * Wraps expressions in MObject for JavaScript truthiness semantics
 */
function generateBooleanExpression(expr) {
  const generated = generateCSharpExpression(expr);

  // Check if this is a member expression on dynamic object (like user.isAdmin)
  if (t.isMemberExpression(expr) && !expr.computed && t.isIdentifier(expr.object)) {
    // Wrap dynamic member access in MObject for proper truthiness
    return `new MObject(${generated})`;
  }

  // Check if this is a simple identifier that might be dynamic
  if (t.isIdentifier(expr)) {
    // Wrap in MObject for null/truthiness handling
    return `new MObject(${generated})`;
  }

  // For other expressions (literals, etc.), use as-is
  return generated;
}

/**
 * Generate JSX expression (e.g., {count}, {user.name})
 */
function generateJSXExpression(expr, component, indent) {
  // Analyze dependencies
  const deps = analyzeDependencies(expr, component);
  const zone = classifyNode(deps);

  // For hybrid zones, we need to split
  if (zone === 'hybrid') {
    return generateHybridExpression(expr, component, deps, indent);
  }

  // Add zone attribute if needed
  const zoneAttr = zone === 'client'
    ? 'data-minimact-client-scope'
    : zone === 'server'
      ? 'data-minimact-server-scope'
      : '';

  // Handle special JSX expression types
  if (t.isConditionalExpression(expr)) {
    // Ternary with JSX: condition ? <A/> : <B/>
    // Force runtime helpers for JSX in conditionals
    const condition = generateBooleanExpression(expr.test);
    const consequent = t.isJSXElement(expr.consequent) || t.isJSXFragment(expr.consequent)
      ? generateRuntimeHelperForJSXNode(expr.consequent, component, indent)
      : generateCSharpExpression(expr.consequent, false); // Normal C# expression context

    // Handle alternate - if null literal, use VNull with path
    let alternate;
    if (!expr.alternate || t.isNullLiteral(expr.alternate)) {
      const exprPath = expr.__minimactPath || '';
      alternate = `new VNull("${exprPath}")`;
    } else if (t.isJSXElement(expr.alternate) || t.isJSXFragment(expr.alternate)) {
      alternate = generateRuntimeHelperForJSXNode(expr.alternate, component, indent);
    } else {
      alternate = generateCSharpExpression(expr.alternate, false); // Normal C# expression context
    }

    return `(${condition}) ? ${consequent} : ${alternate}`;
  }

  if (t.isLogicalExpression(expr) && expr.operator === '&&') {
    // Short-circuit with JSX: condition && <Element/>
    // Force runtime helpers for JSX in logical expressions
    const left = generateBooleanExpression(expr.left);
    const right = t.isJSXElement(expr.right) || t.isJSXFragment(expr.right)
      ? generateRuntimeHelperForJSXNode(expr.right, component, indent)
      : generateCSharpExpression(expr.right);
    // Get path for VNull (use the expression container's path)
    const exprPath = expr.__minimactPath || '';
    return `(${left}) ? ${right} : new VNull("${exprPath}")`;
  }

  if (t.isCallExpression(expr) &&
      t.isMemberExpression(expr.callee) &&
      t.isIdentifier(expr.callee.property, { name: 'map' })) {
    // Array.map() with JSX callback
    return generateMapExpression(expr, component, indent);
  }

  // Generate C# expression
  return generateCSharpExpression(expr);
}

/**
 * Generate conditional (ternary)
 */
function generateConditional(node, component, indent) {
  const indentStr = '    '.repeat(indent);
  const condition = generateCSharpExpression(node.test);
  const consequent = generateJSXElement(node.consequent, component, indent);
  const alternate = generateJSXElement(node.alternate, component, indent);

  return `${indentStr}return ${condition}\n${indentStr}    ? ${consequent}\n${indentStr}    : ${alternate};`;
}

/**
 * Generate short-circuit (&&)
 */
function generateShortCircuit(node, component, indent) {
  const indentStr = '    '.repeat(indent);
  const condition = generateCSharpExpression(node.left);
  const element = generateJSXElement(node.right, component, indent);

  return `${indentStr}if (${condition})\n${indentStr}{\n${indentStr}    return ${element};\n${indentStr}}\n${indentStr}return new VText("");`;
}

/**
 * Generate .map() expression
 */
function generateMapExpression(node, component, indent) {
  const indentStr = '    '.repeat(indent);
  const array = node.callee.object;
  const callback = node.arguments[0];

  const arrayName = array.name || generateCSharpExpression(array);
  const itemParam = callback.params[0].name;
  const indexParam = callback.params[1] ? callback.params[1].name : null;
  const body = callback.body;

  // Track map context for event handler closure capture (nested maps)
  const previousMapContext = component ? component.currentMapContext : null;
  const previousParams = previousMapContext ? previousMapContext.params : [];
  const currentParams = indexParam ? [itemParam, indexParam] : [itemParam];
  if (component) {
    component.currentMapContext = { params: [...previousParams, ...currentParams] };
  }

  let itemCode;
  let hasBlockStatements = false;

  if (t.isJSXElement(body)) {
    // Direct JSX return: item => <div>...</div>
    itemCode = generateJSXElement(body, component, indent + 1);
  } else if (t.isBlockStatement(body)) {
    // Block statement: item => { const x = ...; return <div>...</div>; }
    // Need to generate a statement lambda in C#
    hasBlockStatements = true;

    const statements = [];
    let returnJSX = null;

    // Process all statements in the block
    for (const stmt of body.body) {
      if (t.isReturnStatement(stmt) && t.isJSXElement(stmt.argument)) {
        returnJSX = stmt.argument;
        // Don't add return statement to statements array yet
      } else if (t.isVariableDeclaration(stmt)) {
        // Convert variable declarations: const displayValue = item[field];
        for (const decl of stmt.declarations) {
          const varName = decl.id.name;
          const init = decl.init ? generateCSharpExpression(decl.init) : 'null';
          statements.push(`var ${varName} = ${init};`);
        }
      } else {
        // Other statements - convert them
        statements.push(generateCSharpStatement(stmt));
      }
    }

    if (!returnJSX) {
      console.error('[generateMapExpression] Block statement has no JSX return');
      throw new Error('Map callback with block statement must return JSX element');
    }

    const jsxCode = generateJSXElement(returnJSX, component, indent + 1);
    statements.push(`return ${jsxCode};`);

    itemCode = statements.join(' ');
  } else {
    console.error('[generateMapExpression] Unsupported callback body type:', body?.type);
    throw new Error(`Unsupported map callback body type: ${body?.type}`);
  }

  // Restore previous context
  if (component) {
    component.currentMapContext = previousMapContext;
  }

  // Check if array is dynamic (likely from outer .map())
  const needsCast = arrayName.includes('.') && !arrayName.match(/^[A-Z]/); // Property access, not static class
  const castedArray = needsCast ? `((IEnumerable<dynamic>)${arrayName})` : arrayName;

  // C# Select supports (item, index) => ...
  if (hasBlockStatements) {
    // Use statement lambda: item => { statements; return jsx; }
    if (indexParam) {
      const lambdaExpr = `(${itemParam}, ${indexParam}) => { ${itemCode} }`;
      const castedLambda = needsCast ? `(Func<dynamic, int, dynamic>)(${lambdaExpr})` : lambdaExpr;
      return `${castedArray}.Select(${castedLambda}).ToArray()`;
    } else {
      const lambdaExpr = `${itemParam} => { ${itemCode} }`;
      const castedLambda = needsCast ? `(Func<dynamic, dynamic>)(${lambdaExpr})` : lambdaExpr;
      return `${castedArray}.Select(${castedLambda}).ToArray()`;
    }
  } else {
    // Use expression lambda: item => jsx
    if (indexParam) {
      const lambdaExpr = `(${itemParam}, ${indexParam}) => ${itemCode}`;
      const castedLambda = needsCast ? `(Func<dynamic, int, dynamic>)(${lambdaExpr})` : lambdaExpr;
      return `${castedArray}.Select(${castedLambda}).ToArray()`;
    } else {
      const lambdaExpr = `${itemParam} => ${itemCode}`;
      const castedLambda = needsCast ? `(Func<dynamic, dynamic>)(${lambdaExpr})` : lambdaExpr;
      return `${castedArray}.Select(${castedLambda}).ToArray()`;
    }
  }
}

/**
 * Generate C# statement from JavaScript AST node
 */
function generateCSharpStatement(node) {
  if (!node) return '';

  if (t.isExpressionStatement(node)) {
    return generateCSharpExpression(node.expression) + ';';
  }

  if (t.isReturnStatement(node)) {
    // Handle empty return statement: return; (not return null;)
    if (node.argument === null || node.argument === undefined) {
      return 'return;';
    }
    return `return ${generateCSharpExpression(node.argument)};`;
  }

  if (t.isThrowStatement(node)) {
    return `throw ${generateCSharpExpression(node.argument)};`;
  }

  if (t.isVariableDeclaration(node)) {
    const declarations = node.declarations.map(d => {
      const name = d.id.name;
      const value = generateCSharpExpression(d.init);
      return `var ${name} = ${value};`;
    }).join(' ');
    return declarations;
  }

  if (t.isIfStatement(node)) {
    let test = generateCSharpExpression(node.test);

    // Convert string truthy checks to proper C# boolean expressions
    test = convertStringToBool(test, node.test);

    let result = `if (${test}) {\n`;

    // Handle consequent (then branch)
    if (t.isBlockStatement(node.consequent)) {
      for (const stmt of node.consequent.body) {
        result += '    ' + generateCSharpStatement(stmt) + '\n';
      }
    } else {
      result += '    ' + generateCSharpStatement(node.consequent) + '\n';
    }

    result += '}';

    // Handle alternate (else branch) if it exists
    if (node.alternate) {
      result += ' else {\n';
      if (t.isBlockStatement(node.alternate)) {
        for (const stmt of node.alternate.body) {
          result += '    ' + generateCSharpStatement(stmt) + '\n';
        }
      } else if (t.isIfStatement(node.alternate)) {
        // else if
        result += '    ' + generateCSharpStatement(node.alternate) + '\n';
      } else {
        result += '    ' + generateCSharpStatement(node.alternate) + '\n';
      }
      result += '}';
    }

    return result;
  }

  if (t.isTryStatement(node)) {
    let result = 'try {\n';

    // Handle try block
    if (t.isBlockStatement(node.block)) {
      for (const stmt of node.block.body) {
        result += '    ' + generateCSharpStatement(stmt) + '\n';
      }
    }

    result += '}';

    // Handle catch clause
    if (node.handler) {
      const catchParam = node.handler.param ? node.handler.param.name : 'ex';
      result += ` catch (Exception ${catchParam}) {\n`;

      if (t.isBlockStatement(node.handler.body)) {
        for (const stmt of node.handler.body.body) {
          result += '    ' + generateCSharpStatement(stmt) + '\n';
        }
      }

      result += '}';
    }

    // Handle finally block
    if (node.finalizer) {
      result += ' finally {\n';

      if (t.isBlockStatement(node.finalizer)) {
        for (const stmt of node.finalizer.body) {
          result += '    ' + generateCSharpStatement(stmt) + '\n';
        }
      }

      result += '}';
    }

    return result;
  }

  // Fallback: try to convert as expression
  return generateCSharpExpression(node) + ';';
}

/**
 * Generate C# expression from JS expression
 * @param {boolean} inInterpolation - True if this expression will be inside $"{...}"
 */
function generateCSharpExpression(node, inInterpolation = false) {
  if (!node) {
    const nodePath = node?.__minimactPath || '';
    return `new VNull("${nodePath}")`;
  }

  if (t.isStringLiteral(node)) {
    // In string interpolation context, escape the quotes: \"text\"
    // Otherwise use normal quotes: "text"
    if (inInterpolation) {
      return `\\"${escapeCSharpString(node.value)}\\"`;
    } else {
      return `"${escapeCSharpString(node.value)}"`;
    }
  }

  if (t.isNumericLiteral(node)) {
    return String(node.value);
  }

  if (t.isBooleanLiteral(node)) {
    return node.value ? 'true' : 'false';
  }

  if (t.isNullLiteral(node)) {
    const nodePath = node.__minimactPath || '';
    return `new VNull("${nodePath}")`;
  }

  if (t.isIdentifier(node)) {
    // Special case: 'state' identifier (state proxy)
    // Note: This should only happen as part of member expression (state.key or state["key"])
    // Standalone 'state' reference is unusual - warn but transpile to 'State'
    if (node.name === 'state') {
      console.warn('[Babel Plugin] Naked state reference detected (should be state.key or state["key"])');
      return 'State';
    }

    // 🔥 NEW: Check if this identifier is a custom hook return value
    if (currentComponent && currentComponent.customHooks) {
      for (const hookInstance of currentComponent.customHooks) {
        if (!hookInstance.metadata || !hookInstance.metadata.returnValues) continue;

        // Check if this identifier matches any non-UI return value
        const returnValueIndex = hookInstance.returnValues.indexOf(node.name);
        if (returnValueIndex !== -1) {
          const returnValueMetadata = hookInstance.metadata.returnValues[returnValueIndex];

          // Skip UI return values (they're handled separately as VComponentWrapper)
          if (returnValueMetadata.type === 'jsx') {
            return node.name; // Keep as-is for UI variables
          }

          // Replace with lifted state access for state/method returns
          if (returnValueMetadata.type === 'state') {
            // Access the hook's lifted state: State["hookNamespace.stateVarName"]
            const liftedStatePath = `${hookInstance.namespace}.${returnValueMetadata.name}`;
            return `GetState<dynamic>("${liftedStatePath}")`;
          } else if (returnValueMetadata.type === 'method') {
            // For methods, we can't call them from parent - warn and keep as-is for now
            // In the future, we could emit a method that invokes the child component's method
            console.warn(`[Custom Hook] Cannot access method '${node.name}' from hook '${hookInstance.hookName}' in parent component`);
            return node.name;
          }
        }
      }
    }

    return node.name;
  }

  if (t.isAssignmentExpression(node)) {
    const left = generateCSharpExpression(node.left, inInterpolation);
    const right = generateCSharpExpression(node.right, inInterpolation);
    const operator = node.operator; // =, +=, -=, etc.
    return `${left} ${operator} ${right}`;
  }

  if (t.isAwaitExpression(node)) {
    return `await ${generateCSharpExpression(node.argument, inInterpolation)}`;
  }

  // Handle TypeScript type assertions: (e.target as any) → e.target (strip the cast)
  // In C#, we rely on dynamic typing, so type casts are usually unnecessary
  if (t.isTSAsExpression(node)) {
    return generateCSharpExpression(node.expression, inInterpolation);
  }

  // Handle TypeScript type assertions (angle bracket syntax): <any>e.target → e.target
  if (t.isTSTypeAssertion(node)) {
    return generateCSharpExpression(node.expression, inInterpolation);
  }

  // Handle optional chaining: viewModel?.userEmail → viewModel?.UserEmail
  if (t.isOptionalMemberExpression(node)) {
    const object = generateCSharpExpression(node.object, inInterpolation);
    const propertyName = t.isIdentifier(node.property) ? node.property.name : null;

    // Capitalize first letter for C# property convention (userEmail → UserEmail)
    const csharpProperty = propertyName
      ? propertyName.charAt(0).toUpperCase() + propertyName.slice(1)
      : propertyName;

    const property = node.computed
      ? `?[${generateCSharpExpression(node.property, inInterpolation)}]`
      : `?.${csharpProperty}`;
    return `${object}${property}`;
  }

  if (t.isMemberExpression(node)) {
    // Special case: state.key or state["key"] (state proxy)
    if (t.isIdentifier(node.object, { name: 'state' })) {
      if (node.computed) {
        // state["someKey"] or state["Child.key"] → State["someKey"] or State["Child.key"]
        const key = generateCSharpExpression(node.property, inInterpolation);
        return `State[${key}]`;
      } else {
        // state.someKey → State["someKey"]
        const key = node.property.name;
        return `State["${key}"]`;
      }
    }

    const object = generateCSharpExpression(node.object);
    const propertyName = t.isIdentifier(node.property) ? node.property.name : null;

    // Handle ref.current → just ref (refs in C# are the value itself, not a container)
    if (propertyName === 'current' && !node.computed && t.isIdentifier(node.object)) {
      // Check if the object is a ref variable (ends with "Ref")
      if (node.object.name.endsWith('Ref')) {
        return object;  // Return just the ref variable name without .current
      }
    }

    // Handle JavaScript to C# API conversions
    if (propertyName === 'length' && !node.computed) {
      // array.length → array.Count
      return `${object}.Count`;
    }

    // Handle event object property access (e.target.value → e.Target.Value)
    if (propertyName === 'target' && !node.computed) {
      return `${object}.Target`;
    }
    if (propertyName === 'value' && !node.computed) {
      // Capitalize for C# property convention
      return `${object}.Value`;
    }
    if (propertyName === 'checked' && !node.computed) {
      // Capitalize for C# property convention
      return `${object}.Checked`;
    }

    // Handle exception properties (err.message → err.Message)
    if (propertyName === 'message' && !node.computed) {
      return `${object}.Message`;
    }

    // Handle fetch Response properties (response.ok → response.IsSuccessStatusCode)
    if (propertyName === 'ok' && !node.computed) {
      return `${object}.IsSuccessStatusCode`;
    }

    const property = node.computed
      ? `[${generateCSharpExpression(node.property)}]`
      : `.${propertyName}`;
    return `${object}${property}`;
  }

  if (t.isArrayExpression(node)) {
    // Check if array contains spread elements
    const hasSpread = node.elements.some(e => t.isSpreadElement(e));

    if (hasSpread) {
      // Handle spread operator: [...array, item] → array.Concat(new[] { item }).ToList()
      const parts = [];
      let currentLiteral = [];

      for (const element of node.elements) {
        if (t.isSpreadElement(element)) {
          // Flush current literal elements
          if (currentLiteral.length > 0) {
            const literalCode = currentLiteral.map(e => generateCSharpExpression(e)).join(', ');
            parts.push(`new[] { ${literalCode} }`);
            currentLiteral = [];
          }
          // Add spread array (cast to preserve type)
          parts.push(generateCSharpExpression(element.argument));
        } else {
          currentLiteral.push(element);
        }
      }

      // Flush remaining literals
      if (currentLiteral.length > 0) {
        const literalCode = currentLiteral.map(e => generateCSharpExpression(e)).join(', ');
        parts.push(`new[] { ${literalCode} }`);
      }

      // Combine with Concat
      if (parts.length === 1) {
        return `${parts[0]}.ToList()`;
      } else {
        const concats = parts.slice(1).map(p => `.Concat(${p})`).join('');
        return `${parts[0]}${concats}.ToList()`;
      }
    }

    // No spread - simple array literal
    const elements = node.elements.map(e => generateCSharpExpression(e)).join(', ');

    // Infer type from first element if all are string literals
    if (node.elements.length > 0 && node.elements.every(e => t.isStringLiteral(e))) {
      return `new List<string> { ${elements} }`;
    }

    // Use List<dynamic> for empty arrays to be compatible with dynamic LINQ results
    const listType = elements.length === 0 ? 'dynamic' : 'object';
    return `new List<${listType}> { ${elements} }`;
  }

  if (t.isUnaryExpression(node)) {
    // Handle unary expressions: !expr, -expr, +expr, etc.
    const argument = generateCSharpExpression(node.argument, inInterpolation);
    const operator = node.operator;
    return `${operator}${argument}`;
  }

  if (t.isBinaryExpression(node)) {
    // Helper function to get operator precedence (higher = tighter binding)
    const getPrecedence = (op) => {
      if (op === '*' || op === '/' || op === '%') return 3;
      if (op === '+' || op === '-') return 2;
      if (op === '==' || op === '!=' || op === '===' || op === '!==' ||
          op === '<' || op === '>' || op === '<=' || op === '>=') return 1;
      return 0;
    };

    const currentPrecedence = getPrecedence(node.operator);

    // Generate left side, wrap in parentheses if needed
    let left = generateCSharpExpression(node.left);
    if (t.isBinaryExpression(node.left)) {
      const leftPrecedence = getPrecedence(node.left.operator);
      // Wrap in parentheses if left has lower precedence
      if (leftPrecedence < currentPrecedence) {
        left = `(${left})`;
      }
    }

    // Generate right side, wrap in parentheses if needed
    let right = generateCSharpExpression(node.right);
    if (t.isBinaryExpression(node.right)) {
      const rightPrecedence = getPrecedence(node.right.operator);
      // Wrap in parentheses if right has lower or equal precedence
      // Equal precedence on right needs parens for left-associative operators
      if (rightPrecedence <= currentPrecedence) {
        right = `(${right})`;
      }
    }

    // Convert JavaScript operators to C# operators
    let operator = node.operator;
    if (operator === '===') operator = '==';
    if (operator === '!==') operator = '!=';
    return `${left} ${operator} ${right}`;
  }

  if (t.isLogicalExpression(node)) {
    const left = generateCSharpExpression(node.left);
    const right = generateCSharpExpression(node.right);

    if (node.operator === '||') {
      // JavaScript: a || b
      // C#: a ?? b (null coalescing)
      return `(${left}) ?? (${right})`;
    } else if (node.operator === '&&') {
      // Check if right side is a boolean expression (comparison, logical, etc.)
      const rightIsBooleanExpr = t.isBinaryExpression(node.right) ||
                                  t.isLogicalExpression(node.right) ||
                                  t.isUnaryExpression(node.right);

      if (rightIsBooleanExpr) {
        // JavaScript: a && (b > 0)
        // C#: (a) && (b > 0) - boolean AND
        return `(${left}) && (${right})`;
      } else {
        // JavaScript: a && <jsx> or a && someValue
        // C#: a != null ? value : VNull (for objects)
        const nodePath = node.__minimactPath || '';
        return `(${left}) != null ? (${right}) : new VNull("${nodePath}")`;
      }
    }

    return `${left} ${node.operator} ${right}`;
  }

  if (t.isConditionalExpression(node)) {
    // Handle ternary operator: test ? consequent : alternate
    // Children are always in normal C# expression context, not interpolation context
    const test = generateCSharpExpression(node.test, false);
    const consequent = generateCSharpExpression(node.consequent, false);
    const alternate = generateCSharpExpression(node.alternate, false);
    return `(${test}) ? ${consequent} : ${alternate}`;
  }

  if (t.isCallExpression(node)) {
    // Handle Math.max() → Math.Max()
    if (t.isMemberExpression(node.callee) &&
        t.isIdentifier(node.callee.object, { name: 'Math' }) &&
        t.isIdentifier(node.callee.property, { name: 'max' })) {
      const args = node.arguments.map(arg => generateCSharpExpression(arg)).join(', ');
      return `Math.Max(${args})`;
    }

    // Handle Math.min() → Math.Min()
    if (t.isMemberExpression(node.callee) &&
        t.isIdentifier(node.callee.object, { name: 'Math' }) &&
        t.isIdentifier(node.callee.property, { name: 'min' })) {
      const args = node.arguments.map(arg => generateCSharpExpression(arg)).join(', ');
      return `Math.Min(${args})`;
    }

    // Handle other Math methods (floor, ceil, round, pow, log, etc.) → Pascal case
    if (t.isMemberExpression(node.callee) &&
        t.isIdentifier(node.callee.object, { name: 'Math' })) {
      const methodName = node.callee.property.name;
      const pascalMethodName = methodName.charAt(0).toUpperCase() + methodName.slice(1);
      const args = node.arguments.map(arg => generateCSharpExpression(arg)).join(', ');

      // Cast floor/ceil/round to int for array indexing compatibility
      if (methodName === 'floor' || methodName === 'ceil' || methodName === 'round') {
        return `(int)Math.${pascalMethodName}(${args})`;
      }

      return `Math.${pascalMethodName}(${args})`;
    }

    // Handle encodeURIComponent() → Uri.EscapeDataString()
    if (t.isIdentifier(node.callee, { name: 'encodeURIComponent' })) {
      const args = node.arguments.map(arg => generateCSharpExpression(arg)).join(', ');
      return `Uri.EscapeDataString(${args})`;
    }

    // Handle setState(key, value) → SetState(key, value)
    // This is the compile-time state proxy function for lifted state
    if (t.isIdentifier(node.callee, { name: 'setState' })) {
      if (node.arguments.length >= 2) {
        const key = generateCSharpExpression(node.arguments[0]);
        const value = generateCSharpExpression(node.arguments[1]);
        return `SetState(${key}, ${value})`;
      } else {
        console.warn('[Babel Plugin] setState requires 2 arguments (key, value)');
        return `SetState("", null)`;
      }
    }

    // Handle fetch() → HttpClient call
    // Note: This generates a basic wrapper. Real implementation would use IHttpClientFactory
    if (t.isIdentifier(node.callee, { name: 'fetch' })) {
      const url = node.arguments.length > 0 ? generateCSharpExpression(node.arguments[0]) : '""';
      // Return HttpResponseMessage (await is handled by caller)
      return `new HttpClient().GetAsync(${url})`;
    }

    // Handle Promise.resolve(value) → Task.FromResult(value)
    if (t.isMemberExpression(node.callee) &&
        t.isIdentifier(node.callee.object, { name: 'Promise' }) &&
        t.isIdentifier(node.callee.property, { name: 'resolve' })) {
      if (node.arguments.length > 0) {
        const value = generateCSharpExpression(node.arguments[0]);
        return `Task.FromResult(${value})`;
      }
      return `Task.CompletedTask`;
    }

    // Handle Promise.reject(error) → Task.FromException(error)
    if (t.isMemberExpression(node.callee) &&
        t.isIdentifier(node.callee.object, { name: 'Promise' }) &&
        t.isIdentifier(node.callee.property, { name: 'reject' })) {
      if (node.arguments.length > 0) {
        const error = generateCSharpExpression(node.arguments[0]);
        return `Task.FromException(new Exception(${error}))`;
      }
    }

    // Handle alert() → Console.WriteLine() (or custom alert implementation)
    if (t.isIdentifier(node.callee, { name: 'alert' })) {
      const args = node.arguments.map(arg => generateCSharpExpression(arg)).join(' + ');
      return `Console.WriteLine(${args})`;
    }

    // Handle String(value) → value.ToString()
    if (t.isIdentifier(node.callee, { name: 'String' })) {
      if (node.arguments.length > 0) {
        const arg = generateCSharpExpression(node.arguments[0]);
        return `(${arg}).ToString()`;
      }
      return '""';
    }

    // Handle Object.keys() → dictionary.Keys or reflection for objects
    if (t.isMemberExpression(node.callee) &&
        t.isIdentifier(node.callee.object, { name: 'Object' }) &&
        t.isIdentifier(node.callee.property, { name: 'keys' })) {
      if (node.arguments.length > 0) {
        const obj = generateCSharpExpression(node.arguments[0]);
        // For dynamic objects, cast to IDictionary and get Keys
        return `((IDictionary<string, object>)${obj}).Keys`;
      }
    }

    // Handle Date.now() → DateTimeOffset.Now.ToUnixTimeMilliseconds()
    if (t.isMemberExpression(node.callee) &&
        t.isIdentifier(node.callee.object, { name: 'Date' }) &&
        t.isIdentifier(node.callee.property, { name: 'now' })) {
      return 'DateTimeOffset.Now.ToUnixTimeMilliseconds()';
    }

    // Handle console.log → Console.WriteLine
    if (t.isMemberExpression(node.callee) &&
        t.isIdentifier(node.callee.object, { name: 'console' }) &&
        t.isIdentifier(node.callee.property, { name: 'log' })) {
      const args = node.arguments.map(arg => generateCSharpExpression(arg)).join(' + ');
      return `Console.WriteLine(${args})`;
    }

    // Handle response.json() → response.Content.ReadFromJsonAsync<dynamic>()
    if (t.isMemberExpression(node.callee) && t.isIdentifier(node.callee.property, { name: 'json' })) {
      const object = generateCSharpExpression(node.callee.object);
      return `${object}.Content.ReadFromJsonAsync<dynamic>()`;
    }

    // Handle .toFixed(n) → .ToString("Fn")
    if (t.isMemberExpression(node.callee) && t.isIdentifier(node.callee.property, { name: 'toFixed' })) {
      let object = generateCSharpExpression(node.callee.object);

      // Preserve parentheses for complex expressions (binary operations, conditionals, etc.)
      // This ensures operator precedence is maintained: (price * quantity).toFixed(2) → (price * quantity).ToString("F2")
      if (t.isBinaryExpression(node.callee.object) ||
          t.isLogicalExpression(node.callee.object) ||
          t.isConditionalExpression(node.callee.object) ||
          t.isCallExpression(node.callee.object)) {
        object = `(${object})`;
      }

      const decimals = node.arguments.length > 0 && t.isNumericLiteral(node.arguments[0])
        ? node.arguments[0].value
        : 2;
      return `${object}.ToString("F${decimals}")`;
    }

    // Handle .toLocaleString() → .ToString("g") (DateTime)
    if (t.isMemberExpression(node.callee) && t.isIdentifier(node.callee.property, { name: 'toLocaleString' })) {
      const object = generateCSharpExpression(node.callee.object);
      return `${object}.ToString("g")`;
    }

    // Handle .toLowerCase() → .ToLower()
    if (t.isMemberExpression(node.callee) && t.isIdentifier(node.callee.property, { name: 'toLowerCase' })) {
      const object = generateCSharpExpression(node.callee.object);
      return `${object}.ToLower()`;
    }

    // Handle .toUpperCase() → .ToUpper()
    if (t.isMemberExpression(node.callee) && t.isIdentifier(node.callee.property, { name: 'toUpperCase' })) {
      const object = generateCSharpExpression(node.callee.object);
      return `${object}.ToUpper()`;
    }

    // Handle .trim() → .Trim()
    if (t.isMemberExpression(node.callee) && t.isIdentifier(node.callee.property, { name: 'trim' })) {
      const object = generateCSharpExpression(node.callee.object);
      return `${object}.Trim()`;
    }

    // Handle .substring(start, end) → .Substring(start, end)
    if (t.isMemberExpression(node.callee) && t.isIdentifier(node.callee.property, { name: 'substring' })) {
      const object = generateCSharpExpression(node.callee.object);
      const args = node.arguments.map(arg => generateCSharpExpression(arg)).join(', ');
      return `${object}.Substring(${args})`;
    }

    // Handle .padStart(length, char) → .PadLeft(length, char)
    if (t.isMemberExpression(node.callee) && t.isIdentifier(node.callee.property, { name: 'padStart' })) {
      const object = generateCSharpExpression(node.callee.object);
      const length = node.arguments[0] ? generateCSharpExpression(node.arguments[0]) : '0';
      let padChar = node.arguments[1] ? generateCSharpExpression(node.arguments[1]) : '" "';

      // Convert string literal "0" to char literal '0'
      if (t.isStringLiteral(node.arguments[1]) && node.arguments[1].value.length === 1) {
        padChar = `'${node.arguments[1].value}'`;
      }

      return `${object}.PadLeft(${length}, ${padChar})`;
    }

    // Handle .padEnd(length, char) → .PadRight(length, char)
    if (t.isMemberExpression(node.callee) && t.isIdentifier(node.callee.property, { name: 'padEnd' })) {
      const object = generateCSharpExpression(node.callee.object);
      const length = node.arguments[0] ? generateCSharpExpression(node.arguments[0]) : '0';
      let padChar = node.arguments[1] ? generateCSharpExpression(node.arguments[1]) : '" "';

      // Convert string literal "0" to char literal '0'
      if (t.isStringLiteral(node.arguments[1]) && node.arguments[1].value.length === 1) {
        padChar = `'${node.arguments[1].value}'`;
      }

      return `${object}.PadRight(${length}, ${padChar})`;
    }

    // Handle useState/useClientState setters → SetState calls
    if (t.isIdentifier(node.callee) && currentComponent) {
      const setterName = node.callee.name;

      // Check if this is a useState setter
      const useState = [...(currentComponent.useState || []), ...(currentComponent.useClientState || [])]
        .find(state => state.setter === setterName);

      if (useState && node.arguments.length > 0) {
        const newValue = generateCSharpExpression(node.arguments[0]);
        return `SetState(nameof(${useState.name}), ${newValue})`;
      }
    }

    // Handle .map() → .Select()
    if (t.isMemberExpression(node.callee) && t.isIdentifier(node.callee.property, { name: 'map' })) {
      const object = generateCSharpExpression(node.callee.object);
      if (node.arguments.length > 0) {
        const callback = node.arguments[0];
        if (t.isArrowFunctionExpression(callback)) {
          const paramNames = callback.params.map(p => p.name);
          // C# requires parentheses for 0 or 2+ parameters
          const params = paramNames.length === 1
            ? paramNames[0]
            : `(${paramNames.join(', ')})`;

          // Handle JSX in arrow function body
          let body;
          if (t.isBlockStatement(callback.body)) {
            body = `{ ${callback.body.body.map(stmt => generateCSharpStatement(stmt)).join(' ')} }`;
          } else if (t.isJSXElement(callback.body) || t.isJSXFragment(callback.body)) {
            // JSX element - use generateJSXElement with currentComponent context
            // Store map context for event handler closure capture
            // For nested maps, we need to ACCUMULATE params, not replace them
            const previousMapContext = currentComponent ? currentComponent.currentMapContext : null;
            const previousParams = previousMapContext ? previousMapContext.params : [];
            if (currentComponent) {
              // Combine previous params with current params for nested map support
              currentComponent.currentMapContext = { params: [...previousParams, ...paramNames] };
            }
            body = generateJSXElement(callback.body, currentComponent, 0);
            // Restore previous context
            if (currentComponent) {
              currentComponent.currentMapContext = previousMapContext;
            }
          } else {
            body = generateCSharpExpression(callback.body);
          }

          // Cast to IEnumerable<dynamic> if we detect dynamic access
          // Check for optional chaining or property access (likely dynamic)
          const needsCast = object.includes('?.') || object.includes('?') || object.includes('.');
          const castedObject = needsCast ? `((IEnumerable<dynamic>)${object})` : object;

          // If the object needs casting (is dynamic), we also need to cast the lambda
          // to prevent CS1977: "Cannot use a lambda expression as an argument to a dynamically dispatched operation"
          const lambdaExpr = `${params} => ${body}`;
          const castedLambda = needsCast ? `(Func<dynamic, dynamic>)(${lambdaExpr})` : lambdaExpr;

          return `${castedObject}.Select(${castedLambda}).ToList()`;
        }
      }
    }

    // Generic function call
    const callee = generateCSharpExpression(node.callee);
    const args = node.arguments.map(arg => generateCSharpExpression(arg)).join(', ');
    return `${callee}(${args})`;
  }

  if (t.isOptionalCallExpression(node)) {
    // Handle optional call: array?.map(...)
    // Check if this is .map() which needs to be converted to .Select()
    if (t.isOptionalMemberExpression(node.callee) &&
        t.isIdentifier(node.callee.property, { name: 'map' })) {
      const object = generateCSharpExpression(node.callee.object);
      if (node.arguments.length > 0) {
        const callback = node.arguments[0];
        if (t.isArrowFunctionExpression(callback)) {
          const paramNames = callback.params.map(p => p.name);
          // C# requires parentheses for 0 or 2+ parameters
          const params = paramNames.length === 1
            ? paramNames[0]
            : `(${paramNames.join(', ')})`;

          // Handle JSX in arrow function body
          let body;
          if (t.isBlockStatement(callback.body)) {
            body = `{ ${callback.body.body.map(stmt => generateCSharpStatement(stmt)).join(' ')} }`;
          } else if (t.isJSXElement(callback.body) || t.isJSXFragment(callback.body)) {
            // JSX element - use generateJSXElement with currentComponent context
            // Store map context for event handler closure capture
            // For nested maps, we need to ACCUMULATE params, not replace them
            const previousMapContext = currentComponent ? currentComponent.currentMapContext : null;
            const previousParams = previousMapContext ? previousMapContext.params : [];
            if (currentComponent) {
              // Combine previous params with current params for nested map support
              currentComponent.currentMapContext = { params: [...previousParams, ...paramNames] };
            }
            body = generateJSXElement(callback.body, currentComponent, 0);
            // Restore previous context
            if (currentComponent) {
              currentComponent.currentMapContext = previousMapContext;
            }
          } else {
            body = generateCSharpExpression(callback.body);
          }

          // Cast to IEnumerable<dynamic> for optional chaining (likely dynamic)
          const castedObject = `((IEnumerable<dynamic>)${object})`;

          // Cast result to List<dynamic> for ?? operator compatibility
          // Anonymous types from Select need explicit Cast<dynamic>() before ToList()
          return `${castedObject}?.Select(${params} => ${body})?.Cast<dynamic>().ToList()`;
        }
      }
    }

    // Generic optional call
    const callee = generateCSharpExpression(node.callee);
    const args = node.arguments.map(arg => generateCSharpExpression(arg)).join(', ');
    return `${callee}(${args})`;
  }

  if (t.isTemplateLiteral(node)) {
    // Convert template literal to C# string

    // If no expressions, use verbatim string literal (@"...") to avoid escaping issues
    if (node.expressions.length === 0) {
      const text = node.quasis[0].value.raw;
      // Use verbatim string literal (@"...") for multiline or strings with special chars
      // Escape " as "" in verbatim strings
      const escaped = text.replace(/"/g, '""');
      return `@"${escaped}"`;
    }

    // Has expressions - use C# string interpolation
    let result = '$"';
    for (let i = 0; i < node.quasis.length; i++) {
      // Escape special chars in C# interpolated strings
      let text = node.quasis[i].value.raw;
      // Escape { and } by doubling them
      text = text.replace(/{/g, '{{').replace(/}/g, '}}');
      // Escape " as \"
      text = text.replace(/"/g, '\\"');
      result += text;

      if (i < node.expressions.length) {
        const expr = node.expressions[i];
        // Wrap conditional (ternary) expressions in parentheses to avoid ':' conflict in C# interpolation
        const exprCode = generateCSharpExpression(expr);
        const needsParens = t.isConditionalExpression(expr);
        result += '{' + (needsParens ? `(${exprCode})` : exprCode) + '}';
      }
    }
    result += '"';
    return result;
  }

  if (t.isNewExpression(node)) {
    // Handle new Promise(resolve => setTimeout(resolve, ms)) → Task.Delay(ms)
    if (t.isIdentifier(node.callee, { name: 'Promise' }) && node.arguments.length > 0) {
      const callback = node.arguments[0];

      // Check if it's the setTimeout pattern
      if (t.isArrowFunctionExpression(callback) && callback.params.length === 1) {
        const resolveParam = callback.params[0].name;
        const body = callback.body;

        // Check if body is: setTimeout(resolve, ms)
        if (t.isCallExpression(body) &&
            t.isIdentifier(body.callee, { name: 'setTimeout' }) &&
            body.arguments.length === 2 &&
            t.isIdentifier(body.arguments[0], { name: resolveParam })) {
          const delay = generateCSharpExpression(body.arguments[1]);
          return `Task.Delay(${delay})`;
        }
      }

      // Generic Promise constructor - not directly supported in C#
      // Return Task.CompletedTask as a fallback
      return `Task.CompletedTask`;
    }

    // Handle new Date() → DateTime.Parse()
    if (t.isIdentifier(node.callee, { name: 'Date' })) {
      if (node.arguments.length === 0) {
        return 'DateTime.Now';
      } else if (node.arguments.length === 1) {
        const arg = generateCSharpExpression(node.arguments[0]);
        return `DateTime.Parse(${arg})`;
      }
    }

    // Handle new Error() → new Exception()
    if (t.isIdentifier(node.callee, { name: 'Error' })) {
      const args = node.arguments.map(arg => generateCSharpExpression(arg)).join(', ');
      return `new Exception(${args})`;
    }

    // Handle other new expressions: new Foo() → new Foo()
    const callee = generateCSharpExpression(node.callee);
    const args = node.arguments.map(arg => generateCSharpExpression(arg)).join(', ');
    return `new ${callee}(${args})`;
  }

  if (t.isObjectExpression(node)) {
    // Convert JS object literal to C# anonymous object or Dictionary
    // Check if any key has hyphens (invalid for C# anonymous types)
    const hasHyphenatedKeys = node.properties.some(prop => {
      if (t.isObjectProperty(prop)) {
        const key = t.isIdentifier(prop.key) ? prop.key.name : prop.key.value;
        return typeof key === 'string' && key.includes('-');
      }
      return false;
    });

    const properties = node.properties.map(prop => {
      if (t.isObjectProperty(prop)) {
        const key = t.isIdentifier(prop.key) ? prop.key.name : prop.key.value;
        const value = generateCSharpExpression(prop.value);

        if (hasHyphenatedKeys) {
          // Use Dictionary syntax with quoted keys
          return `["${key}"] = ${value}`;
        } else {
          // Use anonymous object syntax
          return `${key} = ${value}`;
        }
      }
      return '';
    }).filter(p => p !== '');

    if (properties.length === 0) return 'null';

    if (hasHyphenatedKeys) {
      return `new Dictionary<string, object> { ${properties.join(', ')} }`;
    } else {
      return `new { ${properties.join(', ')} }`;
    }
  }

  if (t.isArrowFunctionExpression(node) || t.isFunctionExpression(node)) {
    // Arrow function: (x) => x * 2  →  x => x * 2
    // Function expression: function(x) { return x * 2; }  →  x => x * 2
    const params = node.params.map(p => {
      if (t.isIdentifier(p)) return p.name;
      if (t.isObjectPattern(p)) return '{...}'; // Destructuring - simplified
      return 'param';
    }).join(', ');

    // Wrap params in parentheses if multiple or none
    const paramsString = node.params.length === 1 ? params : `(${params})`;

    // Generate function body
    let body;
    if (t.isBlockStatement(node.body)) {
      // Block body: (x) => { return x * 2; }
      const statements = node.body.body.map(stmt => generateCSharpStatement(stmt)).join(' ');
      body = `{ ${statements} }`;
    } else {
      // Expression body: (x) => x * 2
      body = generateCSharpExpression(node.body);
    }

    return `${paramsString} => ${body}`;
  }

  // Fallback for unknown node types
  const nodePath = node?.__minimactPath || '';
  return `new VNull("${nodePath}")`;
}

/**
 * Generate attribute value
 */
function generateAttributeValue(value) {
  if (!value) return '""';

  if (t.isStringLiteral(value)) {
    return `"${escapeCSharpString(value.value)}"`;
  }

  if (t.isJSXExpressionContainer(value)) {
    return generateCSharpExpression(value.expression);
  }

  return '""';
}

/**
 * Generate hybrid expression with smart span splitting
 */
function generateHybridExpression(expr, component, deps, indent) {
  // For now, return a simplified version
  // TODO: Implement full AST splitting logic
  return `new VText(${generateCSharpExpression(expr)})`;
}




/**
 * Set the current component context for useState setter detection
 */
function setCurrentComponent(component) {
  currentComponent = component;
}

module.exports = {
  generateAttributeValue,
  generateCSharpExpression,
  generateCSharpStatement,
  generateMapExpression,
  generateConditional,
  generateShortCircuit,
  generateHybridExpression,
  generateJSXExpression,
  generateBooleanExpression,
  setCurrentComponent
};
