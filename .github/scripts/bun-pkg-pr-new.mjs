#!/usr/bin/env node

import fs from 'node:fs';
import path from 'node:path';

function usage() {
  console.error(`Usage:
  bun-pkg-pr-new.mjs is-bun-project <package.json>
  bun-pkg-pr-new.mjs patch-package <package.json> <core-url> <vite-plus-url>
  bun-pkg-pr-new.mjs add-core-dependency <package.json> <core-spec>
  bun-pkg-pr-new.mjs normalize-vite-paths <project-dir> <tarball-path>`);
  process.exit(2);
}

function readPackageJson(packageJsonPath) {
  const text = fs.readFileSync(packageJsonPath, 'utf8');
  return {
    indent: text.match(/\n([\t ]+)"/)?.[1] ?? '  ',
    pkg: JSON.parse(text),
  };
}

function writePackageJson(packageJsonPath, pkg, indent) {
  fs.writeFileSync(packageJsonPath, `${JSON.stringify(pkg, null, indent)}\n`);
}

function isBunProject(packageJsonPath) {
  const { pkg } = readPackageJson(packageJsonPath);
  const packageManager =
    typeof pkg.packageManager === 'string' ? pkg.packageManager.split('@')[0] : undefined;
  const devEngine = pkg.devEngines?.packageManager;
  const devEngineName = typeof devEngine === 'string' ? devEngine : devEngine?.name;
  process.exit(packageManager === 'bun' || devEngineName === 'bun' ? 0 : 1);
}

function patchPackage(packageJsonPath, coreUrl, vitePlusUrl) {
  const { pkg } = readPackageJson(packageJsonPath);
  const bundledViteVersion = pkg.bundledVersions?.vite;

  pkg.name = 'vite';
  pkg.version =
    typeof bundledViteVersion === 'string' && bundledViteVersion.length > 0
      ? bundledViteVersion
      : '8.0.0';
  pkg.dependencies = {
    ...pkg.dependencies,
    '@voidzero-dev/vite-plus-core': coreUrl,
    'vite-plus': vitePlusUrl,
  };

  writePackageJson(packageJsonPath, pkg, '  ');
}

function addCoreDependency(packageJsonPath, coreSpec) {
  const { indent, pkg } = readPackageJson(packageJsonPath);
  pkg.devDependencies ??= {};
  pkg.devDependencies['@voidzero-dev/vite-plus-core'] = coreSpec;
  writePackageJson(packageJsonPath, pkg, indent);
}

function normalizeVitePaths(projectDir, tarballPath) {
  const absoluteSpec = `file:${tarballPath}`;
  const skippedDirectories = new Set([
    '.git',
    '.output',
    'build',
    'dist',
    'node_modules',
    'vendor',
  ]);

  function rewriteValue(value, relativeSpec) {
    if (value === absoluteSpec) {
      return relativeSpec;
    }
    if (Array.isArray(value)) {
      return value.map((item) => rewriteValue(item, relativeSpec));
    }
    if (value && typeof value === 'object') {
      for (const [key, child] of Object.entries(value)) {
        value[key] = rewriteValue(child, relativeSpec);
      }
    }
    return value;
  }

  function visit(directory) {
    for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
      const entryPath = path.join(directory, entry.name);
      if (entry.isDirectory()) {
        if (!skippedDirectories.has(entry.name)) {
          visit(entryPath);
        }
        continue;
      }
      if (!entry.isFile() || entry.name !== 'package.json') {
        continue;
      }

      const text = fs.readFileSync(entryPath, 'utf8');
      if (!text.includes(absoluteSpec)) {
        continue;
      }
      const relativePath = path
        .relative(path.dirname(entryPath), tarballPath)
        .split(path.sep)
        .join('/');
      const relativeSpec = `file:${relativePath.startsWith('.') ? relativePath : `./${relativePath}`}`;
      const pkg = rewriteValue(JSON.parse(text), relativeSpec);
      const indent = text.match(/\n([\t ]+)"/)?.[1] ?? '  ';
      writePackageJson(entryPath, pkg, indent);
    }
  }

  visit(projectDir);
}

const [command, ...args] = process.argv.slice(2);

switch (command) {
  case 'is-bun-project':
    if (args.length !== 1) usage();
    isBunProject(...args);
    break;
  case 'patch-package':
    if (args.length !== 3) usage();
    patchPackage(...args);
    break;
  case 'add-core-dependency':
    if (args.length !== 2) usage();
    addCoreDependency(...args);
    break;
  case 'normalize-vite-paths':
    if (args.length !== 2) usage();
    normalizeVitePaths(...args);
    break;
  default:
    usage();
}
