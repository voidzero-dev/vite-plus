import fs from 'node:fs';
import path from 'node:path';

import { parse as parseYaml, stringify as stringifyYaml } from '@std/yaml';
import type { DownloadPackageManagerResult } from '@voidzero-dev/vite-plus/binding';
import validateNpmPackageName from 'validate-npm-package-name';

// Get the package root directory (packages/global)
// Built files are in dist/, templates are in templates/
// So from dist/ we need to go up to the package root
export const pkgRoot = import.meta.dirname.endsWith('dist')
  ? path.dirname(import.meta.dirname)
  : path.join(import.meta.dirname, '../..');

export const templatesDir = path.join(pkgRoot, 'templates');

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

export function editFile(file: string, callback: (content: string) => string) {
  const content = fs.readFileSync(file, 'utf-8');
  fs.writeFileSync(file, callback(content), 'utf-8');
}

export function editOrCreateFile(file: string, callback: (content: string) => string) {
  if (!fs.existsSync(file)) {
    fs.writeFileSync(file, '', 'utf-8');
  }
  editFile(file, callback);
}

export function readYamlFile<T = Record<string, any>>(file: string): T {
  const content = fs.readFileSync(file, 'utf-8');
  return parseYaml(content) as T;
}

export function editYamlFile<T = Record<string, any>>(file: string, callback: (content: T) => T) {
  const yaml = readYamlFile<T>(file);
  const newYaml = callback(yaml);
  fs.writeFileSync(file, stringifyYaml(newYaml), 'utf-8');
}

export function editOrCreateYamlFile<T = Record<string, any>>(file: string, callback: (content: T) => T) {
  if (!fs.existsSync(file)) {
    fs.writeFileSync(file, '', 'utf-8');
  }
  editYamlFile(file, callback);
}

export function readJsonFile<T = Record<string, any>>(file: string): T {
  const content = fs.readFileSync(file, 'utf-8');
  return JSON.parse(content) as T;
}

export function editJsonFile<T = Record<string, any>>(file: string, callback: (content: T) => T) {
  const json = readJsonFile<T>(file);
  const newJson = callback(json);
  fs.writeFileSync(file, JSON.stringify(newJson, null, 2) + '\n', 'utf-8');
}

export function isEmpty(path: string) {
  const files = fs.readdirSync(path);
  return files.length === 0 || (files.length === 1 && files[0] === '.git');
}

export function emptyDir(dir: string) {
  if (!fs.existsSync(dir)) {
    return;
  }
  for (const file of fs.readdirSync(dir)) {
    if (file === '.git') {
      continue;
    }
    fs.rmSync(path.resolve(dir, file), { recursive: true, force: true });
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
export function formatTargetDir(input: string): { directory: string; packageName: string; error?: string } {
  let targetDir = path.normalize(input.trim());
  const parsed = path.parse(targetDir);
  if (parsed.root || path.isAbsolute(targetDir)) {
    return { directory: '', packageName: '', error: 'Absolute path is not allowed' };
  }
  if (targetDir.includes('..')) {
    return { directory: '', packageName: '', error: 'Relative path contains ".." which is not allowed' };
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
    return { directory: '', packageName: '', error: `Parsed package name "${packageName}" is invalid: ${message}` };
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

export function getScopeFromPackageName(packageName: string) {
  if (packageName.startsWith('@')) {
    return packageName.split('/')[0];
  }
  return '';
}

export const RENAME_FILES: Record<string, string> = {
  _gitignore: '.gitignore',
  '_npmrc': '.npmrc',
  '_yarnrc.yml': '.yarnrc.yml',
};

export function renameFiles(projectDir: string) {
  for (const [from, to] of Object.entries(RENAME_FILES)) {
    const fromPath = path.join(projectDir, from);
    if (fs.existsSync(fromPath)) {
      fs.renameSync(fromPath, path.join(projectDir, to));
    }
  }
}

export function setPackageManager(projectDir: string, downloadPackageManager: DownloadPackageManagerResult) {
  // set package manager
  editJsonFile<{ packageManager?: string }>(path.join(projectDir, 'package.json'), (pkg) => {
    if (!pkg.packageManager) {
      pkg.packageManager = `${downloadPackageManager.name}@${downloadPackageManager.version}`;
    }
    return pkg;
  });
}

export function setPackageName(projectDir: string, packageName: string) {
  editJsonFile<{ name?: string }>(path.join(projectDir, 'package.json'), (pkg) => {
    pkg.name = packageName;
    return pkg;
  });
}
