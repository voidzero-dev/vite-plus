import fs from 'node:fs';
import path from 'node:path';

const nodeVersions = ['20.18.0', '22.18.0', '24.11.0'];

for (const version of nodeVersions) {
  const runtimeDir = path.join('home', 'js_runtime', 'node', version);
  fs.mkdirSync(path.join(runtimeDir, 'bin'), { recursive: true });
  fs.writeFileSync(path.join(runtimeDir, 'node.exe'), '');
}

fs.mkdirSync(path.join('home', 'package_manager', 'pnpm', '10.0.0', 'pnpm', 'bin'), {
  recursive: true,
});
fs.mkdirSync(path.join('home', 'package_manager', 'npm', '11.0.0', 'npm', 'bin'), {
  recursive: true,
});
fs.mkdirSync('fake-bin', { recursive: true });

if (process.platform === 'win32') {
  fs.writeFileSync(
    path.join('fake-bin', 'corepack.cmd'),
    '@echo off\r\nif not exist "%VP_HOME%" mkdir "%VP_HOME%"\r\ntype nul > "%VP_HOME%\\corepack-cleaned"\r\n',
  );
} else {
  fs.writeFileSync(
    path.join('fake-bin', 'corepack'),
    '#!/bin/sh\nmkdir -p "$VP_HOME"\ntouch "$VP_HOME/corepack-cleaned"\n',
    { mode: 0o755 },
  );
}
fs.writeFileSync(path.join('home', 'config.json'), '{"defaultNodeVersion":"24.11.0"}\n');
