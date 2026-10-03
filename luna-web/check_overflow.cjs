const fs = require('fs');
const html = fs.readFileSync('dist/index.html', 'utf8');

// Find all classes on elements
const matches = [...html.matchAll(/class="([^"]+)"/g)].map(m => m[1]);
console.log('Total classes in index.html:', matches.length);
