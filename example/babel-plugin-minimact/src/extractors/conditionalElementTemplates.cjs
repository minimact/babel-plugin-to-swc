/**
 * Conditional Element Template Extractor
 *
 * Extracts complete element structure templates for conditional rendering.
 * This enables the client to construct entire element trees instantly when
 * conditions change, without waiting for server round-trip.
 *
 * Examples:
 * - {myState1 && <div>Content</div>}
 * - {myState1 && !myState2 && <div>{myState3}</div>}
 * - {condition ? <Active /> : <Inactive />}
 *
 * The client can evaluate simple boolean conditions and construct the full
 * DOM tree from the template, providing instant feedback.
 */

const t = require('@babel/types');
const generate = require('@babel/generator').default;

/**
 * Extract conditional element templates from JSX render body
 *
 * Returns object keyed by hex path:
 * {
 *   "1.2": {
 *     type: "conditional-element",
 *     conditionExpression: "myState1",
 *     conditionBindings: ["state_0"],  // ← Resolved to state keys!
 *     branches: {
 *       true: { element structure },
 *       false: null
 *     }
 *   }
 * }
 */
function extractConditionalElementTemplates(renderBody, component) {
  if (!renderBody) return {};

  const conditionalTemplates = {};

  // Build mapping from variable name → state key
  const stateKeyMap = new Map();

  // Map useState variables to state_N keys
  if (component.useState) {
    component.useState.forEach((state, index) => {
      stateKeyMap.set(state.name, `state_${index}`);
    });
  }

  // Map useRef variables to ref_N keys
  if (component.useRef) {
    component.useRef.forEach((ref, index) => {
      stateKeyMap.set(ref.name, `ref_${index}`);
    });
  }

  // Map props (they use the prop name as-is)
  if (component.props) {
    component.props.forEach((prop) => {
      stateKeyMap.set(prop.name, prop.name);
    });
  }

  /**
   * Traverse JSX tree to find conditional expressions
   * @param {*} node - JSX node to traverse
   * @param {string|null} parentPath - Hex path of parent conditional (for nesting)
   */
  function traverseJSX(node, parentPath = null) {
    if (t.isJSXElement(node)) {
      // Process children
      for (const child of node.children) {
        if (t.isJSXExpressionContainer(child)) {
          const expr = child.expression;

          // Logical AND: {condition && <Element />}
          if (t.isLogicalExpression(expr) && expr.operator === '&&') {
            const template = extractLogicalAndElementTemplate(expr, child, parentPath);
            if (template) {
              const path = child.__minimactPath;
              if (path) {
                conditionalTemplates[path] = template;
                // Recursively find nested conditionals inside this template
                traverseJSX(expr.right, path);
              }
            }
          }

          // Ternary: {condition ? <A /> : <B />}
          if (t.isConditionalExpression(expr)) {
            const template = extractTernaryElementTemplate(expr, child, parentPath);
            if (template) {
              const path = child.__minimactPath;
              if (path) {
                conditionalTemplates[path] = template;
                // Recursively find nested conditionals in both branches
                if (expr.consequent) {
                  traverseJSX(expr.consequent, path);
                }
                if (expr.alternate) {
                  traverseJSX(expr.alternate, path);
                }
              }
            }
          }
        } else if (t.isJSXElement(child)) {
          traverseJSX(child, parentPath);
        }
      }
    }
  }

  /**
   * Extract template from logical AND expression
   * Example: {myState1 && !myState2 && <div>{myState3}</div>}
   * @param {*} parentPath - Hex path of parent conditional template (for nesting)
   */
  function extractLogicalAndElementTemplate(expr, containerNode, parentPath) {
    const right = expr.right;

    // Check if right side is JSX element (structural)
    if (!t.isJSXElement(right) && !t.isJSXFragment(right)) {
      return null;
    }

    // Extract full condition expression
    const condition = extractLeftSideOfAnd(expr);
    const conditionCode = generate(condition).code;

    // Extract bindings from condition (variable names)
    const variableNames = extractBindingsFromCondition(condition);

    // Build mapping from variable names to state keys
    const conditionMapping = {};
    const stateKeys = [];

    for (const varName of variableNames) {
      const stateKey = stateKeyMap.get(varName) || varName;
      conditionMapping[varName] = stateKey;
      stateKeys.push(stateKey);
    }

    // Can we evaluate this condition client-side?
    const isEvaluable = isConditionEvaluableClientSide(condition, variableNames);

    // Extract element structure
    const elementStructure = extractElementStructure(right);

    if (!elementStructure) {
      return null;
    }

    const template = {
      type: "conditional-element",
      conditionExpression: conditionCode,
      conditionBindings: stateKeys, // ["state_0", "state_1"]
      conditionMapping: conditionMapping, // { "myState1": "state_0", "myState2": "state_1" }
      evaluable: isEvaluable,
      branches: {
        true: elementStructure,
        false: null
      },
      operator: "&&"
    };

    // Add parent reference if nested
    if (parentPath) {
      template.parentTemplate = parentPath;
    }

    return template;
  }

  /**
   * Extract template from ternary expression
   * Example: {myState1 ? <div>Active</div> : <div>Inactive</div>}
   * @param {*} parentPath - Hex path of parent conditional template (for nesting)
   */
  function extractTernaryElementTemplate(expr, containerNode, parentPath) {
    const test = expr.test;
    const consequent = expr.consequent;
    const alternate = expr.alternate;

    // Check if branches are JSX elements
    const hasConsequent = t.isJSXElement(consequent) || t.isJSXFragment(consequent);
    const hasAlternate = t.isJSXElement(alternate) || t.isJSXFragment(alternate) || t.isNullLiteral(alternate);

    if (!hasConsequent && !hasAlternate) {
      return null; // Not a structural template
    }

    // Extract condition
    const conditionCode = generate(test).code;
    const variableNames = extractBindingsFromCondition(test);

    // Build mapping from variable names to state keys
    const conditionMapping = {};
    const stateKeys = [];

    for (const varName of variableNames) {
      const stateKey = stateKeyMap.get(varName) || varName;
      conditionMapping[varName] = stateKey;
      stateKeys.push(stateKey);
    }

    const isEvaluable = isConditionEvaluableClientSide(test, variableNames);

    // Extract both branches
    const branches = {};

    if (hasConsequent) {
      branches.true = extractElementStructure(consequent);
    }

    if (hasAlternate) {
      if (t.isNullLiteral(alternate)) {
        branches.false = null;
      } else {
        branches.false = extractElementStructure(alternate);
      }
    }

    const template = {
      type: "conditional-element",
      conditionExpression: conditionCode,
      conditionBindings: stateKeys, // ["state_0", "state_1"]
      conditionMapping: conditionMapping, // { "myState1": "state_0", "myState2": "state_1" }
      evaluable: isEvaluable,
      branches,
      operator: "?"
    };

    // Add parent reference if nested
    if (parentPath) {
      template.parentTemplate = parentPath;
    }

    return template;
  }

  /**
   * Extract the left side of a chained AND expression
   * Example: myState1 && !myState2 && <div /> → returns myState1 && !myState2
   */
  function extractLeftSideOfAnd(expr) {
    if (!t.isLogicalExpression(expr) || expr.operator !== '&&') {
      return expr;
    }

    const right = expr.right;

    // If right is JSX, left is the condition
    if (t.isJSXElement(right) || t.isJSXFragment(right)) {
      return expr.left;
    }

    // Otherwise, keep recursing
    return expr;
  }

  /**
   * Extract all state bindings from a condition expression
   * Example: myState1 && !myState2 → ["myState1", "myState2"]
   */
  function extractBindingsFromCondition(expr) {
    const bindings = new Set();

    function traverse(node) {
      if (t.isIdentifier(node)) {
        bindings.add(node.name);
      } else if (t.isLogicalExpression(node)) {
        traverse(node.left);
        traverse(node.right);
      } else if (t.isUnaryExpression(node)) {
        traverse(node.argument);
      } else if (t.isBinaryExpression(node)) {
        traverse(node.left);
        traverse(node.right);
      } else if (t.isMemberExpression(node)) {
        const path = buildMemberPath(node);
        if (path) bindings.add(path);
      }
    }

    traverse(expr);
    return Array.from(bindings);
  }

  /**
   * Check if condition can be evaluated client-side
   * Only simple boolean logic is supported (&&, ||, !, comparisons)
   */
  function isConditionEvaluableClientSide(expr, bindings) {
    // Simple identifier: myState1
    if (t.isIdentifier(expr)) {
      return true;
    }

    // Unary: !myState1
    if (t.isUnaryExpression(expr) && expr.operator === '!') {
      return isConditionEvaluableClientSide(expr.argument, bindings);
    }

    // Logical: myState1 && myState2, myState1 || myState2
    if (t.isLogicalExpression(expr)) {
      return isConditionEvaluableClientSide(expr.left, bindings) &&
             isConditionEvaluableClientSide(expr.right, bindings);
    }

    // Binary comparisons: count > 0, name === "admin"
    if (t.isBinaryExpression(expr)) {
      // Simple comparisons are evaluable
      const operators = ['==', '===', '!=', '!==', '<', '>', '<=', '>='];
      if (operators.includes(expr.operator)) {
        return isSimpleExpression(expr.left) && isSimpleExpression(expr.right);
      }
    }

    // Member expressions: user.isAdmin
    if (t.isMemberExpression(expr)) {
      return true;
    }

    // Complex expressions require server evaluation
    return false;
  }

  /**
   * Check if expression is simple (identifier, member, literal)
   */
  function isSimpleExpression(expr) {
    return t.isIdentifier(expr) ||
           t.isMemberExpression(expr) ||
           t.isStringLiteral(expr) ||
           t.isNumericLiteral(expr) ||
           t.isBooleanLiteral(expr) ||
           t.isNullLiteral(expr);
  }

  /**
   * Extract complete element structure including dynamic content
   */
  function extractElementStructure(node) {
    if (t.isJSXElement(node)) {
      const tagName = node.openingElement.name.name;
      const hexPath = node.__minimactPath;

      // Extract attributes
      const attributes = {};
      for (const attr of node.openingElement.attributes) {
        if (t.isJSXAttribute(attr)) {
          const attrName = attr.name.name;
          const attrValue = attr.value;

          if (!attrValue) {
            attributes[attrName] = true; // Boolean attribute
          } else if (t.isStringLiteral(attrValue)) {
            attributes[attrName] = attrValue.value;
          } else if (t.isJSXExpressionContainer(attrValue)) {
            // Dynamic attribute
            const expr = attrValue.expression;
            if (t.isIdentifier(expr)) {
              attributes[attrName] = { binding: expr.name };
            } else if (t.isMemberExpression(expr)) {
              attributes[attrName] = { binding: buildMemberPath(expr) };
            } else {
              attributes[attrName] = { expression: generate(expr).code };
            }
          }
        }
      }

      // Extract children
      const children = [];
      for (const child of node.children) {
        if (t.isJSXText(child)) {
          const text = child.value.trim();
          if (text) {
            children.push({
              type: "text",
              value: text,
              hexPath: child.__minimactPath
            });
          }
        } else if (t.isJSXElement(child)) {
          const childStructure = extractElementStructure(child);
          if (childStructure) {
            children.push(childStructure);
          }
        } else if (t.isJSXExpressionContainer(child)) {
          // Dynamic text content
          const expr = child.expression;
          if (t.isIdentifier(expr)) {
            children.push({
              type: "text",
              binding: expr.name,
              hexPath: child.__minimactPath
            });
          } else if (t.isMemberExpression(expr)) {
            children.push({
              type: "text",
              binding: buildMemberPath(expr),
              hexPath: child.__minimactPath
            });
          } else {
            // Complex expression
            children.push({
              type: "text",
              expression: generate(expr).code,
              hexPath: child.__minimactPath
            });
          }
        }
      }

      return {
        type: "element",
        tag: tagName,
        hexPath,
        attributes,
        children
      };
    } else if (t.isJSXFragment(node)) {
      const children = [];
      for (const child of node.children) {
        if (t.isJSXElement(child)) {
          const childStructure = extractElementStructure(child);
          if (childStructure) {
            children.push(childStructure);
          }
        }
      }

      return {
        type: "fragment",
        children
      };
    }

    return null;
  }

  /**
   * Build member expression path (user.profile.name → "user.profile.name")
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

    return parts.length > 0 ? parts.join('.') : null;
  }

  // Start traversal
  traverseJSX(renderBody);

  return conditionalTemplates;
}

module.exports = {
  extractConditionalElementTemplates
};
