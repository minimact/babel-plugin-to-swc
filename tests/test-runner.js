/**
 * Test Runner - Compares Babel plugin output with Rust/SWC transpiler output
 */

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

const FIXTURES_DIR = path.join(__dirname, '../fixtures');
const REFERENCE_DIR = path.join(__dirname, 'reference-outputs');
const TEST_OUTPUT_DIR = path.join(__dirname, 'test-outputs');

// Ensure output directories exist
[REFERENCE_DIR, TEST_OUTPUT_DIR].forEach(dir => {
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true });
  }
});

/**
 * Run Babel plugin on a fixture and capture output
 */
function runBabelPlugin(fixturePath) {
  console.log(`\n[Babel] Processing ${path.basename(fixturePath)}...`);

  const babel = require('@babel/core');
  const babelPlugin = require('../example/simple-babel-plugin/index.js');

  const code = fs.readFileSync(fixturePath, 'utf-8');

  // Transform with Babel plugin
  const result = babel.transformSync(code, {
    plugins: [babelPlugin],
    filename: fixturePath,
    parserOpts: {
      sourceType: 'module',
      plugins: ['jsx', 'typescript']
    }
  });

  // Extract metadata (components detected by plugin)
  const components = result.metadata?.components || [];

  return {
    components,
    transformedCode: result.code
  };
}

/**
 * Run generated Rust/SWC transpiler on a fixture
 */
function runRustTranspiler(fixturePath) {
  console.log(`[Rust/SWC] Processing ${path.basename(fixturePath)}...`);

  // For now, this would call the Rust binary with the fixture
  // In the future, we'll have a generated SWC plugin
  try {
    const output = execSync(
      `target/debug/babel-to-swc.exe --analyze "${fixturePath}"`,
      { encoding: 'utf-8', cwd: path.join(__dirname, '..') }
    );

    // Parse the output to extract component data
    // This is a placeholder - we'll need to output JSON from Rust
    return {
      components: [],
      rawOutput: output
    };
  } catch (error) {
    console.error('[Rust/SWC] Error:', error.message);
    return {
      components: [],
      error: error.message
    };
  }
}

/**
 * Compare two outputs and report differences
 */
function compareOutputs(babelOutput, rustOutput, testName) {
  console.log(`\n[Compare] Analyzing ${testName}...`);

  const diffs = [];

  // Compare component count
  if (babelOutput.components.length !== rustOutput.components.length) {
    diffs.push({
      type: 'component-count',
      babel: babelOutput.components.length,
      rust: rustOutput.components.length
    });
  }

  // Compare each component
  babelOutput.components.forEach((babelComp, i) => {
    const rustComp = rustOutput.components[i];

    if (!rustComp) {
      diffs.push({
        type: 'missing-component',
        component: babelComp.name,
        message: `Babel detected component '${babelComp.name}', Rust did not`
      });
      return;
    }

    // Compare component name
    if (babelComp.name !== rustComp.name) {
      diffs.push({
        type: 'component-name',
        babel: babelComp.name,
        rust: rustComp.name
      });
    }

    // Compare hooks
    compareLists(babelComp.hooks, rustComp.hooks, 'hooks', diffs);

    // Compare props
    compareLists(babelComp.props, rustComp.props, 'props', diffs);

    // Compare JSX elements
    compareLists(babelComp.jsxElements, rustComp.jsxElements, 'jsxElements', diffs);
  });

  return diffs;
}

/**
 * Compare two lists (hooks, props, etc.)
 */
function compareLists(babelList = [], rustList = [], listName, diffs) {
  if (babelList.length !== rustList.length) {
    diffs.push({
      type: `${listName}-count`,
      babel: babelList.length,
      rust: rustList.length
    });
  }

  // Deep compare each item
  babelList.forEach((babelItem, i) => {
    const rustItem = rustList[i];

    if (!rustItem) {
      diffs.push({
        type: `missing-${listName}`,
        item: babelItem,
        message: `Babel detected ${listName} item, Rust did not`
      });
      return;
    }

    // Compare all properties
    Object.keys(babelItem).forEach(key => {
      if (JSON.stringify(babelItem[key]) !== JSON.stringify(rustItem[key])) {
        diffs.push({
          type: `${listName}-value`,
          property: key,
          babel: babelItem[key],
          rust: rustItem[key]
        });
      }
    });
  });
}

/**
 * Run all tests
 */
function runAllTests() {
  console.log('═══════════════════════════════════════════════');
  console.log('  Babel-to-SWC Test Suite');
  console.log('═══════════════════════════════════════════════\n');

  // Get all fixtures
  const fixtures = fs.readdirSync(FIXTURES_DIR)
    .filter(f => f.endsWith('.tsx') || f.endsWith('.jsx'))
    .map(f => path.join(FIXTURES_DIR, f));

  console.log(`Found ${fixtures.length} test fixtures\n`);

  const results = [];

  fixtures.forEach(fixturePath => {
    const testName = path.basename(fixturePath, path.extname(fixturePath));

    console.log(`\n${'='.repeat(50)}`);
    console.log(`TEST: ${testName}`);
    console.log('='.repeat(50));

    // Run Babel plugin
    const babelOutput = runBabelPlugin(fixturePath);

    // Save reference output
    const refPath = path.join(REFERENCE_DIR, `${testName}.json`);
    fs.writeFileSync(refPath, JSON.stringify(babelOutput, null, 2));
    console.log(`[Babel] Reference saved to ${path.relative(process.cwd(), refPath)}`);

    // Run Rust transpiler
    const rustOutput = runRustTranspiler(fixturePath);

    // Save test output
    const testPath = path.join(TEST_OUTPUT_DIR, `${testName}.json`);
    fs.writeFileSync(testPath, JSON.stringify(rustOutput, null, 2));
    console.log(`[Rust/SWC] Output saved to ${path.relative(process.cwd(), testPath)}`);

    // Compare outputs
    const diffs = compareOutputs(babelOutput, rustOutput, testName);

    results.push({
      testName,
      passed: diffs.length === 0,
      diffs
    });

    // Report results
    if (diffs.length === 0) {
      console.log(`\n✅ ${testName}: PASSED`);
    } else {
      console.log(`\n❌ ${testName}: FAILED (${diffs.length} differences)`);
      diffs.forEach(diff => {
        console.log(`   - ${diff.type}: ${JSON.stringify(diff, null, 2)}`);
      });
    }
  });

  // Summary
  console.log('\n\n' + '═'.repeat(50));
  console.log('  TEST SUMMARY');
  console.log('═'.repeat(50));

  const passed = results.filter(r => r.passed).length;
  const failed = results.filter(r => !r.passed).length;

  console.log(`\nTotal Tests: ${results.length}`);
  console.log(`✅ Passed: ${passed}`);
  console.log(`❌ Failed: ${failed}`);

  if (failed > 0) {
    console.log('\nFailed Tests:');
    results.filter(r => !r.passed).forEach(r => {
      console.log(`  - ${r.testName} (${r.diffs.length} diffs)`);
    });
  }

  // Exit with error code if any tests failed
  process.exit(failed > 0 ? 1 : 0);
}

// Run tests
runAllTests();
