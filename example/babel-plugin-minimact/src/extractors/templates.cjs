/**
 * Template Extractor for Hot Reload
 *
 * Extracts parameterized templates from JSX text nodes for instant hot reload.
 * This enables 100% coverage with minimal memory (2KB vs 100KB per component).
 *
 * Architecture:
 * - Build time: Extract templates with {0}, {1} placeholders
 * - Runtime: Re-hydrate templates with current state values
 * - Hot reload: Send template patches instead of re-rendering
 */

const t = require('@babel/types');
const { getPathFromNode, getPathSegmentsFromNode } = require('../utils/pathAssignment.cjs');

/**
 * Shared helper: Extract identifiers from expression (module-level for reuse)
 */
function extractIdentifiersShared(expr, result) {
  if (t.isIdentifier(expr)) {
    result.push(expr.name);
  } else if (t.isBinaryExpression(expr) || t.isLogicalExpression(expr)) {
    extractIdentifiersShared(expr.left, result);
    extractIdentifiersShared(expr.right, result);
  } else if (t.isUnaryExpression(expr)) {
    extractIdentifiersShared(expr.argument, result);
  } else if (t.isMemberExpression(expr)) {
    result.push(buildMemberPathShared(expr));
  }
}

/**
 * Shared helper: Build member expression path
 */
