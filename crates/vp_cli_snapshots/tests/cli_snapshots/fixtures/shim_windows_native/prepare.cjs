const { copyFileSync, mkdirSync, writeFileSync } = require('node:fs');
const { join } = require('node:path');

const manager = process.argv[2];
const version = manager === 'bun' ? '1.4.2' : '12.8.1';
const aliases = manager === 'bun' ? ['bun', 'bunx'] : ['pnpm', 'pnpx'];
const bin = join(process.env.VP_HOME, 'package_manager', manager, version, manager, 'bin');
mkdirSync(bin, { recursive: true });
// Node stands in for the cached native runtime so argument/stdin checks stay
// offline. The old script wrappers deliberately fail if they are executed.
copyFileSync(process.execPath, join(bin, `${manager}.native.exe`));
for (const name of aliases) {
  writeFileSync(join(bin, name), 'fixture shim');
  writeFileSync(join(bin, `${name}.cmd`), '@echo unexpected batch launcher\r\n@exit /b 9\r\n');
  writeFileSync(join(bin, `${name}.ps1`), "Write-Output 'unexpected PowerShell launcher'\nexit 9\n");
}
writeFileSync('package.json', JSON.stringify({ private: true, packageManager: `${manager}@${version}` }));
writeFileSync('.node-version', process.versions.node);
