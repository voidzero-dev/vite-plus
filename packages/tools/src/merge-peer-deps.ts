import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import * as semver from 'semver';

interface PackageJson {
  name?: string;
  peerDependencies?: Record<string, string>;
  peerDependenciesMeta?: Record<string, { optional?: boolean }>;
  [key: string]: any;
}

function log(message: string) {
  console.log(`[merge-peer-deps] ${message}`);
}

function error(message: string): never {
  console.error(`[merge-peer-deps] ERROR: ${message}`);
  process.exit(1);
}

function mergeSemverVersions(
  v1: string,
  v2: string,
  packageName: string,
): string {
  // Handle special cases
  if (v1 === v2) return v1;

  // Handle exact version specifiers (=)
  const isExact1 = v1.startsWith('=');
  const isExact2 = v2.startsWith('=');
  if (isExact1 || isExact2) {
    if (isExact1 && isExact2 && v1 !== v2) {
      error(`Incompatible exact versions for ${packageName}: ${v1} vs ${v2}`);
    }
    return isExact1 ? v1 : v2;
  }

  // Handle npm: prefix
  if (v1.startsWith('npm:') || v2.startsWith('npm:')) {
    // If one has npm: prefix, prefer the non-npm version or return the first one
    if (!v1.startsWith('npm:')) return v1;
    if (!v2.startsWith('npm:')) return v2;
    return v1;
  }

  // Handle workspace: prefix
  if (v1.startsWith('workspace:') || v2.startsWith('workspace:')) {
    if (v1.startsWith('workspace:')) return v1;
    if (v2.startsWith('workspace:')) return v2;
    return v1;
  }

  // Handle wildcards
  if (v1 === '*' || v2 === '*') {
    // Prefer specific version over wildcard
    if (v1 === '*') return v2;
    if (v2 === '*') return v1;
  }

  // Parse version ranges
  const range1 = semver.validRange(v1);
  const range2 = semver.validRange(v2);

  if (!range1 || !range2) {
    log(
      `Warning: Could not parse semver for ${packageName}: ${v1}, ${v2}. Using ${v1}`,
    );
    return v1;
  }

  // Get the major versions from the ranges
  const getMajor = (range: string): number | null => {
    const match = range.match(/(\d+)\./);
    return match ? parseInt(match[1], 10) : null;
  };

  const major1 = getMajor(v1);
  const major2 = getMajor(v2);

  if (major1 === null || major2 === null) {
    return v1;
  }

  // Check if major versions are compatible
  if (major1 !== major2) {
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

function mergePeerDependencies(
  packages: PackageJson[],
): Record<string, string> {
  const result: Record<string, string> = {};

  for (const pkg of packages) {
    if (!pkg.peerDependencies) continue;

    for (const [dep, version] of Object.entries(pkg.peerDependencies)) {
      if (result[dep]) {
        // Merge versions
        result[dep] = mergeSemverVersions(result[dep], version, dep);
      } else {
        result[dep] = version;
      }
    }
  }

  // Sort alphabetically
  return Object.keys(result)
    .sort()
    .reduce(
      (sorted, key) => {
        sorted[key] = result[key];
        return sorted;
      },
      {} as Record<string, string>,
    );
}

function mergePeerDependenciesMeta(
  packages: PackageJson[],
): Record<string, { optional?: boolean }> {
  const result: Record<string, { optional?: boolean }> = {};

  for (const pkg of packages) {
    if (!pkg.peerDependenciesMeta) continue;

    for (const [dep, meta] of Object.entries(pkg.peerDependenciesMeta)) {
      if (!result[dep]) {
        result[dep] = { ...meta };
      } else {
        // If any package marks it as optional, keep it optional
        if (meta.optional) {
          result[dep].optional = true;
        }
      }
    }
  }

  // Sort alphabetically
  return Object.keys(result)
    .sort()
    .reduce(
      (sorted, key) => {
        sorted[key] = result[key];
        return sorted;
      },
      {} as Record<string, { optional?: boolean }>,
    );
}

export function mergePeerDeps() {
  log('Starting peerDependencies merge...');

  const rootDir = process.cwd();

  // Paths to package.json files
  const cliPackagePath = join(rootDir, 'packages/cli/package.json');
  const vitepressPackagePath = join(
    rootDir,
    'packages/cli/node_modules/vitepress/package.json',
  );
  const tsdownPackagePath = join(
    rootDir,
    'packages/cli/node_modules/tsdown/package.json',
  );
  const vitestPackagePath = join(
    rootDir,
    'packages/cli/node_modules/vitest-dev/package.json',
  );
  const rolldownVitePackagePath = join(
    rootDir,
    'rolldown-vite/packages/vite/package.json',
  );

  // Check if all files exist
  const packagePaths = [
    { path: vitepressPackagePath, name: 'vitepress' },
    { path: tsdownPackagePath, name: 'tsdown' },
    { path: vitestPackagePath, name: 'vitest' },
    { path: rolldownVitePackagePath, name: 'rolldown-vite' },
  ];

  const packages: PackageJson[] = [];

  for (const { path, name } of packagePaths) {
    if (!existsSync(path)) {
      log(`Warning: ${name} package.json not found at ${path}, skipping...`);
      continue;
    }
    const pkg = JSON.parse(readFileSync(path, 'utf-8')) as PackageJson;
    packages.push(pkg);
    log(`Loaded ${name} package.json`);
  }

  if (packages.length === 0) {
    error('No package.json files found to merge');
  }

  log(`Merging peerDependencies from ${packages.length} packages...`);

  const mergedPeerDeps = mergePeerDependencies(packages);
  const mergedPeerDepsMeta = mergePeerDependenciesMeta(packages);

  log(`Merged ${Object.keys(mergedPeerDeps).length} peerDependencies`);
  log(
    `Merged ${Object.keys(mergedPeerDepsMeta).length} peerDependenciesMeta entries`,
  );

  // Read CLI package.json
  const cliPackage = JSON.parse(
    readFileSync(cliPackagePath, 'utf-8'),
  ) as PackageJson;

  // Update with merged dependencies
  cliPackage.peerDependencies = mergedPeerDeps;
  cliPackage.peerDependenciesMeta = mergedPeerDepsMeta;

  // Write back to CLI package.json
  writeFileSync(
    cliPackagePath,
    JSON.stringify(cliPackage, null, 2) + '\n',
    'utf-8',
  );

  log('✓ peerDependencies merged successfully!');
  log('✓ Done!');
}
