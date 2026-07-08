// Deletes lines containing any of the given substrings, standing in for the
// legacy `sed -e '/pattern/d'` cleanup of the generated vite.config.ts (the
// migrated lint config carries rule options vp check cannot fix in place).
// Usage: node delete-lines.cjs <file> <substring>...
const fs = require('node:fs');
const [file, ...patterns] = process.argv.slice(2);
const lines = fs.readFileSync(file, 'utf8').split('\n');
const kept = lines.filter((line) => !patterns.some((p) => line.includes(p)));
fs.writeFileSync(file, kept.join('\n'));
