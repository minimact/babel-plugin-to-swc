@echo off
echo ═══════════════════════════════════════════════
echo   Manual Test - Comparing Babel vs Rust Output
echo ═══════════════════════════════════════════════
echo.

set TEST_FILE=fixtures/Counter.tsx

echo [1/3] Running Babel plugin on %TEST_FILE%...
echo.
call node -e "const babel = require('@babel/core'); const plugin = require('./example/simple-babel-plugin/index.js'); const fs = require('fs'); const code = fs.readFileSync('%TEST_FILE%', 'utf-8'); const result = babel.transformSync(code, { plugins: [plugin], filename: '%TEST_FILE%', parserOpts: { sourceType: 'module', plugins: ['jsx', 'typescript'] } }); console.log('Babel Output:'); console.log(JSON.stringify(result.metadata, null, 2));"

echo.
echo.
echo [2/3] Running Rust transpiler in JSON mode...
echo.
target\debug\babel-to-swc.exe --json

echo.
echo.
echo [3/3] Comparison
echo.
echo NOTE: Full automated comparison will be implemented in test-runner.js
echo For now, manually compare the outputs above.
echo.
