import fs from 'node:fs';
import path from 'node:path';

import { globSync } from 'glob';
import { minimatch } from 'minimatch';
import { Scalar, YAMLSeq } from 'yaml';

import { detectWorkspace as detectWorkspaceBinding } from '../../binding/index.js';
import {
  DependencyType,
  PackageManager,
  type WorkspaceInfo,
  type WorkspaceInfoOptional,
  type WorkspacePackage,
} from '../types/index.js';
import { editJsonFile, readJsonFile } from './json.js';
import { getScopeFromPackageName } from './package.js';
import { editYamlFile, readYamlFile } from './yaml.js';

export function findPackageJsonFilesFromPatterns(patterns: string[], cwd: string): string[] {
  if (patterns.length === 0) {
    return [];
  }
  return globSync(
    patterns.map((pattern) => `${pattern}/package.json`),
    { absolute: true, cwd },
  );
}

// Detect if we're in a monorepo and get workspace info
export async function detectWorkspace(rootDir: string): Promise<WorkspaceInfoOptional> {
  const bindingResult = await detectWorkspaceBinding(rootDir);
  const result: WorkspaceInfoOptional = {
    rootDir,
    packageManager: undefined,
    packageManagerVersion: 'latest',
    isMonorepo: false,
    monorepoScope: '',
    workspacePatterns: [],
    parentDirs: [],
    packages: [],
  };
  if (bindingResult.packageManagerName) {
    result.packageManager = bindingResult.packageManagerName as PackageManager;
  }
  if (bindingResult.packageManagerVersion) {
    result.packageManagerVersion = bindingResult.packageManagerVersion;
  }
  if (bindingResult.isMonorepo) {
    result.isMonorepo = bindingResult.isMonorepo;
  }
  if (bindingResult.root) {
    // automatically correct the root directory from cwd
    result.rootDir = bindingResult.root;
  }

  // Extract parent directories from workspace patterns
  if (result.isMonorepo) {
    const pnpmWorkspaceFile = path.join(result.rootDir, 'pnpm-workspace.yaml');
    const packageJsonFile = path.join(result.rootDir, 'package.json');
    if (fs.existsSync(pnpmWorkspaceFile)) {
      const workspaceConfig = readYamlFile<{ packages?: string[] }>(pnpmWorkspaceFile);
      if (Array.isArray(workspaceConfig.packages)) {
        result.workspacePatterns = workspaceConfig.packages;
      }
    } else if (fs.existsSync(packageJsonFile)) {
      // Check for npm/yarn workspace
      const pkg = readJsonFile<{ workspaces?: string[] }>(packageJsonFile);
      if (Array.isArray(pkg.workspaces)) {
        result.workspacePatterns = pkg.workspaces;
      }
    }

    const dirs = new Set<string>();
    for (const pattern of result.workspacePatterns) {
      // Extract directory from patterns like "apps/*", "packages/*", "foo/bar/*", "website", etc
      if (!pattern.endsWith('*')) {
        continue;
      }
      // Extract the directory name, ignore the wildcard
      const dir = pattern.replace(/\/\*{1,2}$/, '');
      if (dir) {
        dirs.add(dir);
      }
    }
    // eslint-disable-next-line unicorn/no-array-sort -- safe: sorting a fresh Array.from copy
    result.parentDirs = Array.from(dirs).sort();

    // Extract the scope from the package.json
    const pkg = readJsonFile<{ name?: string }>(packageJsonFile);
    if (pkg.name) {
      result.monorepoScope = getScopeFromPackageName(pkg.name);
    }
    result.packages = discoverWorkspacePackages(result.workspacePatterns, result.rootDir);
  }

  return result;
}

// Discover all workspace packages
export function discoverWorkspacePackages(
  workspacePatterns: string[],
  rootDir: string,
): WorkspacePackage[] {
  const packages: WorkspacePackage[] = [];

  if (workspacePatterns.length === 0) {
    return packages;
  }

  // Find all package.json files in the workspace
  const packageJsonRelativePaths = globSync(
    workspacePatterns.map((pattern) => `${pattern}/package.json`),
    {
      absolute: false,
      cwd: rootDir,
      ignore: ['**/node_modules/**'],
    },
  );
  for (const packageJsonRelativePath of packageJsonRelativePaths) {
    const packageJsonPath = path.join(rootDir, packageJsonRelativePath);
    const pkg = readJsonFile<{
      name?: string;
      description?: string;
      version?: string;
      dependencies?: Record<string, string>;
      keywords?: string[];
    }>(packageJsonPath);
    if (!pkg.name) {
      continue;
    }
    const isTemplatePackage =
      pkg.keywords?.includes('vite-plus-template') ||
      pkg.keywords?.includes('bingo-template') ||
      !!pkg.dependencies?.bingo;
    packages.push({
      name: pkg.name,
      path: path.dirname(packageJsonRelativePath),
      description: pkg.description,
      version: pkg.version,
      isTemplatePackage,
    });
  }

  return packages;
}

// Update package.json with workspace dependencies
export function updatePackageJsonWithDeps(
  rootDir: string,
  projectDir: string,
  dependencies: string[],
  dependencyType: DependencyType,
) {
  const packageJsonPath = path.join(rootDir, projectDir, 'package.json');
  editJsonFile<{ [key in DependencyType]?: Record<string, string> }>(packageJsonPath, (pkg) => {
    if (!pkg[dependencyType]) {
      pkg[dependencyType] = {};
    }
    for (const dep of dependencies) {
      pkg[dependencyType][dep] = 'workspace:*';
    }
    return pkg;
  });
}

// Update workspace configuration to include new project
export function updateWorkspaceConfig(projectPath: string, workspaceInfo: WorkspaceInfo) {
  // Check if project path matches any workspace pattern
  for (const pattern of workspaceInfo.workspacePatterns) {
    if (minimatch(projectPath, pattern)) {
      return;
    }
  }

  // Derive pattern from project path (e.g., "packages/my-app" -> "packages/*", "website" -> "website", "foo/bar/app" -> "foo/bar/*")
  let pattern = path.dirname(projectPath);
  if (!pattern) {
    // "website" -> "website"
    pattern = projectPath;
  } else {
    // "foo/bar/app" -> "foo/bar/*"
    pattern = `${pattern}/*`;
  }

  if (workspaceInfo.packageManager === PackageManager.pnpm) {
    editYamlFile(path.join(workspaceInfo.rootDir, 'pnpm-workspace.yaml'), (doc) => {
      let packages = doc.getIn(['packages']) as YAMLSeq<Scalar<string>>;
      if (!packages) {
        packages = new YAMLSeq<Scalar<string>>();
      }
      packages.add(new Scalar(pattern));
      doc.setIn(['packages'], packages);
    });
  } else {
    // Update package.json workspaces
    editJsonFile<{ workspaces?: string[] }>(
      path.join(workspaceInfo.rootDir, 'package.json'),
      (pkg) => {
        pkg.workspaces = [...(pkg.workspaces || []), pattern];
        return pkg;
      },
    );
  }
}
