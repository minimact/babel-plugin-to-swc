@echo off
echo ═══════════════════════════════════════════════
echo   Comparing Babel vs Generated SWC Plugin
echo ═══════════════════════════════════════════════
echo.

set TEST_FILE=../fixtures/Counter.tsx

echo [1/2] Running Babel plugin on Counter.tsx...
echo.
cd ..
node -e "const babel = require('@babel/core'); const plugin = require('./example/simple-babel-plugin/index.js'); const fs = require('fs'); const code = fs.readFileSync('fixtures/Counter.tsx', 'utf-8'); const result = babel.transformSync(code, { plugins: [plugin], filename: 'Counter.tsx', parserOpts: { sourceType: 'module', plugins: ['jsx', 'typescript'] } }); console.log('=== BABEL OUTPUT ==='); console.log(JSON.stringify(result.metadata, null, 2));" 2>&1
echo.
echo.

echo [2/2] Running Generated SWC Plugin on Counter.tsx...
echo.
cd generated-plugin
target\debug\extract-components.exe ../fixtures/Counter.tsx 2>nul
echo.
echo.

echo ═══════════════════════════════════════════════
echo   Comparison Complete
echo ═══════════════════════════════════════════════
