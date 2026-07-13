const fs = require('fs');
fs.mkdirSync('node_modules/.bin', { recursive: true });
fs.writeFileSync(
  'node_modules/.bin/vite',
  '#!/usr/bin/env node\nconst args = process.argv.slice(2);\nconsole.log(args.length ? "vite " + args.join(" ") : "vite");\n',
  { mode: 0o755 },
);
fs.writeFileSync('node_modules/.bin/vite.cmd', '@node "%~dp0\\vite" %*\n');
