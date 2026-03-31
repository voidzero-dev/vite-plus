import { execSync, spawnSync } from 'node:child_process';
import { existsSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { parseArgs } from 'node:util';

import upstreamVersions from '../.upstream-versions.json' with { type: 'json' };

interface PnpmWorkspace {
  packages?: string[];
  catalog?: Record<string, string>;
  catalogMode?: string;
  minimumReleaseAge?: number;
  minimumReleaseAgeExclude?: string[];
  patchedDependencies?: Record<string, string>;
  peerDependencyRules?: {
    allowedVersions?: Record<string, string>;
  };
  packageExtensions?: Record<string, unknown>;
  overrides?: Record<string, string>;
  ignoreScripts?: boolean;
  [key: string]: unknown;
}

interface PackageJson {
  name?: string;
  version?: string;
  exports?: Record<string, unknown>;
  [key: string]: unknown;
}

type ExportValue = string | { [condition: string]: string | ExportValue } | null;

const ROLLDOWN_DIR = 'rolldown';
const VITE_DIR = 'vite';
const CORE_PACKAGE_PATH = 'packages/core';

function log(message: string) {
  console.log(`[sync-rolldown] ${message}`);
}

function error(message: string): never {
  console.error(`[sync-rolldown] ERROR: ${message}`);
  process.exit(1);
}

const getMajor = (range: string): number | null => {
  const match = range.match(/(\d+)\./);
  return match ? parseInt(match[1], 10) : null;
};

function execCommand(command: string, cwd?: string): string {
  try {
    return execSync(command, {
      cwd,
      encoding: 'utf-8',
      stdio: 'pipe',
    }).trim();
  } catch (error) {
    throw new Error(
      `Failed to execute: ${command}\n${error instanceof Error ? error.message : String(error)}`,
      { cause: error },
    );
  }
}

function cloneOrResetRepo(repoUrl: string, dir: string, branch: string = 'main', hash?: string) {
  log(`Processing ${dir}...`);

  if (existsSync(dir)) {
    log(`${dir} exists, checking git status...`);
    try {
      // Check if it's a valid git repo
      const result = spawnSync('git', ['rev-parse', '--git-dir'], {
        cwd: dir,
        encoding: 'utf-8',
      });

      if (result.status !== 0) {
        log(`${dir} is not a valid git repo, removing and re-cloning...`);
        rmSync(dir, { recursive: true, force: true });
        cloneRepo(repoUrl, dir, branch, hash);
        return;
      }

      // Check remote URL
      const remoteUrl = execCommand('git remote get-url origin', dir);
      if (remoteUrl !== repoUrl) {
        log(`${dir} has wrong remote (${remoteUrl} vs ${repoUrl}), removing and re-cloning...`);
        rmSync(dir, { recursive: true, force: true });
        cloneRepo(repoUrl, dir, branch, hash);
        return;
      }

      // Fetch latest commits and tags
      execCommand('git fetch origin --tags', dir);

      if (hash) {
        // Reset to specific hash
        log(`Resetting ${dir} to pinned hash ${hash.substring(0, 8)}...`);
        execCommand(`git checkout ${branch}`, dir);
        execCommand(`git reset --hard ${hash}`, dir);
        log(`${dir} reset to ${hash.substring(0, 8)}`);
      } else {
        // Reset to latest - check if branch is a tag or a branch
        log(`Resetting ${dir} to latest ${branch}...`);
        const isTag =
          spawnSync('git', ['tag', '-l', branch], {
            cwd: dir,
            encoding: 'utf-8',
          }).stdout.trim() === branch;

        if (isTag) {
          // For tags, just checkout the tag directly
          execCommand(`git checkout ${branch}`, dir);
          log(`${dir} reset to tag ${branch}`);
        } else {
          // For branches, reset to origin/branch
          execCommand(`git checkout ${branch}`, dir);
          execCommand(`git reset --hard origin/${branch}`, dir);
          log(`${dir} reset to latest ${branch}`);
        }
      }
    } catch (error) {
      log(
        `Failed to reset ${dir} (${error instanceof Error ? error.message : String(error)}), removing and re-cloning...`,
      );
      rmSync(dir, { recursive: true, force: true });
      cloneRepo(repoUrl, dir, branch, hash);
    }
  } else {
    cloneRepo(repoUrl, dir, branch, hash);
  }
}

function cloneRepo(repoUrl: string, dir: string, branch: string, hash?: string) {
  log(`Cloning ${repoUrl} (${branch}) into ${dir}...`);
  execCommand(`git clone --branch ${branch} ${repoUrl} ${dir}`);
  if (hash) {
    log(`Checking out pinned hash ${hash.substring(0, 8)}...`);
    execCommand(`git reset --hard ${hash}`, dir);
    log(`${dir} cloned and reset to ${hash.substring(0, 8)}`);
  } else {
    log(`${dir} cloned successfully`);
  }
}

function transformRolldownExport(exportPath: string, exportValue: unknown): [string, ExportValue] {
  // Skip package.json
  if (exportPath === './package.json') {
    return ['', null];
  }

  // Transform export path: . -> ./rolldown, ./foo -> ./rolldown/foo
  const newExportPath = exportPath === '.' ? './rolldown' : `./rolldown${exportPath.slice(1)}`;

  // Transform export value
  const transformValue = (value: unknown): ExportValue => {
    if (typeof value === 'string') {
      // Skip 'dev' condition paths that point to src
      if (value.startsWith('./src/')) {
        return null;
      }
      // Transform dist paths
      return value.replace(/^\.\/dist\//, './dist/rolldown/');
    }

    if (value && typeof value === 'object') {
      const result: Record<string, unknown> = {};
      for (const [key, val] of Object.entries(value)) {
        // Skip 'dev' condition
        if (key === 'dev') {
          continue;
        }

        const transformed = transformValue(val);
        if (transformed !== null) {
          result[key] = transformed;
        }
      }
      return Object.keys(result).length > 0 ? (result as ExportValue) : null;
    }

    return value as ExportValue;
  };

  const newValue = transformValue(exportValue);

  // Handle string values or add types if missing
  if (typeof newValue === 'string') {
    // Convert string to object with default and types
    if (newValue.endsWith('.mjs')) {
      return [
        newExportPath,
        {
          default: newValue,
          types: newValue.replace(/\.mjs$/, '.d.mts'),
        },
      ];
    } else if (newValue.endsWith('.js')) {
      return [
        newExportPath,
        {
          default: newValue,
          types: newValue.replace(/\.js$/, '.d.ts'),
        },
      ];
    }
    return [newExportPath, newValue];
  }

  if (newValue && typeof newValue === 'object') {
    const importPath = ('import' in newValue ? newValue.import : newValue.default) as
      | string
      | undefined;
    if (importPath && !('types' in newValue)) {
      if (importPath.endsWith('.mjs')) {
        newValue.types = importPath.replace(/\.mjs$/, '.d.mts');
      } else if (importPath.endsWith('.js')) {
        newValue.types = importPath.replace(/\.js$/, '.d.ts');
      }
    }
  }

  return [newExportPath, newValue];
}

function transformPluginutilsExport(
  exportPath: string,
  exportValue: unknown,
): [string, ExportValue] {
  // Skip package.json
  if (exportPath === './package.json') {
    return ['', null];
  }

  // Transform . -> ./rolldown/pluginutils
  const newExportPath =
    exportPath === '.' ? './rolldown/pluginutils' : `./rolldown/pluginutils${exportPath.slice(1)}`;

  // Transform paths
  const transformValue = (value: unknown): ExportValue => {
    if (typeof value === 'string') {
      if (value.startsWith('./src/')) {
        return null;
      }
      return value.replace(/^\.\/dist\//, './dist/pluginutils/');
    }

    if (value && typeof value === 'object') {
      const result: Record<string, unknown> = {};
      for (const [key, val] of Object.entries(value)) {
        if (key === 'dev') {
          continue;
        }
        const transformed = transformValue(val);
        if (transformed !== null) {
          result[key] = transformed;
        }
      }
      return Object.keys(result).length > 0 ? (result as ExportValue) : null;
    }

    return value as ExportValue;
  };

  const newValue = transformValue(exportValue);

  // Handle string values or add types if missing
  if (typeof newValue === 'string') {
    // Convert string to object with default and types
    if (newValue.endsWith('.js')) {
      return [
        newExportPath,
        {
          default: newValue,
          types: newValue.replace(/\.js$/, '.d.ts'),
        },
      ];
    }
    return [newExportPath, newValue];
  }

  if (newValue && typeof newValue === 'object') {
    const importPath = ('import' in newValue ? newValue.import : newValue.default) as
      | string
      | undefined;
    if (importPath && !('types' in newValue)) {
      if (importPath.endsWith('.js')) {
        newValue.types = importPath.replace(/\.js$/, '.d.ts');
      }
    }
  }

  return [newExportPath, newValue];
}

function transformViteExport(exportPath: string, exportValue: unknown): [string, ExportValue] {
  // Skip package.json
  if (exportPath === './package.json') {
    return ['', null];
  }

  // Keys remain unchanged
  const newExportPath = exportPath;

  // Transform paths in values
  const transformValue = (value: unknown): ExportValue => {
    if (typeof value === 'string') {
      // Transform types paths
      if (value.startsWith('./types/')) {
        return value.replace(/^\.\/types\//, './dist/vite/types/');
      } else if (value.startsWith('./dist')) {
        return value.replace(/^\.\/dist\//, './dist/vite/');
      }

      return `./dist/vite/${value.slice(2)}`;
    }

    if (value && typeof value === 'object') {
      const result: Record<string, unknown> = {};
      for (const [key, val] of Object.entries(value)) {
        const transformed = transformValue(val);
        if (transformed !== null) {
          result[key] = transformed;
        }
      }
      return Object.keys(result).length > 0 ? (result as ExportValue) : null;
    }

    return value as ExportValue;
  };

  const newValue = transformValue(exportValue);

  if (newValue && typeof newValue === 'object') {
    const importPath = ('import' in newValue ? newValue.import : newValue.default) as
      | string
      | undefined;
    if (importPath && !('types' in newValue) && typeof importPath === 'string') {
      if (importPath.endsWith('.js')) {
        newValue.types = importPath.replace(/\.js$/, '.d.ts');
      }
    }
  }

  return [newExportPath, newValue];
}

function mergePackageExports(
  corePkg: PackageJson,
  rolldownPkg: PackageJson,
  rolldownVitePkg: PackageJson,
  pluginutilsPkg: PackageJson,
): Record<string, unknown> {
  const result: Record<string, unknown> = {};

  if (corePkg.exports) {
    for (const [path, value] of Object.entries(corePkg.exports)) {
      result[path] = value;
    }
  }

  // Add rolldown exports
  if (rolldownPkg.exports) {
    for (const [path, value] of Object.entries(rolldownPkg.exports)) {
      const [newPath, newValue] = transformRolldownExport(path, value);
      if (newPath && newValue !== null) {
        result[newPath] = newValue;
      }
    }
  }

  // Add pluginutils exports
  if (pluginutilsPkg.exports) {
    for (const [path, value] of Object.entries(pluginutilsPkg.exports)) {
      const [newPath, newValue] = transformPluginutilsExport(path, value);
      if (newPath && newValue !== null) {
        result[newPath] = newValue;
      }
    }
  }

  // Add vite exports
  if (rolldownVitePkg.exports) {
    for (const [path, value] of Object.entries(rolldownVitePkg.exports)) {
      const [newPath, newValue] = transformViteExport(path, value);
      if (newPath && newValue !== null) {
        result[newPath] = newValue;
      }
    }
  }

  // Sort exports by key
  return Object.keys(result)
    .toSorted()
    .reduce(
      (sorted, key) => {
        sorted[key] = result[key];
        return sorted;
      },
      {} as Record<string, unknown>,
    );
}

// Oxc-related packages that should use the higher version on conflict
const OXC_PACKAGE_PREFIXES = [
  '@oxc-project/',
  '@oxlint/',
  '@oxc-minify/',
  '@oxc-parser/',
  '@oxc-resolver/',
  '@oxc-transform/',
  '@oxfmt/',
  '@oxlint-tsgolint/',
];
const OXC_PACKAGES = new Set([
  'oxc-minify',
  'oxc-parser',
  'oxc-transform',
  'oxfmt',
  'oxlint',
  'oxlint-tsgolint',
]);
const VITEST_DEPS = new Set(['tinybench']);

// These packages should always use the highest version
function syncedPackages(packageName: string): boolean {
  if (OXC_PACKAGES.has(packageName) || VITEST_DEPS.has(packageName)) {
    return true;
  }
  return OXC_PACKAGE_PREFIXES.some((prefix) => packageName.startsWith(prefix));
}

function mergeSemverVersions(
  v1: string,
  v2: string,
  packageName: string,
  semver: typeof import('semver'),
): string {
  // Handle special cases
  if (v1 === v2) {
    return v1;
  }

  // Handle exact version specifiers (=)
  const isExact1 = v1.startsWith('=');
  const isExact2 = v2.startsWith('=');
  if (isExact1 || isExact2) {
    if (isExact1 && isExact2 && v1 !== v2) {
      // For oxc-related packages, use the higher version
      if (syncedPackages(packageName)) {
        const ver1 = v1.slice(1); // Remove '=' prefix
        const ver2 = v2.slice(1);
        if (semver.valid(ver1) && semver.valid(ver2)) {
          const higher = semver.gt(ver1, ver2) ? v1 : v2;
          log(`Resolving ${packageName} version conflict: ${v1} vs ${v2} -> ${higher}`);
          return higher;
        }
      }
      error(`Incompatible exact versions for ${packageName}: ${v1} vs ${v2}`);
    }
    return isExact1 ? v1 : v2;
  }

  if (v1.startsWith('npm:') || v2.startsWith('npm:')) {
    if (!v1.startsWith('npm:')) {
      return v1;
    }
    if (!v2.startsWith('npm:')) {
      return v2;
    }
    return v1;
  }

  const range1 = semver.validRange(v1);
  const range2 = semver.validRange(v2);

  if (!range1 || !range2) {
    log(`Warning: Could not parse semver for ${packageName}: ${v1}, ${v2}. Using ${v1}`);
    return v1;
  }

  const major1 = getMajor(v1);
  const major2 = getMajor(v2);

  if (major1 === null || major2 === null) {
    return v1;
  }

  // Check if major versions are compatible
  if (major1 !== major2) {
    // For synced packages, use the higher major version
    if (syncedPackages(packageName)) {
      const higher = major1 > major2 ? v1 : v2;
      log(`Resolving ${packageName} major version conflict: ${v1} vs ${v2} -> ${higher}`);
      return higher;
    }
    error(
      `Incompatible semver ranges for ${packageName}: ${v1} (major: ${major1}) vs ${v2} (major: ${major2})`,
    );
  }

  // Both have same major version, return the higher one
  // Compare the minimum versions
  const minVersion1 = semver.minVersion(range1);
  const minVersion2 = semver.minVersion(range2);

  if (minVersion1 && minVersion2) {
    if (semver.gt(minVersion1, minVersion2)) {
      return v1;
    } else if (semver.gt(minVersion2, minVersion1)) {
      return v2;
    }
  }

  return v1;
}

function mergePnpmWorkspaces(
  main: PnpmWorkspace,
  rolldown: PnpmWorkspace,
  rolldownVite: PnpmWorkspace,
  semver: typeof import('semver'),
): PnpmWorkspace {
  const result: PnpmWorkspace = { ...main };

  // Merge packages array
  const packagesSet = new Set(main.packages || []);
  // Add rolldown packages
  packagesSet.add(ROLLDOWN_DIR);
  packagesSet.add(`${ROLLDOWN_DIR}/packages/*`);
  // Add vite packages
  packagesSet.add(VITE_DIR);
  packagesSet.add(`${VITE_DIR}/packages/*`);
  result.packages = Array.from(packagesSet);

  // Merge catalog
  const catalog: Record<string, string> = { ...main.catalog };

  // Add all entries from rolldown catalog
  for (const [pkg, version] of Object.entries(rolldown.catalog || {})) {
    if (catalog[pkg]) {
      // Merge versions
      catalog[pkg] = mergeSemverVersions(catalog[pkg], version, pkg, semver);
    } else {
      catalog[pkg] = version;
    }
  }

  // Add all entries from vite catalog (if it has one)
  for (const [pkg, version] of Object.entries(rolldownVite.catalog || {})) {
    if (catalog[pkg]) {
      // Merge versions
      catalog[pkg] = mergeSemverVersions(catalog[pkg], version, pkg, semver);
    } else {
      catalog[pkg] = version;
    }
  }

  // Remove vite from catalog
  delete catalog.vite;

  // Sort catalog keys alphabetically
  result.catalog = Object.keys(catalog)
    .toSorted()
    .reduce(
      (sorted, key) => {
        sorted[key] = catalog[key];
        return sorted;
      },
      {} as Record<string, string>,
    );

  // Merge minimumReleaseAgeExclude
  const excludeSet = new Set(main.minimumReleaseAgeExclude || []);

  (rolldown.minimumReleaseAgeExclude || []).forEach((item) => excludeSet.add(item));
  (rolldownVite.minimumReleaseAgeExclude || []).forEach((item) => excludeSet.add(item));
  result.minimumReleaseAgeExclude = Array.from(excludeSet);

  // Copy patchedDependencies from vite (with path prefix)
  if (rolldownVite.patchedDependencies) {
    result.patchedDependencies = {};
    for (const [dep, patchPath] of Object.entries(rolldownVite.patchedDependencies)) {
      // Prepend vite directory to patch paths
      result.patchedDependencies[dep] = patchPath.startsWith('./')
        ? `./${VITE_DIR}/${patchPath.slice(2)}`
        : `${VITE_DIR}/${patchPath}`;
    }
  }

  // Merge peerDependencyRules
  if (rolldownVite.peerDependencyRules) {
    result.peerDependencyRules = {
      ...main.peerDependencyRules,
      allowedVersions: {
        ...main.peerDependencyRules?.allowedVersions,
        ...rolldownVite.peerDependencyRules.allowedVersions,
      },
    };
    // Add rolldown to allowed versions
    if (result.peerDependencyRules.allowedVersions) {
      result.peerDependencyRules.allowedVersions.rolldown = '*';
    }
  }

  // Copy packageExtensions from vite
  if (rolldownVite.packageExtensions) {
    result.packageExtensions = {
      ...main.packageExtensions,
      ...rolldownVite.packageExtensions,
    };
  }

  // Set ignoreScripts
  result.ignoreScripts = true;

  return result;
}

export async function syncRemote() {
  const { values } = parseArgs({
    options: {
      clean: {
        type: 'boolean',
      },
      'update-hashes': {
        type: 'boolean',
      },
    },
    args: process.argv.slice(3),
  });

  log('Starting rolldown/vite sync...');

  // Get the root directory (assuming script is run from root)
  const rootDir = process.cwd();

  if (values.clean) {
    log('Cleaning existing repositories...');
    if (existsSync(join(rootDir, ROLLDOWN_DIR))) {
      rmSync(join(rootDir, ROLLDOWN_DIR), { recursive: true, force: true });
      log(`Removed ${ROLLDOWN_DIR}`);
    }
    if (existsSync(join(rootDir, VITE_DIR))) {
      rmSync(join(rootDir, VITE_DIR), {
        recursive: true,
        force: true,
      });
      log(`Removed ${VITE_DIR}`);
    }
    // Clean up legacy 'rolldown-vite' directory (renamed to 'vite')
    const legacyViteDir = join(rootDir, 'rolldown-vite');
    if (existsSync(legacyViteDir)) {
      rmSync(legacyViteDir, { recursive: true, force: true });
      log('Removed legacy rolldown-vite directory');
    }
  }

  // Clone or reset repos
  cloneOrResetRepo(
    upstreamVersions.rolldown.repo,
    join(rootDir, ROLLDOWN_DIR),
    upstreamVersions.rolldown.branch,
    upstreamVersions.rolldown.hash,
  );
  cloneOrResetRepo(
    upstreamVersions['vite'].repo,
    join(rootDir, VITE_DIR),
    upstreamVersions['vite'].branch,
    upstreamVersions['vite'].hash,
  );

  // Dynamically import dependencies after git clone
  let parseYaml: typeof import('yaml').parse;
  let stringifyYaml: typeof import('yaml').stringify;
  let semver: typeof import('semver');

  try {
    const yaml = await import('yaml');
    parseYaml = yaml.parse;
    stringifyYaml = yaml.stringify;
    semver = await import('semver');
  } catch {
    log('Dependencies not found, running pnpm install...');
    execCommand('pnpm install --no-frozen-lockfile', rootDir);
    log('Retrying imports...');
    const yaml = await import('yaml');
    parseYaml = yaml.parse;
    stringifyYaml = yaml.stringify;
    semver = await import('semver');
  }

  log('Reading pnpm-workspace.yaml files...');

  // Read main pnpm-workspace.yaml
  const mainWorkspacePath = join(rootDir, 'pnpm-workspace.yaml');
  const mainWorkspace = parseYaml(readFileSync(mainWorkspacePath, 'utf-8')) as PnpmWorkspace;

  // Read rolldown pnpm-workspace.yaml
  const rolldownWorkspacePath = join(rootDir, ROLLDOWN_DIR, 'pnpm-workspace.yaml');
  const rolldownWorkspace = parseYaml(
    readFileSync(rolldownWorkspacePath, 'utf-8'),
  ) as PnpmWorkspace;

  // Read vite pnpm-workspace.yaml
  const rolldownViteWorkspacePath = join(rootDir, VITE_DIR, 'pnpm-workspace.yaml');
  const rolldownViteWorkspace = parseYaml(
    readFileSync(rolldownViteWorkspacePath, 'utf-8'),
  ) as PnpmWorkspace;

  log('Merging pnpm-workspace.yaml files...');

  const mergedWorkspace = mergePnpmWorkspaces(
    mainWorkspace,
    rolldownWorkspace,
    rolldownViteWorkspace,
    semver,
  );

  // Write the merged workspace back
  const yamlContent = stringifyYaml(mergedWorkspace, {
    lineWidth: -1,
  });

  writeFileSync(mainWorkspacePath, yamlContent, 'utf-8');

  log('✓ pnpm-workspace.yaml updated successfully!');

  execCommand('pnpm install --no-frozen-lockfile', rootDir);

  // Merge package.json exports
  log('Merging package.json exports...');

  const corePackagePath = join(rootDir, CORE_PACKAGE_PATH, 'package.json');
  const rolldownPackagePath = join(rootDir, ROLLDOWN_DIR, 'packages', 'rolldown', 'package.json');
  const rolldownVitePackagePath = join(rootDir, VITE_DIR, 'packages', 'vite', 'package.json');
  const pluginutilsPackagePath = join(
    rootDir,
    ROLLDOWN_DIR,
    'packages',
    'pluginutils',
    'package.json',
  );

  const corePackage = JSON.parse(readFileSync(corePackagePath, 'utf-8')) as PackageJson;
  const rolldownPackage = JSON.parse(readFileSync(rolldownPackagePath, 'utf-8')) as PackageJson;
  const rolldownVitePackage = JSON.parse(
    readFileSync(rolldownVitePackagePath, 'utf-8'),
  ) as PackageJson;
  const pluginutilsPackage = JSON.parse(
    readFileSync(pluginutilsPackagePath, 'utf-8'),
  ) as PackageJson;

  const mergedExports = mergePackageExports(
    corePackage,
    rolldownPackage,
    rolldownVitePackage,
    pluginutilsPackage,
  );

  // additional tsdown exports (vp pack)
  mergedExports['./pack'] = {
    default: './dist/tsdown/index.js',
    types: './dist/tsdown/index-types.d.ts',
  };

  // Update CLI package.json with merged exports
  corePackage.exports = mergedExports;

  writeFileSync(corePackagePath, JSON.stringify(corePackage, null, 2) + '\n', 'utf-8');

  log('✓ package.json exports updated successfully!');

  // Apply Vite+ branding patches to vite source
  const { brandVite } = await import('./brand-vite.js');
  brandVite(rootDir);

  log('✓ Done!');
}
