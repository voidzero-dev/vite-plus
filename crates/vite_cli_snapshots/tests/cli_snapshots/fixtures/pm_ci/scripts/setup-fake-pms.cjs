const { chmodSync, mkdirSync, writeFileSync } = require('node:fs');
const { join } = require('node:path');

const vpHome = process.env.VP_HOME;
if (!vpHome) {
  throw new Error('VP_HOME is required');
}

const packageManagers = [
  ['pnpm', '11.0.0'],
  ['npm', '10.5.0'],
  ['yarn', '1.22.22'],
  ['yarn', '4.0.0'],
  ['bun', '1.2.0'],
];

for (const [name, version] of packageManagers) {
  const binDir = join(vpHome, 'package_manager', name, version, name, 'bin');
  mkdirSync(binDir, { recursive: true });

  const fakePm = join(binDir, 'fake-pm.cjs');
  writeFileSync(
    fakePm,
    `const args = process.argv.slice(2);
console.log(${JSON.stringify(name)} + (args.length ? ' ' + args.join(' ') : ''));
`,
  );

  const unixShim = join(binDir, name);
  writeFileSync(unixShim, "#!/usr/bin/env node\nrequire('./fake-pm.cjs');\n");
  chmodSync(unixShim, 0o755);

  writeFileSync(join(binDir, `${name}.cmd`), `@echo off\r\nnode "%~dp0fake-pm.cjs" %*\r\n`);
  writeFileSync(
    join(binDir, `${name}.ps1`),
    'node "$PSScriptRoot/fake-pm.cjs" @args\nexit $LASTEXITCODE\n',
  );
}
