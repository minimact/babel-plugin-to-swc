/**
 * Loop Template Extractor
 *
 * Extracts parameterized loop templates from .map() expressions for predictive rendering.
 * This enables 100% coverage for list rendering patterns with O(1) memory.
 *
 * Architecture:
 * - Build time: Detect .map() patterns and extract item templates
 * - Runtime (Rust predictor): Use Babel-generated templates as primary source
 * - Fallback: Rust runtime extraction if Babel can't generate template
 *
 * Example:
 * {todos.map(todo => <li>{todo.text}</li>)}
 * →
 * LoopTemplate {
 *   arrayBinding: "todos",
 *   itemVar: "todo",
 *   itemTemplate: ElementTemplate {
 *     tag: "li",
 *     children: [TextTemplate { template: "{0}", bindings: ["item.text"] }]
 *   }
 * }
 */

const t = require('@babel/types');

/**
 * Extract all loop templates from JSX render body
 *
 * Returns array of loop template metadata:
 * [
 *   {
 *     stateKey: "todos",
 *     arrayBinding: "todos",
 *     itemVar: "todo",
 *     indexVar: "index",
 *     keyBinding: "item.id",
 *     itemTemplate: { ... }
 *   }
 * ]
 */
function extractLoopTemplates(renderBody, component) {
  if (!renderBody) return [];

  const loopTemplates = [];

  /**
   * Traverse JSX tree looking for .map() call expressions
   */
  function traverseJSX(node) {
    if (t.isJSXElement(node)) {
      // Check attributes for .map() expressions
      for (const attr of node.openingElement.attributes) {
        if (t.isJSXAttribute(attr) && t.isJSXExpressionContainer(attr.value)) {
          findMapExpressions(attr.value.expression);
        }
      }

      // Check children for .map() expressions
      for (const child of node.children) {
        if (t.isJSXExpressionContainer(child)) {
          findMapExpressions(child.expression);
        } else if (t.isJSXElement(child)) {
          traverseJSX(child);
        } else if (t.isJSXFragment(child)) {
          for (const fragmentChild of child.children) {
            if (t.isJSXElement(fragmentChild)) {
              traverseJSX(fragmentChild);
            }
          }
        }
      }
    } else if (t.isJSXFragment(node)) {
      for (const child of node.children) {
        if (t.isJSXElement(child)) {
          traverseJSX(child);
        }
      }
    }
  }

  /**
   * Find .map() call expressions recursively
   */
  function findMapExpressions(expr) {
    if (!expr) return;

    // Direct .map() call: items.map(...)
    if (t.isCallExpression(expr) &&
        t.isMemberExpression(expr.callee) &&
        t.isIdentifier(expr.callee.property) &&
        expr.callee.property.name === 'map') {

      const loopTemplate = extractLoopTemplate(expr);
      if (loopTemplate) {
        loopTemplates.push(loopTemplate);
      }
    }

    // Chained operations: items.filter(...).map(...)
    if (t.isCallExpression(expr) &&
        t.isMemberExpression(expr.callee)) {
      findMapExpressions(expr.callee.object);
    }

    // Wrapped in other expressions
    if (t.isLogicalExpression(expr) || t.isConditionalExpression(expr)) {
      findMapExpressions(expr.left || expr.test);
      findMapExpressions(expr.right || expr.consequent);
      if (expr.alternate) findMapExpressions(expr.alternate);
    }
  }

  /**
   * Extract loop template from .map() call expression
   *
   * Example:
   * todos.map((todo, index) => <li key={todo.id}>{todo.text}</li>)
   */
  function extractLoopTemplate(mapCallExpr) {
    // Get array binding (the object being mapped)
    const arrayBinding = extractArrayBinding(mapCallExpr.callee.object);
    if (!arrayBinding) {
      console.warn('[Loop Template] Could not extract array binding from .map()');
      return null;
    }

    // Get callback function (arrow function or function expression)
    const callback = mapCallExpr.arguments[0];
    if (!t.isArrowFunctionExpression(callback) && !t.isFunctionExpression(callback)) {
      console.warn('[Loop Template] .map() callback is not a function');
      return null;
    }

    // Get item and index parameter names
    const itemVar = callback.params[0] ? callback.params[0].name : 'item';
    const indexVar = callback.params[1] ? callback.params[1].name : null;

    // Get JSX element returned by callback
    const jsxElement = extractJSXFromCallback(callback);
    if (!jsxElement) {
      console.warn('[Loop Template] .map() callback does not return JSX element');
      return null;
    }

    // Extract item template from JSX element
    const itemTemplate = extractElementTemplate(jsxElement, itemVar, indexVar);
    if (!itemTemplate) {
      console.warn('[Loop Template] Could not extract item template from JSX');
      return null;
    }

    // Extract key binding
    const keyBinding = extractKeyBinding(jsxElement, itemVar, indexVar);

    return {
      stateKey: arrayBinding,  // For C# attribute: which state variable triggers this template
      arrayBinding,            // Runtime: which array to iterate
      itemVar,                 // Runtime: variable name for each item
      indexVar,                // Runtime: variable name for index (optional)
      keyBinding,              // Runtime: expression for React key (optional)
      itemTemplate             // Runtime: template for each list item
    };
  }

  /**
   * Extract array binding from member expression
   *
   * Examples:
   * - todos.map(...) → "todos"
   * - this.state.items.map(...) → "items"
   * - [...todos].map(...) → "todos"
   */
  function extractArrayBinding(expr) {
    if (t.isIdentifier(expr)) {
      return expr.name;
    } else if (t.isMemberExpression(expr)) {
      // Get the last property name
      if (t.isIdentifier(expr.property)) {
        return expr.property.name;
      }
    } else if (t.isCallExpression(expr)) {
      // Handle array methods like .reverse(), .slice()
      if (t.isMemberExpression(expr.callee)) {
        return extractArrayBinding(expr.callee.object);
      }
    } else if (t.isArrayExpression(expr)) {
      // Spread array: [...todos]
      if (expr.elements.length > 0 && t.isSpreadElement(expr.elements[0])) {
        return extractArrayBinding(expr.elements[0].argument);
      }
    }
    return null;
  }

  /**
   * Extract JSX element from callback function body
   */
  function extractJSXFromCallback(callback) {
    const body = callback.body;

    // Arrow function with direct JSX return: (...) => <li>...</li>
    if (t.isJSXElement(body)) {
      return body;
    }

    // Arrow function or function expression with block body
    if (t.isBlockStatement(body)) {
      // Find return statement
      for (const stmt of body.body) {
        if (t.isReturnStatement(stmt) && t.isJSXElement(stmt.argument)) {
          return stmt.argument;
        }
      }
    }

    // Expression wrapped in parentheses or conditional
    if (t.isConditionalExpression(body)) {
      // Handle ternary: condition ? <div/> : <span/>
      // For now, just take the consequent (true branch)
      if (t.isJSXElement(body.consequent)) {
        return body.consequent;
      }
    }

    if (t.isLogicalExpression(body) && body.operator === '&&') {
      // Handle logical AND: condition && <div/>
      if (t.isJSXElement(body.right)) {
        return body.right;
      }
    }

    return null;
  }

  /**
   * Extract key binding from JSX element
   *
   * Example: <li key={todo.id}> → "item.id"
   */
  function extractKeyBinding(jsxElement, itemVar, indexVar) {
    const keyAttr = jsxElement.openingElement.attributes.find(
      attr => t.isJSXAttribute(attr) &&
              t.isIdentifier(attr.name) &&
              attr.name.name === 'key'
    );

    if (!keyAttr) return null;

    const keyValue = keyAttr.value;
    if (t.isJSXExpressionContainer(keyValue)) {
      return buildBindingPath(keyValue.expression, itemVar, indexVar);
    } else if (t.isStringLiteral(keyValue)) {
      return null; // Static key (not based on item data)
    }

    return null;
  }

  /**
   * Extract element template from JSX element
   *
   * Returns template in format compatible with Rust LoopTemplate:
   * {
   *   type: "Element",
   *   tag: "li",
   *   propsTemplates: { className: { template: "{0}", bindings: ["item.done"], ... } },
   *   childrenTemplates: [ ... ],
   *   keyBinding: "item.id"
   * }
   */
  function extractElementTemplate(jsxElement, itemVar, indexVar) {
    const tagName = jsxElement.openingElement.name.name;

    // Extract prop templates
    const propsTemplates = extractPropTemplates(
      jsxElement.openingElement.attributes,
      itemVar,
      indexVar
    );

    // Extract children templates
    const childrenTemplates = extractChildrenTemplates(
      jsxElement.children,
      itemVar,
      indexVar
    );

    return {
      type: 'Element',
      tag: tagName,
      propsTemplates: Object.keys(propsTemplates).length > 0 ? propsTemplates : null,
      childrenTemplates: childrenTemplates.length > 0 ? childrenTemplates : null
    };
  }

  /**
   * Extract prop templates from JSX attributes
   *
   * Handles:
   * - Simple bindings: checked={todo.done} → { template: "{0}", bindings: ["item.done"] }
   * - Conditionals: className={todo.done ? 'done' : 'pending'} → conditional template
   * - Template literals: className={`item-${todo.id}`} → template with placeholder
   */
  function extractPropTemplates(attributes, itemVar, indexVar) {
    const templates = {};

    for (const attr of attributes) {
      // Skip non-JSXAttribute (spreads, etc.)
      if (!t.isJSXAttribute(attr)) continue;

      // Skip key attribute (handled separately)
      if (attr.name.name === 'key') continue;

      const propName = attr.name.name;
      const propValue = attr.value;

      // Static string: className="static"
      if (t.isStringLiteral(propValue)) {
        templates[propName] = {
          template: propValue.value,
          bindings: [],
          slots: [],
          type: 'static'
        };
        continue;
      }

      // Expression: {todo.done}, {todo.done ? 'yes' : 'no'}
      if (t.isJSXExpressionContainer(propValue)) {
        const expr = propValue.expression;

        // Conditional: {todo.done ? 'active' : 'inactive'}
        if (t.isConditionalExpression(expr)) {
          const conditionalTemplate = extractConditionalTemplate(expr, itemVar, indexVar);
          if (conditionalTemplate) {
            templates[propName] = conditionalTemplate;
            continue;
          }
        }

        // Template literal: {`item-${todo.id}`}
        if (t.isTemplateLiteral(expr)) {
          const template = extractTemplateFromTemplateLiteral(expr, itemVar, indexVar);
          if (template) {
            templates[propName] = template;
            continue;
          }
        }

        // Simple binding: {todo.text}, {todo.done}
        const binding = buildBindingPath(expr, itemVar, indexVar);
        if (binding) {
          templates[propName] = {
            template: '{0}',
            bindings: [binding],
            slots: [0],
            type: 'binding'
          };
        }
      }
    }

    return templates;
  }

  /**
   * Extract conditional template from ternary expression
   *
   * Example: todo.done ? 'completed' : 'pending'
   * →
   * {
   *   template: "{0}",
   *   bindings: ["item.done"],
   *   conditionalTemplates: { "true": "completed", "false": "pending" },
   *   conditionalBindingIndex: 0
   * }
   */
  function extractConditionalTemplate(conditionalExpr, itemVar, indexVar) {
    const test = conditionalExpr.test;
    const consequent = conditionalExpr.consequent;
    const alternate = conditionalExpr.alternate;

    // Extract binding from test expression
    const binding = buildBindingPath(test, itemVar, indexVar);
    if (!binding) return null;

    // Extract literal values from consequent and alternate
    const trueValue = extractLiteralValue(consequent);
    const falseValue = extractLiteralValue(alternate);

    if (trueValue === null || falseValue === null) {
      // Complex expressions in branches - can't template it
      return null;
    }

    return {
      template: '{0}',
      bindings: [binding],
      slots: [0],
      conditionalTemplates: {
        'true': trueValue,
        'false': falseValue
      },
      conditionalBindingIndex: 0,
      type: 'conditional'
    };
  }

  /**
   * Extract template from template literal
   *
   * Example: `item-${todo.id}`
   * →
   * {
   *   template: "item-{0}",
   *   bindings: ["item.id"],
   *   slots: [5]
   * }
   */
  function extractTemplateFromTemplateLiteral(templateLiteral, itemVar, indexVar) {
    let templateStr = '';
    const bindings = [];
    const slots = [];

    for (let i = 0; i < templateLiteral.quasis.length; i++) {
      const quasi = templateLiteral.quasis[i];
      templateStr += quasi.value.raw;

      if (i < templateLiteral.expressions.length) {
        const expr = templateLiteral.expressions[i];
        const binding = buildBindingPath(expr, itemVar, indexVar);

        if (binding) {
          slots.push(templateStr.length);
          templateStr += `{${bindings.length}}`;
          bindings.push(binding);
        } else {
          // Complex expression - can't template it
          return null;
        }
      }
    }

    return {
      template: templateStr,
      bindings,
      slots,
      type: 'template-literal'
    };
  }

  /**
   * Extract children templates from JSX children
   *
   * Returns array of templates (text or element)
   */
  function extractChildrenTemplates(children, itemVar, indexVar) {
    const templates = [];

    for (const child of children) {
      // Static text: <li>Static text</li>
      if (t.isJSXText(child)) {
        const text = child.value.trim();
        if (text) {
          templates.push({
            type: 'Text',
            template: text,
            bindings: [],
            slots: []
          });
        }
        continue;
      }

      // Expression: <li>{todo.text}</li>
      if (t.isJSXExpressionContainer(child)) {
        const template = extractTextTemplate(child.expression, itemVar, indexVar);
        if (template) {
          templates.push(template);
        }
        continue;
      }

      // Nested element: <li><span>{todo.text}</span></li>
      if (t.isJSXElement(child)) {
        const elementTemplate = extractElementTemplate(child, itemVar, indexVar);
        if (elementTemplate) {
          templates.push(elementTemplate);
        }
        continue;
      }
    }

    return templates;
  }

  /**
   * Extract text template from expression
   *
   * Handles:
   * - Simple binding: {todo.text} → { template: "{0}", bindings: ["item.text"] }
   * - Conditional: {todo.done ? '✓' : '○'} → conditional template
   * - Binary expressions: {todo.count + 1} → expression template
   * - Method calls: {todo.text.toUpperCase()} → expression template
   * - Logical expressions: {todo.date || 'N/A'} → expression template
   */
  function extractTextTemplate(expr, itemVar, indexVar) {
    // Template literal: {`${user.firstName} ${user.lastName}`}
    if (t.isTemplateLiteral(expr)) {
      const templateLiteralResult = extractTemplateFromTemplateLiteral(expr, itemVar, indexVar);
      if (templateLiteralResult) {
        return {
          type: 'Text',
          ...templateLiteralResult
        };
      }
    }

    // Conditional expression: {todo.done ? '✓' : '○'}
    if (t.isConditionalExpression(expr)) {
      const conditionalTemplate = extractConditionalTemplate(expr, itemVar, indexVar);
      if (conditionalTemplate) {
        return {
          type: 'Text',
          ...conditionalTemplate
        };
      }
    }

    // Try to extract binding (handles simple, binary, method calls, etc.)
    const binding = buildBindingPath(expr, itemVar, indexVar);
    if (binding) {
      return {
        type: 'Text',
        template: '{0}',
        bindings: [binding],
        slots: [0]
      };
    }

    // No binding found
    return null;
  }

  /**
   * Build binding path from expression relative to item variable
   *
   * Examples:
   * - todo → null (just the item itself)
   * - todo.text → "item.text"
   * - todo.author.name → "item.author.name"
   * - index → "index"
   * - todo.priority + 1 → "__expr__:item.priority"
   * - todo.text.toUpperCase() → "__expr__:item.text"
   * - index * 2 + 1 → "__expr__:index"
   */
  function buildBindingPath(expr, itemVar, indexVar) {
    if (t.isIdentifier(expr)) {
      // Just the item variable itself
      if (expr.name === itemVar) {
        return null; // Can't template the entire item object
      }
      // Index variable
      if (expr.name === 'index' || expr.name === indexVar) {
        return 'index';
      }
      // Other identifier (likely a closure variable)
      return null;
    }

    if (t.isMemberExpression(expr)) {
      const path = buildMemberExpressionPath(expr);
      if (path && path.startsWith(itemVar + '.')) {
        // Replace item variable with "item" prefix
        return 'item' + path.substring(itemVar.length);
      }
    }

    // Handle binary expressions: todo.priority + 1, price * quantity, etc.
    if (t.isBinaryExpression(expr)) {
      return extractLoopBinaryExpression(expr, itemVar, indexVar);
    }

    // Handle logical expressions: todo.dueDate || 'No due date'
    if (t.isLogicalExpression(expr)) {
      return extractLoopLogicalExpression(expr, itemVar, indexVar);
    }

    // Handle unary expressions: !todo.completed, -value
    if (t.isUnaryExpression(expr)) {
      return extractLoopUnaryExpression(expr, itemVar, indexVar);
    }

    // Handle call expressions: todo.text.toUpperCase(), array.concat()
    if (t.isCallExpression(expr)) {
      return extractLoopCallExpression(expr, itemVar, indexVar);
    }

    return null;
  }

  /**
   * Extract binding from binary expression in loop
   * Examples: todo.priority + 1, price * quantity, index * 2 + 1
   */
  function extractLoopBinaryExpression(expr, itemVar, indexVar) {
    const identifiers = [];
    extractLoopIdentifiers(expr, identifiers, itemVar, indexVar);

    if (identifiers.length === 0) {
      return null;
    }

    // Use __expr__ prefix to indicate this is a computed expression
    return `__expr__:${identifiers.join(',')}`;
  }

  /**
   * Extract binding from logical expression in loop
   * Examples: todo.dueDate || 'No due date', condition && value
   */
  function extractLoopLogicalExpression(expr, itemVar, indexVar) {
    const identifiers = [];
    extractLoopIdentifiers(expr, identifiers, itemVar, indexVar);

    if (identifiers.length === 0) {
      return null;
    }

    // Use __expr__ prefix to indicate this is a computed expression
    return `__expr__:${identifiers.join(',')}`;
  }

  /**
   * Extract binding from unary expression in loop
   * Examples: !todo.completed, -value
   */
  function extractLoopUnaryExpression(expr, itemVar, indexVar) {
    const identifiers = [];
    extractLoopIdentifiers(expr, identifiers, itemVar, indexVar);

    if (identifiers.length === 0) {
      return null;
    }

    // Use __expr__ prefix to indicate this is a computed expression
    return `__expr__:${identifiers.join(',')}`;
  }

  /**
   * Extract binding from call expression in loop
   * Examples: todo.text.toUpperCase(), todo.text.substring(0, 10)
   */
  function extractLoopCallExpression(expr, itemVar, indexVar) {
    const identifiers = [];
    extractLoopIdentifiers(expr, identifiers, itemVar, indexVar);

    if (identifiers.length === 0) {
      return null;
    }

    // Use __expr__ prefix to indicate this is a computed expression
    return `__expr__:${identifiers.join(',')}`;
  }

  /**
   * Extract identifiers from expression, converting item references to "item" prefix
   */
  function extractLoopIdentifiers(expr, result, itemVar, indexVar) {
    if (t.isIdentifier(expr)) {
      if (expr.name === itemVar) {
        // Don't add raw item variable
        return;
      } else if (expr.name === 'index' || expr.name === indexVar) {
        result.push('index');
      } else {
        result.push(expr.name);
      }
    } else if (t.isBinaryExpression(expr) || t.isLogicalExpression(expr)) {
      extractLoopIdentifiers(expr.left, result, itemVar, indexVar);
      extractLoopIdentifiers(expr.right, result, itemVar, indexVar);
    } else if (t.isUnaryExpression(expr)) {
      extractLoopIdentifiers(expr.argument, result, itemVar, indexVar);
    } else if (t.isMemberExpression(expr)) {
      const path = buildMemberExpressionPath(expr);
      if (path) {
        if (path.startsWith(itemVar + '.')) {
          // Replace item variable with "item" prefix
          result.push('item' + path.substring(itemVar.length));
        } else {
          result.push(path);
        }
      } else {
        // Complex member expression (e.g., (a + b).toFixed())
        // Extract from both object and property
        extractLoopIdentifiers(expr.object, result, itemVar, indexVar);
        if (t.isIdentifier(expr.property)) {
          result.push(expr.property.name);
        }
      }
    } else if (t.isCallExpression(expr)) {
      // Extract from callee
      extractLoopIdentifiers(expr.callee, result, itemVar, indexVar);
      // Extract from arguments
      for (const arg of expr.arguments) {
        extractLoopIdentifiers(arg, result, itemVar, indexVar);
      }
    }
  }

  /**
   * Build full path from member expression
   *
   * Example: todo.author.name → "todo.author.name"
   */
  function buildMemberExpressionPath(expr) {
    const parts = [];
    let current = expr;

    while (t.isMemberExpression(current)) {
      if (t.isIdentifier(current.property)) {
        parts.unshift(current.property.name);
      } else {
        return null; // Computed property (not supported)
      }
      current = current.object;
    }

    if (t.isIdentifier(current)) {
      parts.unshift(current.name);
      return parts.join('.');
    }

    return null;
  }

  /**
   * Extract literal value from expression
   */
  function extractLiteralValue(expr) {
    if (t.isStringLiteral(expr)) {
      return expr.value;
    } else if (t.isNumericLiteral(expr)) {
      return expr.value;
    } else if (t.isBooleanLiteral(expr)) {
      return expr.value;
    } else if (t.isNullLiteral(expr)) {
      return null;
    }
    return null; // Complex expression
  }

  // Start traversal
  traverseJSX(renderBody);

  return loopTemplates;
}

module.exports = {
  extractLoopTemplates
};
