/**
 * Expression Template Extractor (Phase 6)
 *
 * Extracts templates for computed values and transformations.
 * This handles cases like number formatting, arithmetic, string operations, etc.
 *
 * Examples:
 * - {price.toFixed(2)}
 * - {count * 2 + 1}
 * - {name.toUpperCase()}
 * - {items.length}
 *
 * Architecture:
 * - Build time: Detect expression patterns and extract transformation metadata
 * - Runtime: Store expression templates with bindings and transforms
 * - Prediction: Apply transforms to current state values
 *
 * Security Note:
 * Only safe, whitelisted transformations are supported. No arbitrary JavaScript execution.
 */

const t = require('@babel/types');

/**
 * Supported transformation types
 */
const SUPPORTED_TRANSFORMS = {
  // Number formatting
  'toFixed': { type: 'numberFormat', safe: true },
  'toPrecision': { type: 'numberFormat', safe: true },
  'toExponential': { type: 'numberFormat', safe: true },

  // String operations
  'toUpperCase': { type: 'stringTransform', safe: true },
  'toLowerCase': { type: 'stringTransform', safe: true },
  'trim': { type: 'stringTransform', safe: true },
  'substring': { type: 'stringTransform', safe: true },
  'substr': { type: 'stringTransform', safe: true },
  'slice': { type: 'stringTransform', safe: true },

  // Array operations
  'length': { type: 'property', safe: true },
  'join': { type: 'arrayTransform', safe: true },

  // Math operations (handled separately via binary expressions)
  // +, -, *, /, %
};

/**
 * Extract expression templates from JSX render body
 *
 * Returns array of expression template metadata:
 * [
 *   {
 *     type: 'expression',
 *     template: '${0}',
 *     bindings: ['price'],
 *     transforms: [
 *       { type: 'toFixed', args: [2] }
 *     ]
 *   }
 * ]
 */
