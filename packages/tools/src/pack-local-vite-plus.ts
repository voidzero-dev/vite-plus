// Pack the checkout's publishable Vite+ packages (vite-plus and
// @voidzero-dev/vite-plus-core) so a local npm registry can serve them at the
// checkout version. Used by local-npm-registry.ts for snapshot tests, e2e,
// and standalone development.
//
// Uses node builtins only and erasable TypeScript syntax, so
// local-npm-registry.ts stays runnable with bare `node` from any directory.

import { execFile } from 'node:child_process';
import {
  copyFileSync,
  existsSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);

function getHostNativePackageSuffix(): string {
  if (process.arch !== 'arm64' && process.arch !== 'x64') {
    throw new Error(`Unsupported native package architecture: ${process.arch}`);
  }

  if (process.platform === 'darwin') {
    return `darwin-${process.arch}`;
  }
  if (process.platform === 'win32') {
    return `win32-${process.arch}-msvc`;
  }
  if (process.platform === 'linux') {
    const report = process.report?.getReport() as { header?: { glibcVersionRuntime?: string } };
    const libc = report.header?.glibcVersionRuntime ? 'gnu' : 'musl';
    return `linux-${process.arch}-${libc}`;
  }

  throw new Error(`Unsupported native package platform: ${process.platform}`);
}

async function packHostNativePackage(
  repoRoot: string,
  destination: string,
  suffix: string,
  bindingFile: string,
): Promise<void> {
  const cliPkg = JSON.parse(readFileSync(path.join(repoRoot, 'packages/cli/package.json'), 'utf8'));
  const packageDir = mkdtempSync(path.join(tmpdir(), `vite-plus-native-${suffix}-`));
  const binaryName = `vite-plus.${suffix}.node`;

  copyFileSync(bindingFile, path.join(packageDir, binaryName));
  writeFileSync(
    path.join(packageDir, 'package.json'),
    JSON.stringify(
      {
        name: `@voidzero-dev/vite-plus-${suffix}`,
        version: cliPkg.version,
        main: `./${binaryName}`,
      },
      null,
      2,
    ) + '\n',
  );

  try {
    await execFileAsync('npm', ['pack', '--pack-destination', destination], {
      cwd: packageDir,
      timeout: 120_000,
      shell: process.platform === 'win32',
    });
  } finally {
    rmSync(packageDir, { recursive: true, force: true });
  }
}

export async function packLocalVitePlusPackages(
  repoRoot: string,
  destination: string,
): Promise<void> {
  for (const sentinel of ['cli/dist/bin.js', 'core/dist']) {
    if (!existsSync(path.join(repoRoot, 'packages', sentinel))) {
      throw new Error(
        `Cannot pack local Vite+ packages: packages/${sentinel} is missing. Run \`pnpm build\` first.`,
      );
    }
  }
  // `vite-plus` packs `binding/*.node` (see its `files` field), and created
  // projects run `vp config` (their `prepare` script) from their OWN
  // installed vite-plus, so the host binding must exist for the installed
  // package to be functional.
  const bindingDir = path.join(repoRoot, 'packages', 'cli', 'binding');
  const hostNativeSuffix = getHostNativePackageSuffix();
  const hostBindingName = `vite-plus.${hostNativeSuffix}.node`;
  const hostBindingPath = path.join(bindingDir, hostBindingName);
  if (!existsSync(hostBindingPath)) {
    throw new Error(
      `Cannot pack local Vite+ packages: no ${hostBindingName} in ${bindingDir}. ` +
        'Run `pnpm bootstrap-cli` (or build the NAPI binding) first.',
    );
  }
  await packHostNativePackage(repoRoot, destination, hostNativeSuffix, hostBindingPath);
  await execFileAsync(
    'pnpm',
    [
      '--filter',
      'vite-plus',
      '--filter',
      '@voidzero-dev/vite-plus-core',
      'pack',
      '--pack-destination',
      destination,
    ],
    { cwd: repoRoot, timeout: 120_000, shell: process.platform === 'win32' },
  );
  const packed = readdirSync(destination).filter((entry) => entry.endsWith('.tgz'));
  if (packed.length !== 3) {
    throw new Error(
      `Expected 3 packed local Vite+ packages in ${destination}, found: ${packed.join(', ') || 'none'}`,
    );
  }
}
