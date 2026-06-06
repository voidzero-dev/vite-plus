import { spawn, spawnSync } from 'node:child_process';
import { existsSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const marker = join(process.cwd(), 'postinstall-started');
const packageJsonPath = join(process.cwd(), 'slow-install-backup-pkg', 'package.json');
const tarball = join(process.cwd(), 'slow-install-backup-pkg-1.0.1.tgz');
const packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf8'));
packageJson.version = '1.0.1';
writeFileSync(packageJsonPath, `${JSON.stringify(packageJson, null, 2)}\n`);
rmSync(marker, { force: true });
rmSync(tarball, { force: true });

const pack = spawnSync('npm', ['pack', './slow-install-backup-pkg', '--pack-destination', '.'], {
  stdio: 'ignore',
});
if (pack.status !== 0) {
  console.log('npm pack failed');
  process.exit(pack.status ?? 1);
}

const child = spawn('vp', ['install', '-g', './slow-install-backup-pkg-1.0.1.tgz'], {
  detached: true,
  env: {
    ...process.env,
    SLOW_INSTALL_MARKER: marker,
  },
  stdio: 'ignore',
});

for (let i = 0; i < 50; i++) {
  if (existsSync(marker)) {
    break;
  }
  await new Promise((resolve) => setTimeout(resolve, 100));
}

if (!existsSync(marker)) {
  console.log('postinstall did not start');
  process.exit(1);
}

process.kill(-child.pid, 'SIGKILL');

await new Promise((resolve) => {
  child.on('exit', resolve);
});

console.log('killed install');