function extractExpressionTemplates(renderBody, component) {
  if (!renderBody) return [];

  const expressionTemplates = [];

  /**
   * Traverse JSX tree looking for expression containers
   */
  function traverseJSX(node, path = []) {
    if (t.isJSXElement(node)) {
      // Check children for expressions
      for (let i = 0; i < node.children.length; i++) {
        const child = node.children[i];

        if (t.isJSXExpressionContainer(child)) {
          const template = extractExpressionTemplate(child.expression, component, [...path, i]);
          if (template) {
            expressionTemplates.push(template);
          }
        } else if (t.isJSXElement(child)) {
          traverseJSX(child, [...path, i]);
        }
      }

      // Check attributes for expressions
      for (const attr of node.openingElement.attributes) {
        if (t.isJSXAttribute(attr) && t.isJSXExpressionContainer(attr.value)) {
          const template = extractExpressionTemplate(attr.value.expression, component, path);
          if (template) {
            template.attribute = attr.name.name;
            expressionTemplates.push(template);
          }
        }
      }
    }
  }

  /**
   * Extract expression template from expression node
   */
  function extractExpressionTemplate(expr, component, path) {
    // Skip if it's a simple identifier (no transformation)
    if (t.isIdentifier(expr)) {
      return null;
    }

    // Skip conditionals (handled by structural templates)
    if (t.isConditionalExpression(expr) || t.isLogicalExpression(expr)) {
      return null;
    }

    // Method call: price.toFixed(2)
    if (t.isCallExpression(expr) && t.isMemberExpression(expr.callee)) {
      return extractMethodCallTemplate(expr, component, path);
    }

    // Binary expression: count * 2 + 1
    if (t.isBinaryExpression(expr)) {
      return extractBinaryExpressionTemplate(expr, component, path);
    }

    // Member expression: user.name, items.length
    if (t.isMemberExpression(expr)) {
      return extractMemberExpressionTemplate(expr, component, path);
    }

    // Unary expression: -count, +value
    if (t.isUnaryExpression(expr)) {
      return extractUnaryExpressionTemplate(expr, component, path);
    }

    return null;
  }

  /**
   * Extract template from method call
   *
   * Example: price.toFixed(2)
   * →
   * {
   *   type: 'methodCall',
   *   binding: 'price',
   *   method: 'toFixed',
   *   args: [2],
   *   transform: { type: 'numberFormat', method: 'toFixed', args: [2] }
   * }
   */
  function extractMethodCallTemplate(callExpr, component, path) {
    const callee = callExpr.callee;
    const args = callExpr.arguments;

    // Get binding (e.g., 'price' from price.toFixed())
    const binding = extractBinding(callee.object);
    if (!binding) return null;

    // Get method name
    const methodName = callee.property.name;

    // Check if this is a supported transformation
    if (!SUPPORTED_TRANSFORMS[methodName]) {
      console.warn(`[Expression Template] Unsupported method: ${methodName}`);
      return null;
    }

    // Extract arguments
    const extractedArgs = args.map(arg => {
      if (t.isNumericLiteral(arg)) return arg.value;
      if (t.isStringLiteral(arg)) return arg.value;
      if (t.isBooleanLiteral(arg)) return arg.value;
      return null;
    }).filter(a => a !== null);

    // Determine state key
    const stateKey = extractStateKey(callee.object, component);

    return {
      type: 'methodCall',
      stateKey: stateKey || binding,
      binding,
      method: methodName,
      args: extractedArgs,
      transform: {
        type: SUPPORTED_TRANSFORMS[methodName].type,
        method: methodName,
        args: extractedArgs
      },
      path
    };
  }

  /**
   * Extract template from binary expression
   *
   * Example: count * 2 + 1
   * →
   * {
   *   type: 'binaryExpression',
   *   bindings: ['count'],
   *   expression: 'count * 2 + 1',
   *   transform: {
   *     type: 'arithmetic',
   *     operations: [
   *       { op: '*', right: 2 },
   *       { op: '+', right: 1 }
   *     ]
   *   }
   * }
   */
  function extractBinaryExpressionTemplate(binaryExpr, component, path) {
    // Extract all identifiers
    const identifiers = [];
    extractIdentifiers(binaryExpr, identifiers);

    if (identifiers.length === 0) return null;

    // For simple cases (single identifier with constant), extract transform
    if (identifiers.length === 1) {
      const binding = identifiers[0];
      const transform = analyzeBinaryExpression(binaryExpr, binding);

      if (transform) {
        const stateKey = binding.split('.')[0];
        return {
          type: 'binaryExpression',
          stateKey,
          bindings: [binding],
          transform,
          path
        };
      }
    }

    // Complex multi-variable expression - store as formula
    return {
      type: 'complexExpression',
      stateKey: identifiers[0].split('.')[0],
      bindings: identifiers,
      expression: generateExpressionString(binaryExpr),
      path
    };
  }

  /**
   * Analyze binary expression to extract arithmetic operations
   *
   * Example: count * 2 + 1 with binding="count"
   * →
   * {
   *   type: 'arithmetic',
   *   operations: [
   *     { op: '*', value: 2 },
   *     { op: '+', value: 1 }
   *   ]
   * }
   */
  function analyzeBinaryExpression(expr, targetBinding) {
    const operations = [];

    function analyze(node) {
      if (t.isBinaryExpression(node)) {
        const { left, operator, right } = node;

        // Check if one side is our target binding
        const leftIsTarget = isBindingExpression(left, targetBinding);
        const rightIsTarget = isBindingExpression(right, targetBinding);

        if (leftIsTarget && t.isNumericLiteral(right)) {
          operations.push({ op: operator, value: right.value, side: 'right' });
        } else if (rightIsTarget && t.isNumericLiteral(left)) {
          operations.push({ op: operator, value: left.value, side: 'left' });
        } else {
          // Recurse
          analyze(left);
          analyze(right);
        }
      }
    }

    analyze(expr);

    if (operations.length > 0) {
      return {
        type: 'arithmetic',
        operations
      };
    }

    return null;
  }

  /**
   * Check if expression is our target binding
   */
  function isBindingExpression(expr, targetBinding) {
    const binding = extractBinding(expr);
    return binding === targetBinding;
  }

  /**
   * Extract template from member expression
   *
   * Example: items.length
   * →
   * {
   *   type: 'memberExpression',
   *   binding: 'items.length',
   *   transform: { type: 'property', property: 'length' }
   * }
   */
  function extractMemberExpressionTemplate(memberExpr, component, path) {
    // Check for computed property access: item[field]
    if (memberExpr.computed) {
      console.warn('[Minimact Warning] Computed property access detected - skipping template optimization (requires runtime evaluation)');

      // Return a special marker indicating this needs runtime evaluation
      // The C# generator will handle this as dynamic property access
      return {
        type: 'computedMemberExpression',
        isComputed: true,
        requiresRuntimeEval: true,
        object: memberExpr.object,
        property: memberExpr.property,
        path
      };
    }

    const binding = buildMemberPath(memberExpr);
    if (!binding) return null;

    // Get property name (only for non-computed properties)
    const propertyName = memberExpr.property.name;

    // Check if it's a supported property
    if (!SUPPORTED_TRANSFORMS[propertyName]) {
      return null;
    }

    const stateKey = extractStateKey(memberExpr, component);

    return {
      type: 'memberExpression',
      stateKey: stateKey || binding.split('.')[0],
      binding,
      property: propertyName,
      transform: {
        type: SUPPORTED_TRANSFORMS[propertyName].type,
        property: propertyName
      },
      path
    };
  }

  /**
   * Extract template from unary expression
   *
   * Example: -count, +value
   */
  function extractUnaryExpressionTemplate(unaryExpr, component, path) {
    const { operator, argument } = unaryExpr;

    const binding = extractBinding(argument);
    if (!binding) return null;

    if (operator === '-' || operator === '+') {
      const stateKey = extractStateKey(argument, component);

      return {
        type: 'unaryExpression',
        stateKey: stateKey || binding,
        binding,
        operator,
        transform: {
          type: 'unary',
          operator
        },
        path
      };
    }

    return null;
  }

  /**
   * Extract binding from expression
   */
  function extractBinding(expr) {
    if (t.isIdentifier(expr)) {
      return expr.name;
    } else if (t.isMemberExpression(expr)) {
      return buildMemberPath(expr);
    }
    return null;
  }

  /**
   * Extract state key (root variable)
   */
  function extractStateKey(expr, component) {
    if (t.isIdentifier(expr)) {
      return expr.name;
    } else if (t.isMemberExpression(expr)) {
      let current = expr;
      while (t.isMemberExpression(current)) {
        current = current.object;
      }
      if (t.isIdentifier(current)) {
        return current.name;
      }
    }
    return null;
  }

  /**
   * Build member expression path
   */
  function buildMemberPath(expr) {
    const parts = [];
    let current = expr;

    while (t.isMemberExpression(current)) {
      if (t.isIdentifier(current.property)) {
        parts.unshift(current.property.name);
      }
      current = current.object;
    }

    if (t.isIdentifier(current)) {
      parts.unshift(current.name);
    }

    return parts.join('.');
  }

  /**
   * Extract all identifiers from expression
   */
  function extractIdentifiers(expr, result) {
    if (t.isIdentifier(expr)) {
      result.push(expr.name);
    } else if (t.isBinaryExpression(expr)) {
      extractIdentifiers(expr.left, result);
      extractIdentifiers(expr.right, result);
    } else if (t.isUnaryExpression(expr)) {
      extractIdentifiers(expr.argument, result);
    } else if (t.isMemberExpression(expr)) {
      const path = buildMemberPath(expr);
      if (path) result.push(path);
    }
  }

  /**
   * Generate expression string for complex expressions
   */
  function generateExpressionString(expr) {
    if (t.isIdentifier(expr)) {
      return expr.name;
    } else if (t.isNumericLiteral(expr)) {
      return String(expr.value);
    } else if (t.isBinaryExpression(expr)) {
      const left = generateExpressionString(expr.left);
      const right = generateExpressionString(expr.right);
      return `${left} ${expr.operator} ${right}`;
    } else if (t.isUnaryExpression(expr)) {
      const arg = generateExpressionString(expr.argument);
      return `${expr.operator}${arg}`;
    } else if (t.isMemberExpression(expr)) {
      return buildMemberPath(expr);
    }
    return '?';
  }

  // Start traversal
  traverseJSX(renderBody);

  return expressionTemplates;
}

module.exports = {
  extractExpressionTemplates,
  SUPPORTED_TRANSFORMS
};
