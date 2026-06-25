const fs = require('fs');
const path = require('path');

const scopeDir = path.join(process.env.VP_HOME, 'packages', '@scope');
const legacyPackageDir = path.join(scopeDir, 'long-time-install-package');
const identifiedPackageDir = path.join(
  scopeDir,
  'long-time-install-package#123e4567-e89b-42d3-a456-426614174000',
);

fs.mkdirSync(legacyPackageDir, { recursive: true });
fs.writeFileSync(path.join(legacyPackageDir, 'stale'), '');
fs.mkdirSync(identifiedPackageDir, { recursive: true });
fs.writeFileSync(path.join(identifiedPackageDir, 'stale'), '');

console.log('stale packages created');
