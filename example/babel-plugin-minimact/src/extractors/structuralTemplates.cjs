/**
 * Structural Template Extractor (Phase 5)
 *
 * Extracts templates for conditional rendering patterns where the DOM structure changes.
 * This handles cases like loading states, authentication states, error boundaries, etc.
 *
 * Examples:
 * - {isLoading ? <Spinner /> : <Content />}
 * - {user ? <Dashboard /> : <LoginForm />}
 * - {error && <ErrorMessage />}
 *
 * Architecture:
 * - Build time: Detect conditional patterns and extract both branches
 * - Runtime: Store structural templates with condition binding
 * - Prediction: Choose correct branch based on current state
 */

const t = require('@babel/types');

/**
 * Extract structural templates from JSX render body
 *
 * Returns array of structural template metadata:
 * [
 *   {
 *     type: 'conditional',
 *     stateKey: 'isLoggedIn',
 *     conditionBinding: 'isLoggedIn',
 *     branches: {
 *       'true': { type: 'Element', tag: 'div', ... },
 *       'false': { type: 'Element', tag: 'div', ... }
 *     }
 *   }
 * ]
 */
function extractStructuralTemplates(renderBody, component) {
  if (!renderBody) return [];

  const structuralTemplates = [];

  /**
   * Traverse JSX tree looking for conditional expressions that affect structure
   */
  function traverseJSX(node, path = []) {
    if (t.isJSXElement(node)) {
      // Check children for conditional expressions
      for (let i = 0; i < node.children.length; i++) {
        const child = node.children[i];

        if (t.isJSXExpressionContainer(child)) {
          const expr = child.expression;

          // Ternary: {condition ? <A /> : <B />}
          if (t.isConditionalExpression(expr)) {
            const template = extractConditionalStructuralTemplate(expr, component, [...path, i]);
            if (template) {
              structuralTemplates.push(template);
            }
          }

          // Logical AND: {condition && <Component />}
          if (t.isLogicalExpression(expr) && expr.operator === '&&') {
            const template = extractLogicalAndTemplate(expr, component, [...path, i]);
            if (template) {
              structuralTemplates.push(template);
            }
          }
        } else if (t.isJSXElement(child)) {
          traverseJSX(child, [...path, i]);
        }
      }
    } else if (t.isJSXFragment(node)) {
      for (let i = 0; i < node.children.length; i++) {
        const child = node.children[i];
        if (t.isJSXElement(child)) {
          traverseJSX(child, [...path, i]);
        } else if (t.isJSXExpressionContainer(child)) {
          const expr = child.expression;

          if (t.isConditionalExpression(expr)) {
            const template = extractConditionalStructuralTemplate(expr, component, [...path, i]);
            if (template) {
              structuralTemplates.push(template);
            }
          }

          if (t.isLogicalExpression(expr) && expr.operator === '&&') {
            const template = extractLogicalAndTemplate(expr, component, [...path, i]);
            if (template) {
              structuralTemplates.push(template);
            }
          }
        }
      }
    }
  }

  /**
   * Extract structural template from ternary conditional
   *
   * Example: {isLoggedIn ? <Dashboard /> : <LoginForm />}
   * →
   * {
   *   type: 'conditional',
   *   conditionBinding: 'isLoggedIn',
   *   branches: {
   *     'true': ElementTemplate { tag: 'Dashboard', ... },
   *     'false': ElementTemplate { tag: 'LoginForm', ... }
   *   }
   * }
   */
  function extractConditionalStructuralTemplate(conditionalExpr, component, path) {
    const test = conditionalExpr.test;
    const consequent = conditionalExpr.consequent;
    const alternate = conditionalExpr.alternate;

    // Extract condition binding
    const conditionBinding = extractBinding(test, component);
    if (!conditionBinding) {
      console.warn('[Structural Template] Could not extract condition binding');
      return null;
    }

    // Check if both branches are JSX elements (structural change)
    const hasTrueBranch = t.isJSXElement(consequent) || t.isJSXFragment(consequent);
    const hasFalseBranch = t.isJSXElement(alternate) || t.isJSXFragment(alternate) || t.isNullLiteral(alternate);

    if (!hasTrueBranch && !hasFalseBranch) {
      // Not a structural template (probably just conditional text)
      return null;
    }

    // Extract templates for both branches
    const branches = {};

    if (hasTrueBranch) {
      const trueBranch = extractElementOrFragmentTemplate(consequent, component);
      if (trueBranch) {
        branches['true'] = trueBranch;
      }
    }

    if (hasFalseBranch) {
      if (t.isNullLiteral(alternate)) {
        branches['false'] = { type: 'Null' };
      } else {
        const falseBranch = extractElementOrFragmentTemplate(alternate, component);
        if (falseBranch) {
          branches['false'] = falseBranch;
        }
      }
    }

    // Determine state key (for C# attribute)
    const stateKey = extractStateKey(test, component);

    return {
      type: 'conditional',
      stateKey: stateKey || conditionBinding,
      conditionBinding,
      branches,
      path
    };
  }

  /**
   * Extract structural template from logical AND
   *
   * Example: {error && <ErrorMessage />}
   * →
   * {
   *   type: 'logicalAnd',
   *   conditionBinding: 'error',
   *   branches: {
   *     'truthy': ElementTemplate { tag: 'ErrorMessage', ... },
   *     'falsy': { type: 'Null' }
   *   }
   * }
   */
  function extractLogicalAndTemplate(logicalExpr, component, path) {
    const left = logicalExpr.left;
    const right = logicalExpr.right;

    // Extract condition binding from left side
    const conditionBinding = extractBinding(left, component);
    if (!conditionBinding) {
      return null;
    }

    // Check if right side is JSX element (structural change)
    if (!t.isJSXElement(right) && !t.isJSXFragment(right)) {
      return null;
    }

    // Extract template for truthy case
    const truthyBranch = extractElementOrFragmentTemplate(right, component);
    if (!truthyBranch) {
      return null;
    }

    const stateKey = extractStateKey(left, component);

    return {
      type: 'logicalAnd',
      stateKey: stateKey || conditionBinding,
      conditionBinding,
      branches: {
        'truthy': truthyBranch,
        'falsy': { type: 'Null' }
      },
      path
    };
  }

  /**
   * Extract element or fragment template
   */
  function extractElementOrFragmentTemplate(node, component) {
    if (t.isJSXElement(node)) {
      return extractSimpleElementTemplate(node, component);
    } else if (t.isJSXFragment(node)) {
      return {
        type: 'Fragment',
        children: node.children
          .filter(child => t.isJSXElement(child) || t.isJSXText(child))
          .map(child => {
            if (t.isJSXElement(child)) {
              return extractSimpleElementTemplate(child, component);
            } else if (t.isJSXText(child)) {
              const text = child.value.trim();
              return text ? { type: 'Text', content: text } : null;
            }
          })
          .filter(Boolean)
      };
    }
    return null;
  }

  /**
   * Extract simple element template (without nested state dependencies)
   *
   * For structural templates, we extract a simplified version that captures:
   * - Tag name
   * - Static props
   * - Structure (not deeply nested templates)
   */
  function extractSimpleElementTemplate(jsxElement, component) {
    const tagName = jsxElement.openingElement.name.name;
    const attributes = jsxElement.openingElement.attributes;

    // Extract static props only (complex props handled separately)
    const props = {};
    for (const attr of attributes) {
      if (t.isJSXAttribute(attr)) {
        const propName = attr.name.name;
        const propValue = attr.value;

        if (t.isStringLiteral(propValue)) {
          props[propName] = propValue.value;
        } else if (t.isJSXExpressionContainer(propValue)) {
          // Mark as dynamic (will be re-evaluated)
          const expr = propValue.expression;
          if (t.isIdentifier(expr)) {
            props[propName] = { binding: expr.name };
          } else {
            props[propName] = { expression: true };
          }
        }
      }
    }

    // Extract children (simplified)
    const children = jsxElement.children
      .filter(child => t.isJSXElement(child) || t.isJSXText(child))
      .map(child => {
        if (t.isJSXElement(child)) {
          return extractSimpleElementTemplate(child, component);
        } else if (t.isJSXText(child)) {
          const text = child.value.trim();
          return text ? { type: 'Text', content: text } : null;
        }
      })
      .filter(Boolean);

    return {
      type: 'Element',
      tag: tagName,
      props: Object.keys(props).length > 0 ? props : null,
      children: children.length > 0 ? children : null
    };
  }

  /**
   * Extract binding from expression
   */
  function extractBinding(expr, component) {
    if (t.isIdentifier(expr)) {
      return expr.name;
    } else if (t.isMemberExpression(expr)) {
      return buildMemberPath(expr);
    } else if (t.isUnaryExpression(expr) && expr.operator === '!') {
      // Handle !isLoading
      const binding = extractBinding(expr.argument, component);
      return binding ? `!${binding}` : null;
    }
    return null;
  }

  /**
   * Extract state key (root variable name) from expression
   */
  function extractStateKey(expr, component) {
    if (t.isIdentifier(expr)) {
      return expr.name;
    } else if (t.isMemberExpression(expr)) {
      // Get root object: user.isLoggedIn → "user"
      let current = expr;
      while (t.isMemberExpression(current)) {
        current = current.object;
      }
      if (t.isIdentifier(current)) {
        return current.name;
      }
    } else if (t.isUnaryExpression(expr)) {
      return extractStateKey(expr.argument, component);
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

  // Start traversal
  traverseJSX(renderBody);

  return structuralTemplates;
}

module.exports = {
  extractStructuralTemplates
};
