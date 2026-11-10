const { copyFileSync, mkdirSync, writeFileSync } = require('node:fs');
const { join } = require('node:path');

const manager = process.argv[2];
const version = { pnpm: '10.18.0', npm: '10.8.2', yarn: '1.22.22' }[manager];
const aliases = { pnpm: ['pnpm', 'pnpx'], npm: ['npm', 'npx'], yarn: ['yarn', 'yarnpkg'] }[manager];
const scriptTool = process.argv[3] === 'alias' ? aliases[1] : manager;
const bin = join(process.env.VP_HOME, 'package_manager', manager, version, manager, 'bin');
mkdirSync(bin, { recursive: true });
for (const alias of aliases) {
  writeFileSync(join(bin, alias), 'fixture shim');
  copyFileSync('launcher.cmd', join(bin, `${alias}.cmd`));
  copyFileSync('launcher.ps1', join(bin, `${alias}.ps1`));
}
writeFileSync('package.json', JSON.stringify({
  private: true,
  packageManager: `${manager}@${version}`,
  scripts: { probe: `${scriptTool} --flag "value with spaces" 'path\\file'` },
}));
writeFileSync('.node-version', process.versions.node);
