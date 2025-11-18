/**
 * Simplified Babel Plugin - Enhanced with Real Patterns
 *
 * Based on babel-plugin-minimact, this demonstrates:
 * 1. Function components (function declarations & arrow functions)
 * 2. useState hook extraction with array destructuring
 * 3. Object destructuring for props
 * 4. path.traverse() for nested visitors
 * 5. Helper function calls
 * 6. Type annotations (TypeScript)
 * 7. Conditional logic
 * 8. JSX element traversal
 */

const t = require('@babel/types');
const { getComponentName } = require('./utils/helpers.cjs');
const { tsTypeToCSharpType } = require('./types/typeConversion.cjs');
const { extractHook } = require('./extractors/hooks.cjs');

module.exports = function(babel) {
  return {
    name: 'simple-transform',

    visitor: {
      Program: {
        enter(path, state) {
          state.components = [];
          console.log('[Simple Plugin] Starting transformation...');
        },

        exit(path, state) {
          console.log('[Simple Plugin] Components found:', state.components.length);
          state.components.forEach(comp => {
            console.log(`  - ${comp.name}: ${comp.hooks.length} hooks, ${comp.jsxElements.length} JSX elements`);
          });

          // Store components in file metadata for testing
          state.file.metadata = state.file.metadata || {};
          state.file.metadata.components = state.components;

          // Generate C# code
          const csharpCode = generateCSharpFile(state.components);
          console.log('[Simple Plugin] Generated C# code:');
          console.log(csharpCode);
        }
      },

      // Handle function declarations: function Counter() {}
      FunctionDeclaration(path, state) {
        const funcName = path.node.id.name;

        // Check if it's a component (starts with uppercase)
        if (funcName && funcName[0] === funcName[0].toUpperCase()) {
          processComponent(path, state, funcName);
        }
      },

      // Handle arrow functions: const Counter = () => {}
      ArrowFunctionExpression(path, state) {
        if (path.parent.type === 'VariableDeclarator') {
          const name = path.parent.id.name;
          if (name && name[0] === name[0].toUpperCase()) {
            processComponent(path, state, name);
          }
        }
      }
    }
  };
};

/**
 * Process a component function - extract hooks, props, JSX
 */
function processComponent(path, state, name) {
  // Use getComponentName helper to get the canonical component name
  const componentName = getComponentName(path) || name;

  const component = {
    name: componentName,
    props: extractProps(path),
    hooks: [],
    jsxElements: [],
    localVariables: [],
    helperFunctions: []
  };

  // Pattern: Traverse function body to find hooks and JSX
  path.traverse({
    // Pattern: Extract useState calls with array destructuring
    CallExpression(callPath) {
      if (t.isIdentifier(callPath.node.callee, { name: 'useState' })) {
        const hook = extractUseState(callPath);
        if (hook) {
          component.hooks.push(hook);
        }
      }
    },

    // Pattern: Extract local variables
    VariableDeclaration(varPath) {
      // Only extract variables at the top level of the function
      if (varPath.getFunctionParent() === path && varPath.parent.type === 'BlockStatement') {
        for (const decl of varPath.node.declarations) {
          if (t.isIdentifier(decl.id)) {
            const varInfo = {
              name: decl.id.name,
              hasInitializer: !!decl.init
            };
            component.localVariables.push(varInfo);
          }
        }
      }
    },

    // Pattern: Extract helper functions declared inside component
    FunctionDeclaration(funcPath) {
      if (funcPath.getFunctionParent() === path && funcPath.parent.type === 'BlockStatement') {
        const helperName = funcPath.node.id.name;
        component.helperFunctions.push({
          name: helperName,
          params: funcPath.node.params.length
        });
      }
    },

    // Pattern: Track JSX elements with attributes
    JSXElement(jsxPath) {
      const openingElement = jsxPath.node.openingElement;
      const elementName = t.isJSXIdentifier(openingElement.name)
        ? openingElement.name.name
        : 'unknown';

      const element = {
        type: elementName,
        attributes: extractAttributes(openingElement.attributes),
        hasChildren: jsxPath.node.children.length > 0
      };
      component.jsxElements.push(element);
    }
  });

  state.components.push(component);
}

