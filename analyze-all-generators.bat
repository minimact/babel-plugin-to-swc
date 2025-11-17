@echo off
echo Analyzing all generator files...
echo.

for %%f in (example\babel-plugin-minimact\src\generators\*.cjs) do (
    echo === %%~nf ===
    target\debug\babel-to-swc.exe --analyze "%%f" 2>&1 | grep -oP "^\s+\d+\.\s+\K[A-Z][a-zA-Z]+(?= \{)" | sort | uniq -c | sort -rn
    echo.
)
