import { parse as parseYaml, stringify as stringifyYaml } from '@std/yaml';
import { execSync, spawnSync } from 'node:child_process';
import { existsSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { parseArgs } from 'node:util';
import * as semver from 'semver';

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
  packageExtensions?: Record<string, any>;
  overrides?: Record<string, string>;
  ignoreScripts?: boolean;
  [key: string]: any;
}

const ROLLDOWN_REPO = 'git@github.com:rolldown/rolldown.git';
const ROLLDOWN_VITE_REPO = 'git@github.com:vitejs/rolldown-vite.git';
const ROLLDOWN_DIR = 'rolldown';
const ROLLDOWN_VITE_DIR = 'rolldown-vite';
const ROLLDOWN_VITE_BRANCH = 'rolldown-vite';

function log(message: string) {
  console.log(`[sync-rolldown] ${message}`);
}

function error(message: string): never {
  console.error(`[sync-rolldown] ERROR: ${message}`);
  process.exit(1);
}

function execCommand(command: string, cwd?: string): string {
  try {
    return execSync(command, {
      cwd,
      encoding: 'utf-8',
      stdio: 'pipe',
    }).trim();
  } catch (err: any) {
    throw new Error(`Failed to execute: ${command}\n${err.message}`);
  }
}

function cloneOrResetRepo(
  repoUrl: string,
  dir: string,
  branch: string = 'main',
) {
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
        cloneRepo(repoUrl, dir, branch);
        return;
      }

      // Check remote URL
      const remoteUrl = execCommand('git remote get-url origin', dir);
      if (remoteUrl !== repoUrl) {
        log(
          `${dir} has wrong remote (${remoteUrl} vs ${repoUrl}), removing and re-cloning...`,
        );
        rmSync(dir, { recursive: true, force: true });
        cloneRepo(repoUrl, dir, branch);
        return;
      }

      // Reset to latest
      log(`Resetting ${dir} to latest ${branch}...`);
      execCommand('git fetch origin', dir);
      execCommand(`git checkout ${branch}`, dir);
      execCommand(`git reset --hard origin/${branch}`, dir);
      execCommand('git clean -fdx', dir);
      log(`${dir} reset to latest ${branch}`);
    } catch (err: any) {
      log(
        `Failed to reset ${dir} (${err.message}), removing and re-cloning...`,
      );
      rmSync(dir, { recursive: true, force: true });
      cloneRepo(repoUrl, dir, branch);
    }
  } else {
    cloneRepo(repoUrl, dir, branch);
  }
}

function cloneRepo(repoUrl: string, dir: string, branch: string) {
  log(`Cloning ${repoUrl} (${branch}) into ${dir}...`);
  execCommand(`git clone --branch ${branch} ${repoUrl} ${dir}`);
  log(`${dir} cloned successfully`);
}

