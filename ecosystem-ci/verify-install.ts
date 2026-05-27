import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';

import cliPkg from '../packages/cli/package.json' with { type: 'json' };

const require = createRequire(`${process.cwd()}/`);

const expectedVersion = cliPkg.version;

try {
  const pkgPath = require.resolve('vite-plus/package.json');
  const pkg = require(pkgPath) as {
    version: string;
    name: string;
    dependencies?: Record<string, string>;
  };
  if (pkg.version !== expectedVersion) {
    console.error(`x vite-plus: expected version ${expectedVersion}, got ${pkg.version}`);
    process.exit(1);
  }

  const projectPkg = JSON.parse(
    readFileSync(path.join(process.cwd(), 'package.json'), 'utf-8'),
  ) as {
    dependencies?: Record<string, string>;
    devDependencies?: Record<string, string>;
  };
  const vitePlusSpec =
    projectPkg.dependencies?.['vite-plus'] ?? projectPkg.devDependencies?.['vite-plus'];

  const isFileSpec = vitePlusSpec?.startsWith('file:') ?? false;
  const isPnpmFileInstall = `${path.sep}${pkgPath}`.includes(
    `${path.sep}.pnpm${path.sep}vite-plus@file+`,
  );
  if (!isFileSpec && !isPnpmFileInstall) {
    console.error(
      `x vite-plus: expected local file: install, got spec ${vitePlusSpec ?? '<missing>'}`,
    );
    console.error(`  resolved to ${pkgPath}`);
    process.exit(1);
  }

  const vitePlusRequire = createRequire(pkgPath);
  const oxlintPkgPath = vitePlusRequire.resolve('oxlint/package.json');
  const oxlintPkg = vitePlusRequire('oxlint/package.json') as { version: string };
  const expectedOxlint = pkg.dependencies?.oxlint?.replace(/^=/, '');
  if (expectedOxlint && oxlintPkg.version !== expectedOxlint) {
    console.error(`x oxlint: expected ${expectedOxlint}, got ${oxlintPkg.version}`);
    console.error(`  resolved to ${oxlintPkgPath}`);
    process.exit(1);
  }

  console.log(`ok vite-plus@${pkg.version} (${vitePlusSpec ?? 'unknown spec'})`);
  console.log(`ok oxlint@${oxlintPkg.version} from vite-plus dependency tree`);
} catch (error) {
  console.error('x vite-plus: not installed or incomplete');
  if (error instanceof Error) {
    console.error(error.message);
  }
  process.exit(1);
}