function buildMemberPathShared(expr) {
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
 * Shared helper: Extract method call binding
 * Handles: price.toFixed(2), text.toLowerCase(), etc.
 */
function extractMethodCallBindingShared(expr) {
  const callee = expr.callee;

  if (!t.isMemberExpression(callee) && !t.isOptionalMemberExpression(callee)) {
    return null;
  }

  const methodName = t.isIdentifier(callee.property) ? callee.property.name : null;
  if (!methodName) return null;

  const transformMethods = [
    'toFixed', 'toString', 'toLowerCase', 'toUpperCase',
    'trim', 'trimStart', 'trimEnd'
  ];

  if (!transformMethods.includes(methodName)) {
    return null;
  }

  let binding = null;
  if (t.isMemberExpression(callee.object)) {
    binding = buildMemberPathShared(callee.object);
  } else if (t.isIdentifier(callee.object)) {
    binding = callee.object.name;
  } else if (t.isBinaryExpression(callee.object)) {
    const identifiers = [];
    extractIdentifiersShared(callee.object, identifiers);
    binding = `__expr__:${identifiers.join(',')}`;
  }

  if (!binding) return null;

  const args = expr.arguments.map(arg => {
    if (t.isNumericLiteral(arg)) return arg.value;
    if (t.isStringLiteral(arg)) return arg.value;
    if (t.isBooleanLiteral(arg)) return arg.value;
    return null;
  }).filter(v => v !== null);

  return {
    transform: methodName,
    binding: binding,
    args: args
  };
}

/**
 * Check if expression is a .map() call (including chained calls like .filter().map())
 */
function isMapCallExpression(expr) {
  if (!t.isCallExpression(expr)) {
    return false;
  }

  // Check if it's a direct .map() call
  if (t.isMemberExpression(expr.callee) &&
      t.isIdentifier(expr.callee.property) &&
      expr.callee.property.name === 'map') {
    return true;
  }

  // Check if it's a chained call ending in .map()
  // e.g., items.filter(...).map(...), items.slice(0, 10).map(...)
  let current = expr;
  while (t.isCallExpression(current)) {
    if (t.isMemberExpression(current.callee) &&
        t.isIdentifier(current.callee.property) &&
        current.callee.property.name === 'map') {
      return true;
    }
    // Move to the next call in the chain
    if (t.isMemberExpression(current.callee)) {
      current = current.callee.object;
    } else {
      break;
    }
  }

  return false;
}

/**
 * Shared helper: Extract binding from expression
 */
function extractBindingShared(expr, component) {
  if (t.isIdentifier(expr)) {
    return expr.name;
  } else if (t.isMemberExpression(expr)) {
    return buildMemberPathShared(expr);
  } else if (t.isCallExpression(expr)) {
    // First try method call binding (toFixed, etc.)
    const methodBinding = extractMethodCallBindingShared(expr);
    if (methodBinding) {
      return methodBinding;
    }

    // Otherwise, handle chained method calls: todo.text.substring(0, 10).toUpperCase()
    return extractComplexCallExpression(expr);
  } else if (t.isBinaryExpression(expr)) {
    // Handle binary expressions: todo.priority + 1, price * quantity, etc.
    return extractBinaryExpressionBinding(expr);
  } else if (t.isLogicalExpression(expr)) {
    // Handle logical expressions: todo.dueDate || 'No due date'
    return extractLogicalExpressionBinding(expr);
  } else if (t.isUnaryExpression(expr)) {
    // Handle unary expressions: !todo.completed
    return extractUnaryExpressionBinding(expr);
  } else {
    return null;
  }
}

/**
 * Extract binding from binary expression
 * Examples: todo.priority + 1, price * quantity, index * 2 + 1
 */
function extractBinaryExpressionBinding(expr) {
  const identifiers = [];
  extractIdentifiersShared(expr, identifiers);

  // Use __expr__ prefix to indicate this is a computed expression
  return `__expr__:${identifiers.join(',')}`;
}

/**
 * Extract binding from logical expression
 * Examples: todo.dueDate || 'No due date', condition && value
 */
function extractLogicalExpressionBinding(expr) {
  const identifiers = [];
  extractIdentifiersShared(expr, identifiers);

  // Use __expr__ prefix to indicate this is a computed expression
  return `__expr__:${identifiers.join(',')}`;
}

/**
 * Extract binding from unary expression
 * Examples: !todo.completed, -value
 */
function extractUnaryExpressionBinding(expr) {
  const identifiers = [];
  extractIdentifiersShared(expr, identifiers);

  // Use __expr__ prefix to indicate this is a computed expression
  return `__expr__:${identifiers.join(',')}`;
}

/**
 * Extract binding from complex call expression (non-transform methods)
 * Examples: todo.text.substring(0, 10).toUpperCase(), array.concat(other)
 */
function extractComplexCallExpression(expr) {
  const identifiers = [];
  extractIdentifiersShared(expr, identifiers);

  if (identifiers.length === 0) {
    return null;
  }

  // Use __expr__ prefix to indicate this is a computed expression
  return `__expr__:${identifiers.join(',')}`;
}

/**
 * Shared helper: Extract template literal (module-level for reuse)
 */
function extractTemplateLiteralShared(node, component) {
  let templateStr = '';
  const bindings = [];
  const slots = [];
  const transforms = [];
  const conditionals = [];

  for (let i = 0; i < node.quasis.length; i++) {
    const quasi = node.quasis[i];
    templateStr += quasi.value.raw;

    if (i < node.expressions.length) {
      const expr = node.expressions[i];
      slots.push(templateStr.length);
      templateStr += `{${i}}`;

      const binding = extractBindingShared(expr, component);

      if (binding && typeof binding === 'object' && binding.transform) {
        bindings.push(binding.binding);
        transforms.push({
          slotIndex: i,
          method: binding.transform,
          args: binding.args
        });
      } else if (binding) {
        bindings.push(binding);
      } else {
        bindings.push('__complex__');
      }
    }
  }

  const result = {
    template: templateStr,
    bindings,
    slots,
    type: 'attribute'
  };

  if (transforms.length > 0) {
    result.transforms = transforms;
  }
  if (conditionals.length > 0) {
    result.conditionals = conditionals;
  }

  return result;
}

/**
 * Extract all templates from JSX render body
 *
 * Returns a map of node paths to templates:
 * {
 *   "div[0].h1[0].text": {
 *     template: "Count: {0}",
 *     bindings: ["count"],
 *     slots: [7],
 *     path: [0, 0]
 *   }
 * }
 */
function extractTemplates(renderBody, component) {
  if (!renderBody) return {};

  const templates = {};
  let templateCounter = 0;

  // Build path stack for tracking node positions
  const pathStack = [];

  /**
   * Traverse JSX tree and extract text templates
   */
  function traverseJSX(node, parentPath = [], siblingCounts = {}) {
    if (t.isJSXElement(node)) {
      const tagName = node.openingElement.name.name;

      // 🔥 USE PRE-ASSIGNED HEX PATH (no recalculation!)
      const pathKey = node.__minimactPath || null;
      if (!pathKey) {
        throw new Error(`[Template Extractor] No __minimactPath found on <${tagName}>. Did assignPathsToJSX run first?`);
      }

      // For backward compatibility with attribute extraction that expects array paths
      const currentPath = getPathSegmentsFromNode(node);

      pathStack.push({ tag: tagName, path: pathKey });

      // Process children
      let textNodeIndex = 0;

      // First pass: Identify text/expression children and check for mixed content
      const textChildren = [];
      let hasTextNodes = false;
      let hasExpressionNodes = false;

      for (const child of node.children) {
        if (t.isJSXText(child)) {
          const text = child.value.trim();
          if (text) {
            textChildren.push(child);
            hasTextNodes = true;
          }
        } else if (t.isJSXExpressionContainer(child)) {
          const expr = child.expression;

          // Skip structural JSX
          const isStructural = t.isJSXElement(expr) ||
                               t.isJSXFragment(expr) ||
                               t.isJSXEmptyExpression(expr) ||
                               (t.isLogicalExpression(expr) &&
                                (t.isJSXElement(expr.right) || t.isJSXFragment(expr.right))) ||
                               (t.isConditionalExpression(expr) &&
                                (t.isJSXElement(expr.consequent) || t.isJSXElement(expr.alternate) ||
                                 t.isJSXFragment(expr.consequent) || t.isJSXFragment(expr.alternate))) ||
                               isMapCallExpression(expr);

          if (!isStructural) {
            textChildren.push(child);
            hasExpressionNodes = true;
          }
        }
      }

      // Second pass: Process text content
      if (textChildren.length > 0) {
        // Check if this is mixed content (text + expressions together)
        const isMixedContent = hasTextNodes && hasExpressionNodes;

        if (isMixedContent) {
          // Mixed content: process all children together as one template
          // Use the first child's hex path as the template path
          const firstTextChild = textChildren[0];
          const textPath = firstTextChild.__minimactPath || `${pathKey}.text[${textNodeIndex}]`;

          const template = extractTextTemplate(node.children, currentPath, textNodeIndex);
          if (template) {
            console.log(`[Template Extractor] Found mixed content in <${tagName}>: "${template.template.substring(0, 50)}" (path: ${textPath})`);
            templates[textPath] = template;
            textNodeIndex++;
          }
        } else {
          // Pure text or pure expressions: process each separately
          for (const child of textChildren) {
            if (t.isJSXText(child)) {
              const text = child.value.trim();
              if (text) {
                // 🔥 USE PRE-ASSIGNED HEX PATH for text nodes
                const textPath = child.__minimactPath || `${pathKey}.text[${textNodeIndex}]`;
                console.log(`[Template Extractor] Found static text in <${tagName}>: "${text}" (path: ${textPath})`);
                templates[textPath] = {
                  template: text,
                  bindings: [],
                  slots: [],
                  path: getPathSegmentsFromNode(child),
                  type: 'static'
                };
                textNodeIndex++;
              }
            } else if (t.isJSXExpressionContainer(child)) {
              // Pure expression: extract template for this child only
              // 🔥 USE PRE-ASSIGNED HEX PATH for expression containers
              const exprPath = child.__minimactPath || `${pathKey}.text[${textNodeIndex}]`;

              const template = extractTextTemplate([child], currentPath, textNodeIndex);
              if (template) {
                console.log(`[Template Extractor] Found dynamic expression in <${tagName}>: "${template.template}" (path: ${exprPath})`);
                templates[exprPath] = template;
                textNodeIndex++;
              }
            }
          }
        }
      }

      // Third pass: Traverse JSXElement children
      const childSiblingCounts = {}; // Fresh sibling counts for children
      for (const child of node.children) {
        if (t.isJSXElement(child)) {
          traverseJSX(child, currentPath, childSiblingCounts);
        } else if (t.isJSXExpressionContainer(child)) {
          const expr = child.expression;

          // Traverse conditional JSX branches to extract templates from their content
          // This handles: {condition && <div>...</div>} and {condition ? <A/> : <B/>}
          if (t.isLogicalExpression(expr) && expr.operator === '&&') {
            // Logical AND: {isAdmin && <div>Admin Panel</div>}
            if (t.isJSXElement(expr.right)) {
              console.log(`[Template Extractor] Traversing conditional branch (&&) in <${tagName}>`);
              traverseJSX(expr.right, currentPath, childSiblingCounts);
            }
          } else if (t.isConditionalExpression(expr)) {
            // Ternary: {isAdmin ? <AdminPanel/> : <UserPanel/>}
            if (t.isJSXElement(expr.consequent)) {
              console.log(`[Template Extractor] Traversing conditional branch (? consequent) in <${tagName}>`);
              traverseJSX(expr.consequent, currentPath, childSiblingCounts);
            }
            if (t.isJSXElement(expr.alternate)) {
              console.log(`[Template Extractor] Traversing conditional branch (? alternate) in <${tagName}>`);
              traverseJSX(expr.alternate, currentPath, childSiblingCounts);
            }
          }
        }
      }

      pathStack.pop();
    } else if (t.isJSXFragment(node)) {
      // Handle fragments
      const childSiblingCounts = {}; // Fresh sibling counts for fragment children
      for (const child of node.children) {
        if (t.isJSXElement(child)) {
          traverseJSX(child, parentPath, childSiblingCounts);
        }
      }
    }
  }

  /**
   * Extract template from mixed text/expression children
   * Example: <h1>Count: {count}</h1> → "Count: {0}"
   */
  function extractTextTemplate(children, currentPath, textIndex) {
    let templateStr = '';
    const bindings = [];
    const slots = [];
    let paramIndex = 0;
    let hasExpressions = false;
    let conditionalTemplates = null;
    let transformMetadata = null;
    let nullableMetadata = null;

    for (const child of children) {
      if (t.isJSXText(child)) {
        const text = child.value;
        templateStr += text;
      } else if (t.isJSXExpressionContainer(child)) {
        hasExpressions = true;

        // Special case: Template literal inside JSX expression container
        // Example: {`${(discount * 100).toFixed(0)}%`}
        if (t.isTemplateLiteral(child.expression)) {
          const templateResult = extractTemplateLiteralShared(child.expression, component);
          if (templateResult) {
            // Merge the template literal's content into the current template
            templateStr += templateResult.template;
            // Add the template literal's bindings
            for (const binding of templateResult.bindings) {
              bindings.push(binding);
            }
            // Store transforms and conditionals if present
            if (templateResult.transforms && templateResult.transforms.length > 0) {
              transformMetadata = templateResult.transforms[0]; // Simplified: take first transform
            }
            if (templateResult.conditionals && templateResult.conditionals.length > 0) {
              conditionalTemplates = {
                true: templateResult.conditionals[0].trueValue,
                false: templateResult.conditionals[0].falseValue
              };
            }
            paramIndex++;
            continue; // Skip normal binding extraction
          }
        }

        const binding = extractBinding(child.expression, component);

        if (binding && typeof binding === 'object' && binding.conditional) {
          // Conditional binding (ternary)
          slots.push(templateStr.length);
          templateStr += `{${paramIndex}}`;
          bindings.push(binding.conditional);

          // Store conditional template values
          conditionalTemplates = {
            true: binding.trueValue,
            false: binding.falseValue
          };

          paramIndex++;
        } else if (binding && typeof binding === 'object' && binding.transform) {
          // Phase 1: Transform binding (method call)
          slots.push(templateStr.length);
          templateStr += `{${paramIndex}}`;
          bindings.push(binding.binding);

          // Store transform metadata
          transformMetadata = {
            method: binding.transform,
            args: binding.args
          };

          paramIndex++;
        } else if (binding && typeof binding === 'object' && binding.nullable) {
          // Phase 2: Nullable binding (optional chaining)
          slots.push(templateStr.length);
          templateStr += `{${paramIndex}}`;
          bindings.push(binding.binding);

          // Mark as nullable
          nullableMetadata = true;

          paramIndex++;
        } else if (binding) {
          // Simple binding (string)
          slots.push(templateStr.length);
          templateStr += `{${paramIndex}}`;
          bindings.push(binding);
          paramIndex++;
        } else {
          // Complex expression - can't template it
          templateStr += `{${paramIndex}}`;
          bindings.push('__complex__');
          paramIndex++;
        }
      }
    }

    // Clean up whitespace
    templateStr = templateStr.trim();

    if (!hasExpressions) return null;

    // Determine template type
    let templateType = 'dynamic';
    if (conditionalTemplates) {
      templateType = 'conditional';
    } else if (transformMetadata) {
      templateType = 'transform';
    } else if (nullableMetadata) {
      templateType = 'nullable';
    }

    const result = {
      template: templateStr,
      bindings,
      slots,
      path: currentPath,  // Already has full hex path from getPathSegmentsFromNode
      type: templateType
    };

    // Add conditional template values if present
    if (conditionalTemplates) {
      result.conditionalTemplates = conditionalTemplates;
    }

    // Add transform metadata if present
    if (transformMetadata) {
      result.transform = transformMetadata;
    }

    // Add nullable flag if present
    if (nullableMetadata) {
      result.nullable = true;
    }

    return result;
  }

  /**
   * Extract binding name from expression
   * Supports:
   * - Identifiers: {count}
   * - Member expressions: {user.name}
   * - Simple operations: {count + 1}
   * - Conditionals: {isExpanded ? 'Hide' : 'Show'}
   * - Method calls: {price.toFixed(2)}
   * - Optional chaining: {viewModel?.userEmail}
   */
  function extractBinding(expr, component) {
    if (t.isIdentifier(expr)) {
      return expr.name;
    } else if (t.isMemberExpression(expr)) {
      return buildMemberPath(expr);
    } else if (t.isOptionalMemberExpression(expr)) {
      // Phase 2: Optional chaining (viewModel?.userEmail)
      return extractOptionalChainBinding(expr);
    } else if (t.isCallExpression(expr)) {
      // Phase 1: Method calls (price.toFixed(2))
      return extractMethodCallBinding(expr);
    } else if (t.isBinaryExpression(expr) || t.isUnaryExpression(expr)) {
      // Simple operations - extract all identifiers
      const identifiers = [];
      extractIdentifiers(expr, identifiers);
      return identifiers.join('.');
    } else if (t.isConditionalExpression(expr)) {
      // Ternary expression: {isExpanded ? 'Hide' : 'Show'}
      // Return special marker that will be processed into conditional template
      return extractConditionalBinding(expr);
    } else {
      // Complex expression
      return null;
    }
  }

  /**
   * Extract conditional binding from ternary expression
   * Returns object with test identifier and consequent/alternate values
   * Example: isExpanded ? 'Hide' : 'Show'
   * Returns: { conditional: 'isExpanded', trueValue: 'Hide', falseValue: 'Show' }
   */
  function extractConditionalBinding(expr) {
    // Check if test is a simple identifier
    if (!t.isIdentifier(expr.test)) {
      // Complex test condition - mark as complex
      return null;
    }

    // Check if consequent and alternate are literals
    const trueValue = extractLiteralValue(expr.consequent);
    const falseValue = extractLiteralValue(expr.alternate);

    if (trueValue === null || falseValue === null) {
      // Not simple literals - mark as complex
      return null;
    }

    // Return conditional template metadata
    return {
      conditional: expr.test.name,
      trueValue,
      falseValue
    };
  }

  /**
   * Extract literal value from node (string, number, boolean)
   */
  function extractLiteralValue(node) {
    if (t.isStringLiteral(node)) {
      return node.value;
    } else if (t.isNumericLiteral(node)) {
      return node.value.toString();
    } else if (t.isBooleanLiteral(node)) {
      return node.value.toString();
    } else {
      return null;
    }
  }

  /**
   * Extract method call binding (Phase 1)
   * Handles: price.toFixed(2), text.toLowerCase(), etc.
   * Returns: { transform: 'toFixed', binding: 'price', args: [2] }
   */
  function extractMethodCallBinding(expr) {
    const callee = expr.callee;

    // Only handle method calls (obj.method()), not function calls (func())
    if (!t.isMemberExpression(callee) && !t.isOptionalMemberExpression(callee)) {
      return null;
    }

    const methodName = t.isIdentifier(callee.property) ? callee.property.name : null;
    if (!methodName) {
      return null;
    }

    // Supported transformation methods
    const transformMethods = [
      'toFixed', 'toString', 'toLowerCase', 'toUpperCase',
      'trim', 'trimStart', 'trimEnd'
    ];

    if (!transformMethods.includes(methodName)) {
      return null; // Unsupported method - mark as complex
    }

    // Extract the object being called (price from price.toFixed(2))
    let binding = null;
    if (t.isMemberExpression(callee.object)) {
      binding = buildMemberPath(callee.object);
    } else if (t.isOptionalMemberExpression(callee.object)) {
      binding = buildOptionalMemberPath(callee.object);
    } else if (t.isIdentifier(callee.object)) {
      binding = callee.object.name;
    } else if (t.isBinaryExpression(callee.object)) {
      // Handle expressions like (discount * 100).toFixed(0)
      // Extract all identifiers from the binary expression
      const identifiers = [];
      extractIdentifiers(callee.object, identifiers);
      binding = `__expr__:${identifiers.join(',')}`;
    }

    if (!binding) {
      return null; // Can't extract binding
    }

    // Extract method arguments (e.g., 2 from toFixed(2))
    const args = expr.arguments.map(arg => {
      if (t.isNumericLiteral(arg)) return arg.value;
      if (t.isStringLiteral(arg)) return arg.value;
      if (t.isBooleanLiteral(arg)) return arg.value;
      return null;
    }).filter(v => v !== null);

    // Return transform binding metadata
    return {
      transform: methodName,
      binding: binding,
      args: args
    };
  }

  /**
   * Extract optional chaining binding (Phase 2)
   * Handles: viewModel?.userEmail, obj?.prop1?.prop2
   * Returns: { nullable: true, binding: 'viewModel.userEmail' }
   */
  function extractOptionalChainBinding(expr) {
    const path = buildOptionalMemberPath(expr);

    if (!path) {
      return null; // Can't build path
    }

    return {
      nullable: true,
      binding: path
    };
  }

  /**
   * Build optional member expression path: viewModel?.userEmail → "viewModel.userEmail"
   */
  function buildOptionalMemberPath(expr) {
    const parts = [];
    let current = expr;

    while (t.isOptionalMemberExpression(current) || t.isMemberExpression(current)) {
      if (t.isIdentifier(current.property)) {
        parts.unshift(current.property.name);
      } else {
        return null; // Computed property
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
   * Build member expression path: user.name → "user.name"
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
    } else if (t.isBinaryExpression(expr) || t.isLogicalExpression(expr)) {
      extractIdentifiers(expr.left, result);
      extractIdentifiers(expr.right, result);
    } else if (t.isUnaryExpression(expr)) {
      extractIdentifiers(expr.argument, result);
    } else if (t.isMemberExpression(expr)) {
      result.push(buildMemberPath(expr));
    }
  }

  /**
   * Build path key for template map
   * Example: div[0].h1[0].text → "div[0].h1[0]"
   */
  function buildPathKey(tagName, index, parentPath) {
    const parentKeys = [];
    let currentPath = parentPath;

    // Build parent path from indices
    // This is simplified - in production we'd track tag names
    for (let i = 0; i < currentPath.length; i++) {
      parentKeys.push(`[${currentPath[i]}]`);
    }

    return `${parentKeys.join('.')}.${tagName}[${index}]`.replace(/^\./, '');
  }

  // Start traversal
  traverseJSX(renderBody);

  return templates;
}

/**
 * Extract templates for attributes (props)
 * Supports:
 * - Template literals: className={`count-${count}`}
 * - Style objects: style={{ fontSize: '32px', color: isActive ? 'red' : 'blue' }}
 * - Static string attributes: className="btn-primary"
 */
function extractAttributeTemplates(renderBody, component) {
  const templates = {};

  // Traverse JSX tree using pre-assigned hex paths
  function traverseJSX(node) {
    if (t.isJSXElement(node)) {
      const tagName = node.openingElement.name.name;

      // 🔥 USE PRE-ASSIGNED HEX PATH (no recalculation!)
      const elementPath = node.__minimactPath;
      if (!elementPath) {
        throw new Error(`[Attribute Extractor] No __minimactPath found on <${tagName}>. Did assignPathsToJSX run first?`);
      }

      const currentPath = getPathSegmentsFromNode(node);

      // Check attributes for template expressions
      for (const attr of node.openingElement.attributes) {
        if (t.isJSXAttribute(attr)) {
          const attrName = attr.name.name;
          const attrValue = attr.value;

          // 🔥 USE PRE-ASSIGNED ATTRIBUTE PATH
          const attrPath = attr.__minimactPath || `${elementPath}.@${attrName}`;

          // 1. Template literal: className={`count-${count}`}
          if (t.isJSXExpressionContainer(attrValue) && t.isTemplateLiteral(attrValue.expression)) {
            const template = extractTemplateLiteralShared(attrValue.expression, component);
            if (template) {
              console.log(`[Attribute Template] Found template literal in ${attrName}: "${template.template}" (path: ${attrPath})`);
              templates[attrPath] = {
                ...template,
                path: currentPath,
                attribute: attrName,
                type: template.bindings.length > 0 ? 'attribute-dynamic' : 'attribute-static'
              };
            }
          }
          // 2. Style object: style={{ fontSize: '32px', opacity: isVisible ? 1 : 0.5 }}
          else if (attrName === 'style' && t.isJSXExpressionContainer(attrValue) && t.isObjectExpression(attrValue.expression)) {
            const styleTemplate = extractStyleObjectTemplate(attrValue.expression, tagName, null, null, currentPath, component);
            if (styleTemplate) {
              console.log(`[Attribute Template] Found style object: "${styleTemplate.template.substring(0, 60)}..." (path: ${attrPath})`);
              templates[attrPath] = styleTemplate;
            }
          }
          // 3. Static string attribute: className="btn-primary", placeholder="Enter name"
          else if (t.isStringLiteral(attrValue)) {
            console.log(`[Attribute Template] Found static attribute ${attrName}: "${attrValue.value}" (path: ${attrPath})`);
            templates[attrPath] = {
              template: attrValue.value,
              bindings: [],
              slots: [],
              path: currentPath,
              attribute: attrName,
              type: 'attribute-static'
            };
          }
          // 4. Simple expression (for future dynamic attribute support)
          else if (t.isJSXExpressionContainer(attrValue)) {
            const expr = attrValue.expression;
            // Check if it's a simple binding (identifier or member expression)
            if (t.isIdentifier(expr) || t.isMemberExpression(expr)) {
              const binding = t.isIdentifier(expr) ? expr.name : buildMemberPathShared(expr);
              console.log(`[Attribute Template] Found dynamic attribute ${attrName}: binding="${binding}" (path: ${attrPath})`);
              templates[attrPath] = {
                template: '{0}',
                bindings: [binding],
                slots: [0],
                path: currentPath,
                attribute: attrName,
                type: 'attribute-dynamic'
              };
            }
          }
        }
      }

      // Traverse children (no need to track indices - paths are pre-assigned!)
      for (const child of node.children) {
        if (t.isJSXElement(child)) {
          traverseJSX(child);
        }
      }
    }
  }

  /**
   * Build attribute path key
   * Example: div[0].@style or div[1].@className
   */
  function buildAttributePathKey(tagName, index, parentPath, attrName) {
    const parentKeys = [];
    for (let i = 0; i < parentPath.length; i++) {
      parentKeys.push(`[${parentPath[i]}]`);
    }
    return `${parentKeys.join('.')}.${tagName}[${index}].@${attrName}`.replace(/^\./, '');
  }

  /**
   * Extract template from style object
   * Handles: { fontSize: '32px', opacity: isVisible ? 1 : 0.5 }
   */
  function extractStyleObjectTemplate(objectExpr, tagName, elementIndex, parentPath, currentPath, component) {
    const { convertStyleObjectToCss } = require('../utils/styleConverter.cjs');

    let hasBindings = false;
    const cssProperties = [];
    const bindings = [];
    const slots = [];
    let slotIndex = 0;

    // Check each property for dynamic values
    for (const prop of objectExpr.properties) {
      if (t.isObjectProperty(prop) && !prop.computed) {
        const key = t.isIdentifier(prop.key) ? prop.key.name : String(prop.key.value);
        const cssKey = camelToKebabShared(key);
        const value = prop.value;

        // Check if value is dynamic (expression, conditional, etc.)
        if (t.isConditionalExpression(value) || t.isIdentifier(value) || t.isMemberExpression(value)) {
          // Dynamic value - extract binding
          hasBindings = true;
          const binding = extractBindingShared(value, component);
          if (binding) {
            bindings.push(typeof binding === 'object' ? binding.binding || binding.conditional : binding);
            cssProperties.push(`${cssKey}: {${slotIndex}}`);
            slots.push(cssProperties.join('; ').lastIndexOf('{'));
            slotIndex++;
          } else {
            // Complex expression - fall back to static
            const cssValue = convertStyleValueShared(value);
            cssProperties.push(`${cssKey}: ${cssValue}`);
          }
        } else {
          // Static value
          const cssValue = convertStyleValueShared(value);
          cssProperties.push(`${cssKey}: ${cssValue}`);
        }
      }
    }

    const cssString = cssProperties.join('; ');

    return {
      template: cssString,
      bindings: bindings,
      slots: slots,
      path: currentPath,
      attribute: 'style',
      type: hasBindings ? 'attribute-dynamic' : 'attribute-static'
    };
  }

  /**
   * Convert camelCase to kebab-case (shared helper)
   */
  function camelToKebabShared(str) {
    return str.replace(/[A-Z]/g, letter => `-${letter.toLowerCase()}`);
  }

  /**
   * Convert style value to CSS string (shared helper)
   */
  function convertStyleValueShared(value) {
    if (t.isStringLiteral(value)) {
      return value.value;
    } else if (t.isNumericLiteral(value)) {
      return `${value.value}px`;
    } else if (t.isIdentifier(value)) {
      return value.name;
    }
    return String(value);
  }

  if (renderBody) {
    traverseJSX(renderBody);
  }

  return templates;
}

/**
 * Generate template map JSON file content
 */
function generateTemplateMapJSON(componentName, templates, attributeTemplates, conditionalElementTemplates = {}) {
  const allTemplates = {
    ...templates,
    ...attributeTemplates
  };

  const result = {
    component: componentName,
    version: '1.0',
    generatedAt: Date.now(),
    templates: Object.entries(allTemplates).reduce((acc, [path, template]) => {
      acc[path] = {
        template: template.template,
        bindings: template.bindings,
        slots: template.slots,
        path: template.path,
        type: template.type
      };

      // Include conditionalTemplates if present (for ternary expressions)
      if (template.conditionalTemplates) {
        acc[path].conditionalTemplates = template.conditionalTemplates;
      }

      // Include transform metadata if present (for method calls like toFixed)
      if (template.transform) {
        acc[path].transform = template.transform;
      }

      // Include nullable flag if present (for optional chaining)
      if (template.nullable) {
        acc[path].nullable = template.nullable;
      }

      return acc;
    }, {})
  };

  // Add conditional element templates if present
  if (Object.keys(conditionalElementTemplates).length > 0) {
    result.conditionalElements = conditionalElementTemplates;
  }

  return result;
}

/**
 * Add template metadata to component for C# code generation
 */
function addTemplateMetadata(component, templates) {
  component.templates = templates;

  // Add template bindings to track which state affects which templates
  component.templateBindings = new Map();

  for (const [path, template] of Object.entries(templates)) {
    for (const binding of template.bindings) {
      if (!component.templateBindings.has(binding)) {
        component.templateBindings.set(binding, []);
      }
      component.templateBindings.get(binding).push(path);
    }
  }
}

module.exports = {
  extractTemplates,
  extractAttributeTemplates,
  generateTemplateMapJSON,
  addTemplateMetadata
};