/**
 * Pattern: Extract useState hook with array destructuring
 * Example: const [count, setCount] = useState(0)
 */
function extractUseState(callPath) {
  // Use extractHook helper from the extractors module
  const hookData = extractHook(callPath);

  if (hookData) {
    return hookData;
  }

  // Fallback to original implementation
  const parent = callPath.parent;

  // Pattern: const [state, setState] = useState(initialValue)
  if (t.isVariableDeclarator(parent) && t.isArrayPattern(parent.id)) {
    const elements = parent.id.elements;
    const [stateId, setStateId] = elements;
    const initialValue = callPath.node.arguments[0];

    return {
      type: 'useState',
      stateName: stateId ? stateId.name : null,
      setterName: setStateId ? setStateId.name : null,
      initialValue: getInitialValueString(initialValue),
      hasTypeAnnotation: !!stateId?.typeAnnotation
    };
  }

  return null;
}

/**
 * Pattern: Extract props from function parameters
 */
function extractProps(path) {
  const params = path.node.params;
  if (params.length === 0) return [];

  const firstParam = params[0];

  // Pattern: Object destructuring with TypeScript type annotation
  // function Component({ user, loading }: { user: User, loading: boolean })
  if (t.isObjectPattern(firstParam)) {
    const props = firstParam.properties.map(prop => {
      if (t.isObjectProperty(prop) && t.isIdentifier(prop.key)) {
        // Use tsTypeToCSharpType to convert TypeScript types to C# types
        let csharpType = 'object';
        if (prop.value.typeAnnotation && prop.value.typeAnnotation.typeAnnotation) {
          csharpType = tsTypeToCSharpType(prop.value.typeAnnotation.typeAnnotation);
        }

        return {
          name: prop.key.name,
          hasDefault: !!prop.value.default,
          hasTypeAnnotation: !!firstParam.typeAnnotation,
          csharpType: csharpType
        };
      }
      return null;
    }).filter(Boolean);

    return props;
  }

  // Pattern: Simple props parameter
  // function Component(props)
  if (t.isIdentifier(firstParam)) {
    return [{
      name: firstParam.name,
      hasDefault: false,
      hasTypeAnnotation: !!firstParam.typeAnnotation
    }];
  }

  return [];
}

/**
 * Pattern: Extract JSX attributes
 */
function extractAttributes(attributes) {
  return attributes.map(attr => {
    if (t.isJSXAttribute(attr)) {
      const attrInfo = {
        name: attr.name.name,
        value: getAttributeValue(attr.value),
        isExpression: t.isJSXExpressionContainer(attr.value)
      };
      return attrInfo;
    }
    // Pattern: Spread attributes {...props}
    if (t.isJSXSpreadAttribute(attr)) {
      return {
        name: '...spread',
        value: '<spread>',
        isExpression: true
      };
    }
    return null;
  }).filter(Boolean);
}

/**
 * Pattern: Get attribute value as string
 */
function getAttributeValue(value) {
  if (!value) return true; // Boolean attribute like <input disabled />
  if (t.isStringLiteral(value)) return value.value;
  if (t.isJSXExpressionContainer(value)) {
    // Pattern: Expression in JSX {count}, {user.name}, etc.
    if (t.isIdentifier(value.expression)) {
      return value.expression.name;
    }
    if (t.isMemberExpression(value.expression)) {
      return '<member-expr>';
    }
    if (t.isCallExpression(value.expression)) {
      return '<call-expr>';
    }
    return '<expression>';
  }
  return '<unknown>';
}

/**
 * Simple JSX code generation stub
 */
