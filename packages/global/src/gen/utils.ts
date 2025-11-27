import fs from 'node:fs';
import path from 'node:path';

import validateNpmPackageName from 'validate-npm-package-name';

import { editJsonFile } from '../utils/index.ts';

// Helper functions for file operations
export function copy(src: string, dest: string) {
  const stat = fs.statSync(src);
  if (stat.isDirectory()) {
    copyDir(src, dest);
  } else {
    fs.copyFileSync(src, dest);
  }
}

export function copyDir(srcDir: string, destDir: string) {
  fs.mkdirSync(destDir, { recursive: true });
  for (const file of fs.readdirSync(srcDir)) {
    const srcFile = path.resolve(srcDir, file);
    const destFile = path.resolve(destDir, file);
    copy(srcFile, destFile);
  }
}

/**
 * Format the target directory into a valid directory name and package name
 *
 * Examples:
 * ```
 * # invalid target directories
 * ./ -> { directory: '', packageName: '', error: 'Invalid target directory' }
 * /foo/bar -> { directory: '', packageName: '', error: 'Absolute path is not allowed' }
 * @scope/ -> { directory: '', packageName: '', error: 'Invalid target directory' }
 * ../../foo/bar -> { directory: '', packageName: '', error: 'Invalid target directory' }
 *
 * # valid target directories
 * ./my-package -> { directory: './my-package', packageName: 'my-package' }
 * ./foo/bar-package -> { directory: './foo/bar-package', packageName: 'bar-package' }
 * ./foo/bar-package/ -> { directory: './foo/bar-package', packageName: 'bar-package' }
 * my-package -> { directory: 'my-package', packageName: 'my-package' }
 * @my-scope/my-package -> { directory: 'my-package', packageName: '@my-scope/my-package' }
 * foo/@my-scope/my-package -> { directory: 'foo/my-package', packageName: '@scope/my-package' }
 * ./foo/@my-scope/my-package -> { directory: './foo/my-package', packageName: '@scope/my-package' }
 * ./foo/bar/@scope/my-package -> { directory: './foo/bar/my-package', packageName: '@scope/my-package' }
 * ```
 */
export function formatTargetDir(input: string): {
  directory: string;
  packageName: string;
  error?: string;
} {
  let targetDir = path.normalize(input.trim());
  const parsed = path.parse(targetDir);
  if (parsed.root || path.isAbsolute(targetDir)) {
    return {
      directory: '',
      packageName: '',
      error: 'Absolute path is not allowed',
    };
  }
  if (targetDir.includes('..')) {
    return {
      directory: '',
      packageName: '',
      error: 'Relative path contains ".." which is not allowed',
    };
  }
  let packageName = parsed.base;
  const parentName = path.basename(parsed.dir);
  if (parentName.startsWith('@')) {
    // skip scope directory
    // ./@my-scope/my-package -> ./my-package
    targetDir = path.join(path.dirname(parsed.dir), packageName);
    packageName = `${parentName}/${packageName}`;
  }
  const result = validateNpmPackageName(packageName);
  if (!result.validForNewPackages) {
    // invalid package name
    const message = result.errors?.[0] ?? result.warnings?.[0] ?? 'Invalid package name';
    return {
      directory: '',
      packageName: '',
      error: `Parsed package name "${packageName}" is invalid: ${message}`,
    };
  }
  return { directory: targetDir, packageName };
}

// Get the project directory from the project name
// If the project name is a scoped package name, return the second part
// Otherwise, return the project name
export function getProjectDirFromPackageName(packageName: string) {
  if (packageName.startsWith('@')) {
    return packageName.split('/')[1];
  }
  return packageName;
}

export function setPackageName(projectDir: string, packageName: string) {
  editJsonFile<{ name?: string }>(path.join(projectDir, 'package.json'), (pkg) => {
    pkg.name = packageName;
    return pkg;
  });
}
