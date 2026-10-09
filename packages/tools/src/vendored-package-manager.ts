import { existsSync, readFileSync, realpathSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

// This module runs before dependency installation in CI.
// Keep it independent of installed project dependencies, with only built-in imports.
//
// Upstream rolldown/vite pin pnpm v12 in their packageManager fields. pnpm v12's
// @pnpm/exe ships a native binary as the `pnpm` bin, which breaks pnpm v11's
// version self-switch (pnpm/pnpm#12811): the switch re-executes the downloaded
// bin through `node`, crashing on the ELF header. Nested pnpm calls inside the
// vendored repos (e.g. vite's `pnpm build-types-roll`) would self-switch to the
// upstream pin, so align the vendored packageManager fields with the root's to
// keep every nested call on the pnpm version that runs the build.

const VENDORED_DIRS = ['rolldown', 'vite'];

interface PackageJson {
  packageManager?: string;
}

export function alignVendoredPackageManagers(rootDir: string): void {
  const rootManifestPath = join(rootDir, 'package.json');
  const { packageManager: rootPackageManager } = JSON.parse(
    readFileSync(rootManifestPath, 'utf8'),
  ) as PackageJson;
  if (typeof rootPackageManager !== 'string' || rootPackageManager === '') {
    throw new Error(`Root ${rootManifestPath} must declare a packageManager pin`);
  }
  for (const dir of VENDORED_DIRS) {
    const file = join(rootDir, dir, 'package.json');
    if (!existsSync(file)) {
      continue;
    }
    const pkg: PackageJson = JSON.parse(readFileSync(file, 'utf8'));
    if (pkg.packageManager === undefined || pkg.packageManager === rootPackageManager) {
      continue;
    }
    pkg.packageManager = rootPackageManager;
    writeFileSync(file, `${JSON.stringify(pkg, null, 2)}\n`);
    console.log(`Aligned ${dir}/package.json packageManager with root pin ${rootPackageManager}`);
  }
}

if (
  process.argv[1] &&
  existsSync(process.argv[1]) &&
  fileURLToPath(import.meta.url) === realpathSync(process.argv[1])
) {
  alignVendoredPackageManagers(process.cwd());
}
