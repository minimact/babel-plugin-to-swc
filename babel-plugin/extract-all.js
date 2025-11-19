/**
 * Extract templates from all helper modules in babel-plugin-minimact
 *
 * Usage: node extract-all.js [output-file]
 * Default output: templates.json
 */

const babel = require('@babel/core');
const fs = require('fs');
const path = require('path');

const srcDir = path.resolve(__dirname, '../example/babel-plugin-minimact/src');
const forceOverwrite = process.argv.includes('--force');
const outputFile = process.argv.find(arg => !arg.startsWith('-') && arg !== process.argv[0] && arg !== process.argv[1]) || 'templates.json';

// Directories containing helper functions (not visitor logic)
const helperDirs = [
  'utils',
  'analyzers',
  'extractors',
  'generators',
  'types',
  'transpilers'
];

// Collect all templates from all files
const allTemplates = {
  version: '1.0',
  generated_at: new Date().toISOString(),
  source_dir: srcDir,
  files: {},
  templates: {}
};

// Modified plugin that returns data instead of logging
function createExtractPlugin() {
  const crypto = require('crypto');

  function hashContent(content) {
    return crypto.createHash('sha256').update(content).digest('hex').slice(0, 8);
  }

  return function({ types: t }) {
    let templates = {};
    let helperFunctions = new Map();

    return {
      name: 'extract-templates',

      pre(state) {
        // Reset for each file
        templates = {};
        helperFunctions = new Map();
      },

      visitor: {
        Program: {
          exit(path, state) {
            // Call callback when done with program
            if (state.opts && state.opts.onComplete) {
              state.opts.onComplete(templates, Object.fromEntries(helperFunctions));
            }
          }
        },

        VariableDeclaration(path, state) {
          const declaration = path.node.declarations[0];
          if (!declaration) return;

          if (
            t.isCallExpression(declaration.init) &&
            t.isIdentifier(declaration.init.callee, { name: 'require' }) &&
            declaration.init.arguments[0] &&
            t.isStringLiteral(declaration.init.arguments[0])
          ) {
            const requirePath = declaration.init.arguments[0].value;

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

        FunctionDeclaration(path, state) {
          const funcName = path.node.id?.name;
          if (!funcName) return;

          const source = state.file.code;
          const funcContent = source.slice(path.node.start, path.node.end);
          const hash = hashContent(funcContent);

          // Extract parameter info with type hints
          const params = path.node.params.map(p => {
            let name = null;
            let defaultValue = null;
            let inferredType = 'unknown';

            if (t.isIdentifier(p)) {
              name = p.name;
            } else if (t.isAssignmentPattern(p) && t.isIdentifier(p.left)) {
              name = p.left.name;
              defaultValue = source.slice(p.right.start, p.right.end);
            }

            // Infer type from parameter name patterns
            if (name) {
              if (name === 'path' || name.endsWith('Path')) {
                inferredType = '&Path'; // Babel path
              } else if (name === 'node' || name.endsWith('Node')) {
                inferredType = '&Node';
              } else if (name === 'state') {
                inferredType = '&mut State';
              } else if (name.includes('Type') || name === 'tsType') {
                inferredType = '&TsType';
              } else if (name.includes('Expr') || name === 'expr' || name === 'expression') {
                inferredType = '&Expr';
              } else if (name.includes('Stmt') || name === 'stmt' || name === 'statement') {
                inferredType = '&Stmt';
              } else if (name === 'name' || name.endsWith('Name') || name === 'str' || name === 'text') {
                inferredType = '&str';
              } else if (name === 'options' || name === 'config' || name === 'opts') {
                inferredType = '&Options';
              }
            }

            return name ? { name, inferredType, defaultValue } : null;
          }).filter(Boolean);

          // Infer return type from function name patterns
          let inferredReturnType = 'unknown';
          if (funcName.startsWith('is') || funcName.startsWith('has') || funcName.startsWith('should')) {
            inferredReturnType = 'bool';
          } else if (funcName.startsWith('get') || funcName.startsWith('extract')) {
            inferredReturnType = 'Option<T>';
          } else if (funcName.startsWith('generate') || funcName.startsWith('build') || funcName.startsWith('create')) {
            inferredReturnType = 'String';
          } else if (funcName.startsWith('analyze') || funcName.startsWith('process')) {
            inferredReturnType = 'AnalysisResult';
          } else if (funcName.includes('Type') || funcName.endsWith('Type')) {
            inferredReturnType = '&\'static str';
          }

          templates[hash] = {
            type: 'FunctionDeclaration',
            name: funcName,
            params: params,
            inferredReturnType: inferredReturnType,
            babel_source: funcContent,
            body_templates: [],
            rust_translation: null
          };

          // Store parent context for body templates
          const parentContext = {
            functionName: funcName,
            functionParams: params.map(p => p.name)
          };

          path.traverse({
            IfStatement(ifPath) {
              const ifContent = source.slice(ifPath.node.start, ifPath.node.end);
              const ifHash = hashContent(ifContent);

              templates[hash].body_templates.push({
                hash: ifHash,
                type: 'IfStatement',
                condition: source.slice(ifPath.node.test.start, ifPath.node.test.end),
                babel_source: ifContent,
                parentContext,
                rust_translation: null
              });
            },

            ReturnStatement(retPath) {
              const retContent = source.slice(retPath.node.start, retPath.node.end);
              const retHash = hashContent(retContent);

              templates[hash].body_templates.push({
                hash: retHash,
                type: 'ReturnStatement',
                babel_source: retContent,
                parentContext,
                rust_translation: null
              });
            },

            VariableDeclaration(varPath) {
              const varContent = source.slice(varPath.node.start, varPath.node.end);
              const varHash = hashContent(varContent);

              templates[hash].body_templates.push({
                hash: varHash,
                type: 'VariableDeclaration',
                babel_source: varContent,
                parentContext,
                rust_translation: null
              });
            },

            CallExpression(callPath) {
              const callContent = source.slice(callPath.node.start, callPath.node.end);
              const callHash = hashContent(callContent);

              if (
                t.isMemberExpression(callPath.node.callee) &&
                t.isIdentifier(callPath.node.callee.object, { name: 't' })
              ) {
                templates[hash].body_templates.push({
                  hash: callHash,
                  type: 'BabelTypeCheck',
                  method: callPath.node.callee.property.name,
                  babel_source: callContent,
                  parentContext,
                  rust_translation: null
                });
              }
            },

            // Template literals: `List<${type}>`
            TemplateLiteral(tplPath) {
              const tplContent = source.slice(tplPath.node.start, tplPath.node.end);
              const tplHash = hashContent(tplContent);

              templates[hash].body_templates.push({
                hash: tplHash,
                type: 'TemplateLiteral',
                babel_source: tplContent,
                parentContext,
                rust_translation: null
              });
            },

            // Logical expressions: a && b, a || b
            LogicalExpression(logPath) {
              const logContent = source.slice(logPath.node.start, logPath.node.end);
              const logHash = hashContent(logContent);

              templates[hash].body_templates.push({
                hash: logHash,
                type: 'LogicalExpression',
                operator: logPath.node.operator,
                babel_source: logContent,
                parentContext,
                rust_translation: null
              });
            },

            // Binary expressions: a === b, a + b
            BinaryExpression(binPath) {
              const binContent = source.slice(binPath.node.start, binPath.node.end);
              const binHash = hashContent(binContent);

              templates[hash].body_templates.push({
                hash: binHash,
                type: 'BinaryExpression',
                operator: binPath.node.operator,
                babel_source: binContent,
                parentContext,
                rust_translation: null
              });
            },

            // Ternary: condition ? a : b
            ConditionalExpression(condPath) {
              const condContent = source.slice(condPath.node.start, condPath.node.end);
              const condHash = hashContent(condContent);

              templates[hash].body_templates.push({
                hash: condHash,
                type: 'ConditionalExpression',
                babel_source: condContent,
                parentContext,
                rust_translation: null
              });
            },

            // Arrow functions: const fn = () => {}
            ArrowFunctionExpression(arrowPath) {
              const arrowContent = source.slice(arrowPath.node.start, arrowPath.node.end);
              const arrowHash = hashContent(arrowContent);

              templates[hash].body_templates.push({
                hash: arrowHash,
                type: 'ArrowFunctionExpression',
                babel_source: arrowContent,
                parentContext,
                rust_translation: null
              });
            },

            // For...of loops
            ForOfStatement(forOfPath) {
              const forOfContent = source.slice(forOfPath.node.start, forOfPath.node.end);
              const forOfHash = hashContent(forOfContent);

              templates[hash].body_templates.push({
                hash: forOfHash,
                type: 'ForOfStatement',
                babel_source: forOfContent,
                parentContext,
                rust_translation: null
              });
            },

            // For loops
            ForStatement(forPath) {
              const forContent = source.slice(forPath.node.start, forPath.node.end);
              const forHash = hashContent(forContent);

              templates[hash].body_templates.push({
                hash: forHash,
                type: 'ForStatement',
                babel_source: forContent,
                parentContext,
                rust_translation: null
              });
            },

            // Switch statements
            SwitchStatement(switchPath) {
              const switchContent = source.slice(switchPath.node.start, switchPath.node.end);
              const switchHash = hashContent(switchContent);

              templates[hash].body_templates.push({
                hash: switchHash,
                type: 'SwitchStatement',
                babel_source: switchContent,
                parentContext,
                rust_translation: null
              });
            },

            // While loops
            WhileStatement(whilePath) {
              const whileContent = source.slice(whilePath.node.start, whilePath.node.end);
              const whileHash = hashContent(whileContent);

              templates[hash].body_templates.push({
                hash: whileHash,
                type: 'WhileStatement',
                babel_source: whileContent,
                parentContext,
                rust_translation: null
              });
            },

            // Throw statements
            ThrowStatement(throwPath) {
              const throwContent = source.slice(throwPath.node.start, throwPath.node.end);
              const throwHash = hashContent(throwContent);

              templates[hash].body_templates.push({
                hash: throwHash,
                type: 'ThrowStatement',
                babel_source: throwContent,
                parentContext,
                rust_translation: null
              });
            },

            // Assignment expressions: a = b
            AssignmentExpression(assignPath) {
              const assignContent = source.slice(assignPath.node.start, assignPath.node.end);
              const assignHash = hashContent(assignContent);

              templates[hash].body_templates.push({
                hash: assignHash,
                type: 'AssignmentExpression',
                operator: assignPath.node.operator,
                babel_source: assignContent,
                parentContext,
                rust_translation: null
              });
            },

            // Unary expressions: !value, -num, typeof x
            UnaryExpression(unaryPath) {
              const unaryContent = source.slice(unaryPath.node.start, unaryPath.node.end);
              const unaryHash = hashContent(unaryContent);

              templates[hash].body_templates.push({
                hash: unaryHash,
                type: 'UnaryExpression',
                operator: unaryPath.node.operator,
                babel_source: unaryContent,
                parentContext,
                rust_translation: null
              });
            },

            // Object expressions: { key: value }
            ObjectExpression(objPath) {
              const objContent = source.slice(objPath.node.start, objPath.node.end);
              const objHash = hashContent(objContent);

              templates[hash].body_templates.push({
                hash: objHash,
                type: 'ObjectExpression',
                babel_source: objContent,
                parentContext,
                rust_translation: null
              });
            },

            // Array expressions: [1, 2, 3]
            ArrayExpression(arrPath) {
              const arrContent = source.slice(arrPath.node.start, arrPath.node.end);
              const arrHash = hashContent(arrContent);

              templates[hash].body_templates.push({
                hash: arrHash,
                type: 'ArrayExpression',
                babel_source: arrContent,
                parentContext,
                rust_translation: null
              });
            },

            // New expressions: new Error(), new Map()
            NewExpression(newPath) {
              const newContent = source.slice(newPath.node.start, newPath.node.end);
              const newHash = hashContent(newContent);

              templates[hash].body_templates.push({
                hash: newHash,
                type: 'NewExpression',
                babel_source: newContent,
                parentContext,
                rust_translation: null
              });
            },

            // Update expressions: i++, --count
            UpdateExpression(updatePath) {
              const updateContent = source.slice(updatePath.node.start, updatePath.node.end);
              const updateHash = hashContent(updateContent);

              templates[hash].body_templates.push({
                hash: updateHash,
                type: 'UpdateExpression',
                operator: updatePath.node.operator,
                prefix: updatePath.node.prefix,
                babel_source: updateContent,
                parentContext,
                rust_translation: null
              });
            },

            // Break statements
            BreakStatement(breakPath) {
              const breakContent = source.slice(breakPath.node.start, breakPath.node.end);
              const breakHash = hashContent(breakContent);

              templates[hash].body_templates.push({
                hash: breakHash,
                type: 'BreakStatement',
                babel_source: breakContent,
                parentContext,
                rust_translation: null
              });
            },

            // Continue statements
            ContinueStatement(continuePath) {
              const continueContent = source.slice(continuePath.node.start, continuePath.node.end);
              const continueHash = hashContent(continueContent);

              templates[hash].body_templates.push({
                hash: continueHash,
                type: 'ContinueStatement',
                babel_source: continueContent,
                parentContext,
                rust_translation: null
              });
            },

            // RegExp literals: /pattern/g
            RegExpLiteral(regexPath) {
              const regexContent = source.slice(regexPath.node.start, regexPath.node.end);
              const regexHash = hashContent(regexContent);

              templates[hash].body_templates.push({
                hash: regexHash,
                type: 'RegExpLiteral',
                pattern: regexPath.node.pattern,
                flags: regexPath.node.flags,
                babel_source: regexContent,
                parentContext,
                rust_translation: null
              });
            },

            // Spread elements: ...arr
            SpreadElement(spreadPath) {
              const spreadContent = source.slice(spreadPath.node.start, spreadPath.node.end);
              const spreadHash = hashContent(spreadContent);

              templates[hash].body_templates.push({
                hash: spreadHash,
                type: 'SpreadElement',
                babel_source: spreadContent,
                parentContext,
                rust_translation: null
              });
            }
          });
        }
      },

    };
  };
}

// Recursively get all .cjs and .js files from a directory
function getAllFiles(dirPath, arrayOfFiles = [], baseDir = '') {
  const files = fs.readdirSync(dirPath);

  files.forEach(file => {
    const fullPath = path.join(dirPath, file);
    const relativePath = baseDir ? `${baseDir}/${file}` : file;

    if (fs.statSync(fullPath).isDirectory()) {
      // Recurse into subdirectories
      getAllFiles(fullPath, arrayOfFiles, relativePath);
    } else if (file.endsWith('.cjs') || file.endsWith('.js')) {
      arrayOfFiles.push({ fullPath, relativePath });
    }
  });

  return arrayOfFiles;
}

// Process all files
console.log('Extracting templates from babel-plugin-minimact...\n');

helperDirs.forEach(dir => {
  const dirPath = path.join(srcDir, dir);

  if (!fs.existsSync(dirPath)) {
    console.log(`  Skipping ${dir}/ (not found)`);
    return;
  }

  const files = getAllFiles(dirPath);

  files.forEach(({ fullPath, relativePath: fileRelPath }) => {
    const filePath = fullPath;
    const relativePath = `${dir}/${fileRelPath}`;

    try {
      const code = fs.readFileSync(filePath, 'utf-8');

      // Store metadata externally since transformSync doesn't preserve it well
      let extractedTemplates = {};
      let extractedImports = {};

      const result = babel.transformSync(code, {
        filename: filePath,
        plugins: [[createExtractPlugin(), {
          onComplete: (templates, imports) => {
            extractedTemplates = templates;
            extractedImports = imports;
          }
        }]]
      });

      const templates = extractedTemplates;
      const helperImports = extractedImports;
      const templateCount = Object.keys(templates).length;

      if (templateCount > 0) {
        allTemplates.files[relativePath] = {
          helper_imports: helperImports,
          template_hashes: Object.keys(templates)
        };

        // Merge templates into global collection
        Object.entries(templates).forEach(([hash, template]) => {
          if (allTemplates.templates[hash]) {
            // Same hash means same content - just add the file reference
            allTemplates.templates[hash].found_in.push(relativePath);
          } else {
            allTemplates.templates[hash] = {
              ...template,
              found_in: [relativePath]
            };
          }
        });

        console.log(`  ✓ ${relativePath} (${templateCount} functions)`);
      } else {
        console.log(`  - ${relativePath} (no functions)`);
      }
    } catch (err) {
      console.log(`  ✗ ${relativePath} (error: ${err.message})`);
    }
  });
});

// Summary
const totalFunctions = Object.keys(allTemplates.templates).length;
const totalFiles = Object.keys(allTemplates.files).length;

console.log(`\n========================================`);
console.log(`Total: ${totalFunctions} functions from ${totalFiles} files`);
console.log(`========================================\n`);

// Create output directory for individual template files
const templatesDir = path.resolve(__dirname, 'templates');
if (!fs.existsSync(templatesDir)) {
  fs.mkdirSync(templatesDir, { recursive: true });
}

// Write each template to its own file (only if new)
let newCount = 0;
let skippedCount = 0;
const generatedFiles = new Set();

Object.entries(allTemplates.templates).forEach(([hash, template]) => {
  const fileName = `${template.name}-${hash}.json`;
  const templatePath = path.join(templatesDir, fileName);
  generatedFiles.add(fileName);

  // Check if file already exists
  if (fs.existsSync(templatePath) && !forceOverwrite) {
    // File exists - don't overwrite (preserve rust_translation)
    skippedCount++;
  } else {
    // New file or force overwrite - create it
    fs.writeFileSync(templatePath, JSON.stringify(template, null, 2));
    newCount++;
  }
});

// Move stale files to backup folder
const existingFiles = fs.readdirSync(templatesDir).filter(f => f.endsWith('.json'));
const staleFiles = existingFiles.filter(f => !generatedFiles.has(f));

if (staleFiles.length > 0) {
  const dateStr = new Date().toISOString().slice(0, 10);
  const backupDir = path.join(templatesDir, `backup-${dateStr}`);
  if (!fs.existsSync(backupDir)) {
    fs.mkdirSync(backupDir, { recursive: true });
  }

  staleFiles.forEach(f => {
    fs.renameSync(path.join(templatesDir, f), path.join(backupDir, f));
  });
  console.log(`  ${staleFiles.length} stale files moved to ${backupDir}`);
}

console.log(`Templates written to: ${templatesDir}/`);
console.log(`  ${newCount} new files created`);
console.log(`  ${skippedCount} existing files preserved (not overwritten)`);

// Also write an index file with just the file mappings
const indexPath = path.resolve(__dirname, outputFile);
const index = {
  version: allTemplates.version,
  generated_at: allTemplates.generated_at,
  source_dir: allTemplates.source_dir,
  files: allTemplates.files,
  template_count: Object.keys(allTemplates.templates).length
};
fs.writeFileSync(indexPath, JSON.stringify(index, null, 2));
console.log(`Index written to: ${indexPath}`);
