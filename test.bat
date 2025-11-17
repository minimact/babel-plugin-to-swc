@echo off
echo ═══════════════════════════════════════════════
echo   Building Rust Transpiler
echo ═══════════════════════════════════════════════
call build.bat
if errorlevel 1 (
    echo Build failed!
    exit /b 1
)

echo.
echo ═══════════════════════════════════════════════
echo   Running Test Suite
echo ═══════════════════════════════════════════════
echo.

node tests/test-runner.js %*
