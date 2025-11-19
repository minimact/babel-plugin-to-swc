/**
 * First Pass: Extract Parameterized Templates from Helper Functions
 *
 * This Babel plugin parses a Babel plugin's helper functions and extracts
 * each code pattern as a parameterized template with a content hash.
 *
 * Output: JSON with templates that can be manually annotated with Rust translations
 */

const crypto = require('crypto');

module.exports = function({ types: t }) {
  const templates = {};
  const helperFunctions = new Map();

  // Generate hash for template content
  function hashContent(content) {
    return crypto.createHash('sha256').update(content).digest('hex').slice(0, 8);
  }

  // Extract template from a node
  function extractTemplate(node, source) {
    const content = source.slice(node.start, node.end);
    const hash = hashContent(content);

    return {
      hash,
      content,
      node
    };
  }

  return {
    name: 'extract-templates',

    visitor: {
      // Collect require statements to identify helper modules
      VariableDeclaration(path) {
        const declaration = path.node.declarations[0];
        if (!declaration) return;

        // Match: const { fn } = require('./path')
        if (
          t.isCallExpression(declaration.init) &&
          t.isIdentifier(declaration.init.callee, { name: 'require' }) &&
          declaration.init.arguments[0] &&
          t.isStringLiteral(declaration.init.arguments[0])
        ) {
          const requirePath = declaration.init.arguments[0].value;

          // Only process local helper modules (not @babel/types etc)
          if (requirePath.startsWith('./') || requirePath.startsWith('../')) {
            if (t.isObjectPattern(declaration.id)) {
              declaration.id.properties.forEach(prop => {
                if (t.isIdentifier(prop.key)) {
                  helperFunctions.set(prop.key.name, {
                    source: requirePath,
                    imported: prop.key.name
                  });
                }
              });
            }
          }
        }
      },

      // Process function declarations in helper modules
      FunctionDeclaration(path, state) {
        const funcName = path.node.id?.name;
        if (!funcName) return;

        const source = state.file.code;
        const funcContent = source.slice(path.node.start, path.node.end);
        const hash = hashContent(funcContent);

        templates[hash] = {
          type: 'FunctionDeclaration',
          name: funcName,
          params: path.node.params.map(p => {
            if (t.isIdentifier(p)) return p.name;
            if (t.isAssignmentPattern(p) && t.isIdentifier(p.left)) return p.left.name;
            return null;
          }).filter(Boolean),
          babel_source: funcContent,
          body_templates: [],
          rust_translation: null
        };

        // Extract templates from function body
        path.traverse({
          // Conditional statements
          IfStatement(ifPath) {
            const ifContent = source.slice(ifPath.node.start, ifPath.node.end);
            const ifHash = hashContent(ifContent);

            templates[hash].body_templates.push({
              hash: ifHash,
              type: 'IfStatement',
              condition: source.slice(ifPath.node.test.start, ifPath.node.test.end),
              babel_source: ifContent,
              rust_translation: null
            });
          },

          // Return statements
          ReturnStatement(retPath) {
            const retContent = source.slice(retPath.node.start, retPath.node.end);
            const retHash = hashContent(retContent);

            templates[hash].body_templates.push({
              hash: retHash,
              type: 'ReturnStatement',
              babel_source: retContent,
              rust_translation: null
            });
          },

          // Variable declarations
          VariableDeclaration(varPath) {
            const varContent = source.slice(varPath.node.start, varPath.node.end);
            const varHash = hashContent(varContent);

            templates[hash].body_templates.push({
              hash: varHash,
              type: 'VariableDeclaration',
              babel_source: varContent,
              rust_translation: null
            });
          },

          // Function calls (like t.isIdentifier())
          CallExpression(callPath) {
            const callContent = source.slice(callPath.node.start, callPath.node.end);
            const callHash = hashContent(callContent);

            // Check if it's a Babel types call (t.isXxx)
            if (
              t.isMemberExpression(callPath.node.callee) &&
              t.isIdentifier(callPath.node.callee.object, { name: 't' })
            ) {
              templates[hash].body_templates.push({
                hash: callHash,
                type: 'BabelTypeCheck',
                method: callPath.node.callee.property.name,
                babel_source: callContent,
                rust_translation: null
              });
            }
          }
        });
      }
    },

    post(state) {
      // Output the templates
      const output = {
        version: '1.0',
        source_file: state.opts.filename || 'unknown',
        helper_imports: Object.fromEntries(helperFunctions),
        templates
      };

      console.log(JSON.stringify(output, null, 2));
    }
  };
};
