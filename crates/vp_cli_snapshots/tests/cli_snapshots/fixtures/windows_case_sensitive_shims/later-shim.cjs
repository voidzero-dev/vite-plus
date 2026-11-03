const fs = require('node:fs');

fs.mkdirSync('node_modules/.bin', { recursive: true });
fs.writeFileSync('node_modules/.bin/astro.CMD', '@echo wrong workspace shim %*\r\n');