function generateJSX(element) {
  return `new ${element.tagName}()`;
}

/**
 * Pattern: Get initial value as string
 */
function getInitialValueString(node) {
  if (!node) return 'undefined';
  if (t.isNumericLiteral(node)) return node.value.toString();
  if (t.isStringLiteral(node)) return `"${node.value}"`;
  if (t.isBooleanLiteral(node)) return node.value.toString();
  if (t.isNullLiteral(node)) return 'null';
  if (t.isArrayExpression(node)) return `[${node.elements.length} items]`;
  if (t.isObjectExpression(node)) return `{${node.properties.length} props}`;
  if (t.isArrowFunctionExpression(node) || t.isFunctionExpression(node)) return '<function>';
  return '<expression>';
}

/**
 * Generate C# file from components
 * Pattern: Using actual babel-plugin-minimact generators
 */
function generateCSharpFile(components) {
  // Pattern: Require external module
  // const { generateCSharpFile: realGenerator } = require('../babel-plugin-minimact/src/generators/csharpFile.cjs');

  // Pattern: Call external function with state object
  // const state = { opts: { namespace: 'Minimact.Components' } };
  // return realGenerator(components, state);

  // Simplified version for testing
  return "// Generated C# code here";
}

/**
 * Generate a component class
 */
function generateComponent(component) {
  const lines = [];

  lines.push(`public class ${component.name} : MinimactComponent`);
  lines.push('{');

  // Pattern: Logical chain - check if hooks exist AND have length
  if (component.hooks && component.hooks.length > 0) {
    for (const hook of component.hooks) {
      if (hook.type === 'useState') {
        const csharpType = inferCSharpType(hook.initialValue);
        const csharpValue = convertToCSharp(hook.initialValue, csharpType);
        lines.push(`    private ${csharpType} ${hook.stateName} = ${csharpValue};`);
      }
    }
    lines.push('');
  }

  // Pattern: Member access - check props array length
  if (component.props && component.props.length > 0) {
    lines.push('    // Props detected');
    lines.push('');
  }

  // Render method
  lines.push('    protected override MinimactNode Render()');
  lines.push('    {');

  // Pattern: Generate JSX translation
  if (component.jsxElements && component.jsxElements.length > 0) {
    const jsxCode = generateJSX(component.jsxElements[0]); // Generate from root element
    lines.push(`        return ${jsxCode};`);
  } else {
    lines.push('        return null;');
  }

  lines.push('    }');

  lines.push('}');

  return lines;
}

/**
 * Pattern: Convert JavaScript value to C# syntax
 */
function convertToCSharp(jsValue, csharpType) {
  if (csharpType === 'string') {
    return jsValue; // Already has quotes
  }
  if (csharpType === 'List<object>') {
    return 'new()';
  }
  if (csharpType === 'object') {
    return 'new()';
  }
  return jsValue;
}

/**
 * Pattern: Infer C# type from JavaScript initial value
 * Pattern: Multiple return statements with early returns
 */
function inferCSharpType(initialValue) {
  // Pattern: Early return with truthiness check
  if (!initialValue || initialValue === 'undefined') return 'object';

  // Pattern: Regex test in condition
  if (/^\d+$/.test(initialValue)) return 'int';
  if (/^\d+\.\d+$/.test(initialValue)) return 'double';

  // Pattern: Simple equality checks
  if (initialValue === 'true' || initialValue === 'false') return 'bool';

  // Pattern: String method calls in conditions
  if (initialValue.startsWith('"')) return 'string';
  if (initialValue.startsWith('[')) return 'List<object>';
  if (initialValue.startsWith('{')) return 'object';

  // Pattern: Default return
  return 'object';
}

/**
 * Pattern: Helper function that formats a property line
 * Pattern: Template literal in return statement
 */
function formatProperty(name, type, value) {
  return `    private ${type} ${name} = ${value};`;
}
