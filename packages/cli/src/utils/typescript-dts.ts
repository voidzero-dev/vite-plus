import fs from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';

import type { ResolvedConfig } from '@voidzero-dev/vite-plus-core/pack';
import semver from 'semver';

interface TypeScriptPackageJson {
  version?: string;
}

function resolveTypeScriptPackage(cwd: string): {
  packageJsonPath: string;
  version: string;
} | null {
  const projectRequire = createRequire(path.join(cwd, 'package.json'));
  let packageJsonPath: string;
  try {
    packageJsonPath = projectRequire.resolve('typescript/package.json');
  } catch {
    return null;
  }

  const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8')) as TypeScriptPackageJson;
  return packageJson.version ? { packageJsonPath, version: packageJson.version } : null;
}

function resolveTypeScript7Compiler(packageJsonPath: string): string {
  const platformPackage = `@typescript/typescript-${process.platform}-${process.arch}`;
  const typescriptRequire = createRequire(packageJsonPath);
  let platformPackageJsonPath: string;
  try {
    platformPackageJsonPath = typescriptRequire.resolve(`${platformPackage}/package.json`);
  } catch {
    throw new Error(
      `Unable to resolve the TypeScript 7 compiler package ${platformPackage}. ` +
        'Reinstall TypeScript without omitting optional dependencies.',
    );
  }

  const executable = path.join(
    path.dirname(platformPackageJsonPath),
    'lib',
    process.platform === 'win32' ? 'tsc.exe' : 'tsc',
  );
  if (!fs.existsSync(executable)) {
    throw new Error(`TypeScript 7 compiler executable not found: ${executable}`);
  }
  return executable;
}

/**
 * TypeScript 7's package root only exposes the new API, while the default DTS
 * pipeline consumes the legacy TypeScript compiler API. Route declaration
 * generation through the native compiler that TypeScript 7 installs for the
 * current platform instead.
 */
export function configureTypeScript7Dts(config: ResolvedConfig): void {
  if (!config.dts) {
    return;
  }

  const typescript = resolveTypeScriptPackage(config.cwd ?? process.cwd());
  if (!typescript || !semver.satisfies(typescript.version, '^7.0.0')) {
    return;
  }

  const { dts } = config;
  if (dts.oxc) {
    return;
  }

  const tsgo = dts.tsgo;
  if (typeof tsgo === 'object' && tsgo.path) {
    return;
  }
  if (tsgo === false) {
    throw new Error(
      'TypeScript 7 declaration generation requires `pack.dts.tsgo` or `pack.dts.oxc`. ' +
        'Remove `tsgo: false`, enable the Oxc declaration generator, or use TypeScript 6.',
    );
  }
  if (dts.vue || dts.tsMacro) {
    throw new Error(
      'TypeScript 7 declaration generation does not support the legacy `vue` or `tsMacro` ' +
        'pipeline. Use TypeScript 6 for this pack configuration.',
    );
  }

  dts.tsgo = {
    ...(typeof tsgo === 'object' ? tsgo : {}),
    path: resolveTypeScript7Compiler(typescript.packageJsonPath),
  };
}
