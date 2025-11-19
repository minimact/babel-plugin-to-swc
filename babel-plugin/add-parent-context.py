#!/usr/bin/env python3
"""
Add parentContext to all body_templates in extract-all.js
"""

import re

with open('extract-all.js', 'r', encoding='utf-8') as f:
    content = f.read()

# Pattern to find body_templates.push blocks that don't have parentContext
# Look for lines with babel_source followed by rust_translation without parentContext between
pattern = r'(babel_source: \w+,\s*\n)(\s*)(rust_translation: null)'

# Replace with parentContext added
replacement = r'\1\2parentContext,\n\2\3'

new_content = re.sub(pattern, replacement, content)

with open('extract-all.js', 'w', encoding='utf-8') as f:
    f.write(new_content)

print('Added parentContext to all body_templates')
