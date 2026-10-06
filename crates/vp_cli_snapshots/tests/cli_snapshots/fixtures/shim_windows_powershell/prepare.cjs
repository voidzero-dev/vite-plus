const { copyFileSync, mkdirSync, writeFileSync } = require('node:fs');
const { join } = require('node:path');

const manager = process.argv[2];
const version = manager === 'pnpm' ? '10.18.0' : '10.8.2';
const aliases = manager === 'pnpm' ? ['pnpm', 'pnpx'] : ['npm', 'npx'];
const bin = join(process.env.VP_HOME, 'package_manager', manager, version, manager, 'bin');
mkdirSync(bin, { recursive: true });
for (const alias of aliases) {
  copyFileSync('launcher.cmd', join(bin, `${alias}.cmd`));
  copyFileSync('launcher.ps1', join(bin, `${alias}.ps1`));
}
writeFileSync('package.json', JSON.stringify({ private: true, packageManager: `${manager}@${version}` }));
writeFileSync('.node-version', process.versions.node);
