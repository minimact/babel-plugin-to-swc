/**
 * Call expression handlers
 */

const t = require('@babel/types');
const { generateJSXElement } = require('../jsx.cjs');

/**
 * Generate call expression
 */
function generateCallExpression(node, generateCSharpExpression, generateCSharpStatement, currentComponent) {
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

/**
 * Generate optional call expression
 */
function generateOptionalCallExpression(node, generateCSharpExpression, generateCSharpStatement, currentComponent) {
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

module.exports = {
  generateCallExpression,
  generateOptionalCallExpression
};