function mergeSemverVersions(v1: string, v2: string, packageName: string): string {
  // Handle special cases
  if (v1 === v2) return v1;

  // Handle exact version specifiers (=)
  const isExact1 = v1.startsWith('=');
  const isExact2 = v2.startsWith('=');
  if (isExact1 || isExact2) {
    if (isExact1 && isExact2 && v1 !== v2) {
      error(
        `Incompatible exact versions for ${packageName}: ${v1} vs ${v2}`,
      );
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

  // Parse version ranges
  const range1 = semver.validRange(v1);
  const range2 = semver.validRange(v2);

  if (!range1 || !range2) {
    log(`Warning: Could not parse semver for ${packageName}: ${v1}, ${v2}. Using ${v1}`);
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

function mergePnpmWorkspaces(
  main: PnpmWorkspace,
  rolldown: PnpmWorkspace,
  rolldownVite: PnpmWorkspace,
): PnpmWorkspace {
  const result: PnpmWorkspace = { ...main };

  // Merge packages array
  const packagesSet = new Set(main.packages || []);
  // Add rolldown packages
  packagesSet.add(ROLLDOWN_DIR);
  packagesSet.add(`${ROLLDOWN_DIR}/packages/*`);
  // Add rolldown-vite packages
  packagesSet.add(ROLLDOWN_VITE_DIR);
  packagesSet.add(`${ROLLDOWN_VITE_DIR}/packages/*`);
  result.packages = Array.from(packagesSet);

  // Merge catalog
  const catalog: Record<string, string> = { ...main.catalog };

  // Add all entries from rolldown catalog
  for (const [pkg, version] of Object.entries(rolldown.catalog || {})) {
    if (pkg === 'rolldown' || pkg === 'rolldown-vite') {
      // Force workspace:* for rolldown packages
      catalog[pkg] = 'workspace:*';
    } else if (catalog[pkg]) {
      // Merge versions
      catalog[pkg] = mergeSemverVersions(catalog[pkg], version, pkg);
    } else {
      catalog[pkg] = version;
    }
  }

  // Add all entries from rolldown-vite catalog (if it has one)
  for (const [pkg, version] of Object.entries(rolldownVite.catalog || {})) {
    if (pkg === 'rolldown' || pkg === 'rolldown-vite') {
      // Force workspace:* for rolldown packages
      catalog[pkg] = 'workspace:*';
    } else if (catalog[pkg]) {
      // Merge versions
      catalog[pkg] = mergeSemverVersions(catalog[pkg], version, pkg);
    } else {
      catalog[pkg] = version;
    }
  }

  // Sort catalog keys alphabetically
  result.catalog = Object.keys(catalog)
    .sort()
    .reduce((sorted, key) => {
      sorted[key] = catalog[key];
      return sorted;
    }, {} as Record<string, string>);

  // Merge minimumReleaseAgeExclude
  const excludeSet = new Set(main.minimumReleaseAgeExclude || []);
  excludeSet.add('@napi-rs/*');
  (rolldown.minimumReleaseAgeExclude || []).forEach((item) => excludeSet.add(item));
  (rolldownVite.minimumReleaseAgeExclude || []).forEach((item) => excludeSet.add(item));
  result.minimumReleaseAgeExclude = Array.from(excludeSet);

  // Copy patchedDependencies from rolldown-vite (with path prefix)
  if (rolldownVite.patchedDependencies) {
    result.patchedDependencies = {};
    for (
      const [dep, patchPath] of Object.entries(
        rolldownVite.patchedDependencies,
      )
    ) {
      // Prepend rolldown-vite directory to patch paths
      result.patchedDependencies[dep] = patchPath.startsWith('./')
        ? `./${ROLLDOWN_VITE_DIR}/${patchPath.slice(2)}`
        : `${ROLLDOWN_VITE_DIR}/${patchPath}`;
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

  // Copy packageExtensions from rolldown-vite
  if (rolldownVite.packageExtensions) {
    result.packageExtensions = {
      ...main.packageExtensions,
      ...rolldownVite.packageExtensions,
    };
  }

  // Update overrides
  result.overrides = {
    ...main.overrides,
    rolldown: 'workspace:*',
    vite: `./${ROLLDOWN_VITE_DIR}/packages/vite`,
  };

  // Set ignoreScripts
  result.ignoreScripts = true;

  return result;
}

export function syncRemote() {
  const { values } = parseArgs({
    options: {
      'clean': {
        type: 'boolean',
      },
    },
    args: process.argv.slice(3),
  });

  log('Starting rolldown/rolldown-vite sync...');

  // Get the root directory (assuming script is run from root)
  const rootDir = process.cwd();

  if (values.clean) {
    log('Cleaning existing repositories...');
    if (existsSync(join(rootDir, ROLLDOWN_DIR))) {
      rmSync(join(rootDir, ROLLDOWN_DIR), { recursive: true, force: true });
      log(`Removed ${ROLLDOWN_DIR}`);
    }
    if (existsSync(join(rootDir, ROLLDOWN_VITE_DIR))) {
      rmSync(join(rootDir, ROLLDOWN_VITE_DIR), { recursive: true, force: true });
      log(`Removed ${ROLLDOWN_VITE_DIR}`);
    }
  }

  // Clone or reset repos
  cloneOrResetRepo(ROLLDOWN_REPO, join(rootDir, ROLLDOWN_DIR), 'main');
  cloneOrResetRepo(
    ROLLDOWN_VITE_REPO,
    join(rootDir, ROLLDOWN_VITE_DIR),
    ROLLDOWN_VITE_BRANCH,
  );

  log('Reading pnpm-workspace.yaml files...');

  // Read main pnpm-workspace.yaml
  const mainWorkspacePath = join(rootDir, 'pnpm-workspace.yaml');
  const mainWorkspace = parseYaml(
    readFileSync(mainWorkspacePath, 'utf-8'),
  ) as PnpmWorkspace;

  // Read rolldown pnpm-workspace.yaml
  const rolldownWorkspacePath = join(
    rootDir,
    ROLLDOWN_DIR,
    'pnpm-workspace.yaml',
  );
  const rolldownWorkspace = parseYaml(
    readFileSync(rolldownWorkspacePath, 'utf-8'),
  ) as PnpmWorkspace;

  // Read rolldown-vite pnpm-workspace.yaml
  const rolldownViteWorkspacePath = join(
    rootDir,
    ROLLDOWN_VITE_DIR,
    'pnpm-workspace.yaml',
  );
  const rolldownViteWorkspace = parseYaml(
    readFileSync(rolldownViteWorkspacePath, 'utf-8'),
  ) as PnpmWorkspace;

  log('Merging pnpm-workspace.yaml files...');

  const mergedWorkspace = mergePnpmWorkspaces(
    mainWorkspace,
    rolldownWorkspace,
    rolldownViteWorkspace,
  );

  // Write the merged workspace back
  const yamlContent = stringifyYaml(mergedWorkspace, {
    lineWidth: -1,
  });

  writeFileSync(mainWorkspacePath, yamlContent, 'utf-8');

  log('✓ pnpm-workspace.yaml updated successfully!');
  log('✓ Done!');
}
