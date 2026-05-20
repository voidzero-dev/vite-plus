import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';

const require = createRequire(`${process.cwd()}/`);

// The ecosystem-ci pack step pins packages/cli to 0.0.0 before `pnpm pack`, so
// a correctly installed local build always reports 0.0.0 — never the published
// registry version (which `patch-project.ts` likewise references as a fixed
// `vite-plus-0.0.0.tgz`).
const expectedVersion = '0.0.0';

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
  const isPnpmFileInstall = pkgPath.includes(`${path.sep}.pnpm${path.sep}vite-plus@file+`);
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
  const expectedOxlint = pkg.dependencies?.oxlint?.replace(/^[=^~]/, '');
  if (!expectedOxlint) {
    console.error('x vite-plus: package.json missing oxlint dependency');
    process.exit(1);
  }
  if (oxlintPkg.version !== expectedOxlint) {
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
