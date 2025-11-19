const babel = require('@babel/core');
const fs = require('fs');
const path = require('path');

const inputFile = process.argv[2] || '../example/babel-plugin-minimact/src/utils/helpers.cjs';
const code = fs.readFileSync(path.resolve(__dirname, inputFile), 'utf-8');

babel.transformSync(code, {
  filename: inputFile,
  plugins: [require('./extract-templates.js')]
});
