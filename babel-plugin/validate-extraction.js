/**
 * Validate template extraction completeness
 *
 * Counts all AST node types in the helper modules to ensure we're capturing everything
 */

const parser = require('@babel/parser');
const traverse = require('@babel/traverse').default;
const fs = require('fs');
const path = require('path');

const srcDir = path.resolve(__dirname, '../example/babel-plugin-minimact/src');

const helperDirs = [
  'utils',
  'analyzers',
  'extractors',
  'generators',
  'types',
  'transpilers'
];

// Count all node types
const nodeCounts = {};
const nodeExamples = {};

// Process all files
helperDirs.forEach(dir => {
  const dirPath = path.join(srcDir, dir);
  if (!fs.existsSync(dirPath)) return;

  const files = fs.readdirSync(dirPath).filter(f => f.endsWith('.cjs') || f.endsWith('.js'));

  files.forEach(file => {
    const filePath = path.join(dirPath, file);
    try {
      const code = fs.readFileSync(filePath, 'utf-8');
      const ast = parser.parse(code, { sourceType: 'script' });

      traverse(ast, {
        enter(path) {
          const type = path.node.type;
          nodeCounts[type] = (nodeCounts[type] || 0) + 1;

          // Store first example of each type
          if (!nodeExamples[type]) {
            const start = path.node.start;
            const end = path.node.end;
            if (start != null && end != null) {
              const example = code.slice(start, end);
              if (example.length < 200) {
                nodeExamples[type] = example;
              } else {
                nodeExamples[type] = example.slice(0, 200) + '...';
              }
            }
          }
        }
      });
    } catch (err) {
      console.error(`Error in ${file}: ${err.message}`);
    }
  });
});

// Currently extracted patterns
const extractedTypes = [
  'FunctionDeclaration',
  'IfStatement',
  'ReturnStatement',
  'VariableDeclaration',
  'CallExpression',
  'TemplateLiteral',
  'LogicalExpression',
  'BinaryExpression',
  'ConditionalExpression',
  'ArrowFunctionExpression',
  'ForOfStatement',
  'ForStatement',
  'SwitchStatement',
  'WhileStatement',
  'ThrowStatement',
  'AssignmentExpression',
  'UnaryExpression',
  'ObjectExpression',
  'ArrayExpression',
  'NewExpression',
  'UpdateExpression',
  'BreakStatement',
  'ContinueStatement',
  'RegExpLiteral'
];

// Sort by count
const sorted = Object.entries(nodeCounts)
  .sort((a, b) => b[1] - a[1]);

console.log('=== AST Node Type Counts ===\n');
console.log('Currently Extracted:');
sorted
  .filter(([type]) => extractedTypes.includes(type))
  .forEach(([type, count]) => {
    console.log(`  ✅ ${type}: ${count}`);
  });

console.log('\nNOT Extracted (potential gaps):');
sorted
  .filter(([type]) => !extractedTypes.includes(type))
  .forEach(([type, count]) => {
    if (count > 10) { // Only show types with significant occurrences
      console.log(`  ❌ ${type}: ${count}`);
    }
  });

console.log('\n=== Examples of Missing Types ===\n');
sorted
  .filter(([type]) => !extractedTypes.includes(type) && nodeCounts[type] > 50)
  .slice(0, 10)
  .forEach(([type]) => {
    console.log(`${type}:`);
    console.log(`  ${nodeExamples[type]}`);
    console.log('');
  });
