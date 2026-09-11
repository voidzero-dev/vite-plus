import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';

import { vitePlusTgzVersion } from './paths.ts';

const require = createRequire(`${process.cwd()}/`);

// patch-project.ts serves the packed tgz through the local registry, so a
// correctly installed local build always reports the tgz version (0.0.0 on
// CI, where the pack step pins it precisely so a local build is never
// mistaken for a published registry version).
const expectedVersion = vitePlusTgzVersion();

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

  // The migration must pin the local version as a plain registry spec
  // (resolved through the local registry), exactly like a real migration,
  // not a file:/link: escape hatch. pnpm workspace projects reference the
  // pinned version through the catalog instead of an inline version; the
  // installed `pkg.version` assertion above already proves which version the
  // catalog resolved to.
  if (vitePlusSpec !== expectedVersion && !vitePlusSpec?.startsWith('catalog:')) {
    console.error(
      `x vite-plus: expected exact registry spec ${expectedVersion} (or a catalog reference), got ${vitePlusSpec ?? '<missing>'}`,
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

  console.log(`ok vite-plus@${pkg.version} (${vitePlusSpec})`);
  console.log(`ok oxlint@${oxlintPkg.version} from vite-plus dependency tree`);
} catch (error) {
  console.error('x vite-plus: not installed or incomplete');
  if (error instanceof Error) {
    console.error(error.message);
  }
  process.exit(1);
}
