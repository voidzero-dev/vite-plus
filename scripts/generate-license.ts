import fs from 'node:fs';
import path from 'node:path';

interface GenerateLicenseFileOptions {
  title: string;
  packageName: string;
  outputPath: string;
  coreLicensePath: string;
  bundledPaths: string[];
  resolveFrom?: string[];
  extraPackages?: PackageReference[];
  excludePackageNames?: string[];
}

interface PackageReference {
  packageDir: string;
  licensePath?: string;
}

interface DependencyInfo {
  name: string;
  license?: string;
  licenseText?: string;
  author?: string;
  contributors: string[];
  maintainers: string[];
  repository?: string;
}

interface FormattedDependencyInfo {
  license?: string;
  names?: string;
  repository?: string;
}

const LICENSE_FILE_NAMES = [
  'LICENSE',
  'LICENSE.md',
  'LICENSE.txt',
  'LICENCE',
  'LICENCE.md',
  'LICENCE.txt',
  'license',
  'license.md',
  'license.txt',
  'COPYING',
] as const;

const TEXT_FILE_SUFFIXES = [
  '.js',
  '.mjs',
  '.cjs',
  '.ts',
  '.mts',
  '.cts',
  '.d.ts',
  '.d.mts',
  '.d.cts',
  '.css',
  '.html',
] as const;

const NODE_MODULES_REGION_RE = /\/\/#region\s+([^\r\n]*node_modules[^\r\n]*)/g;
const pnpmStoreResolutionCache = new Map<string, Map<string, string | null>>();

export function generateLicenseFile(options: GenerateLicenseFileOptions) {
  const packageRefs = new Map<string, PackageReference>();
  const resolveFrom = options.resolveFrom ?? [process.cwd()];
  const excludedPackageNames = new Set(options.excludePackageNames ?? []);

  for (const packageName of collectBundledPackageNames(options.bundledPaths)) {
    if (excludedPackageNames.has(packageName)) {
      continue;
    }

    const packageDir = resolvePackageDir(packageName, resolveFrom);
    if (!packageDir) {
      throw new Error(`Could not resolve bundled package "${packageName}" for license generation`);
    }

    addPackageReference(packageRefs, { packageDir });
  }

  for (const extraPackage of options.extraPackages ?? []) {
    const packageInfo = readPackageJson(extraPackage.packageDir);
    if (!packageInfo) {
      continue;
    }

    const packageName = typeof packageInfo.name === 'string' ? packageInfo.name : undefined;
    if (packageName && excludedPackageNames.has(packageName)) {
      continue;
    }

    addPackageReference(packageRefs, extraPackage);
  }

  const dependencies = Array.from(packageRefs.values())
    .map((packageRef) => readDependencyInfo(packageRef))
    .filter((dependency): dependency is DependencyInfo => dependency !== null);

  const deps = sortDependencies(dependencies);
  const licenses = sortLicenses(
    new Set(
      deps
        .map((dependency) => dependency.license)
        .filter((license): license is string => typeof license === 'string'),
    ),
  );
  const coreLicense = fs.readFileSync(options.coreLicensePath, 'utf-8');

  let dependencyLicenseTexts = '';
  for (let i = 0; i < deps.length; i++) {
    const licenseText = deps[i].licenseText;
    const sameDeps = [deps[i]];
    if (licenseText) {
      for (let j = i + 1; j < deps.length; j++) {
        if (licenseText === deps[j].licenseText) {
          sameDeps.push(...deps.splice(j, 1));
          j--;
        }
      }
    }

    let text = `## ${sameDeps.map((dependency) => dependency.name).join(', ')}\n`;
    const depInfos = sameDeps.map((dependency) => getDependencyInformation(dependency));

    if (
      depInfos.length > 1 &&
      depInfos.every(
        (info) => info.license === depInfos[0].license && info.names === depInfos[0].names,
      )
    ) {
      const { license, names } = depInfos[0];
      const repositoryText = depInfos
        .map((info) => info.repository)
        .filter(Boolean)
        .join(', ');

      if (license) {
        text += `License: ${license}\n`;
      }
      if (names) {
        text += `By: ${names}\n`;
      }
      if (repositoryText) {
        text += `Repositories: ${repositoryText}\n`;
      }
    } else {
      for (let j = 0; j < depInfos.length; j++) {
        const { license, names, repository } = depInfos[j];
        if (license) {
          text += `License: ${license}\n`;
        }
        if (names) {
          text += `By: ${names}\n`;
        }
        if (repository) {
          text += `Repository: ${repository}\n`;
        }
        if (j !== depInfos.length - 1) {
          text += '\n';
        }
      }
    }

    if (licenseText) {
      text +=
        '\n' +
        licenseText
          .trim()
          .replace(/\r\n|\r/g, '\n')
          .split('\n')
          .map((line) => `> ${line}`)
          .join('\n') +
        '\n';
    }

    if (i !== deps.length - 1) {
      text += '\n---------------------------------------\n\n';
    }

    dependencyLicenseTexts += text;
  }

  const licenseFileContent =
    `# ${options.title}\n` +
    `${options.packageName} is released under the MIT license:\n\n` +
    coreLicense +
    `\n` +
    `# Licenses of bundled dependencies\n` +
    `The published ${options.packageName} artifact additionally contains code with the following licenses:\n` +
    `${licenses.join(', ')}\n\n` +
    `# Bundled dependencies:\n` +
    dependencyLicenseTexts;

  let existingContent: string | undefined;
  try {
    existingContent = fs.readFileSync(options.outputPath, 'utf-8');
  } catch {
    // File does not exist yet.
  }

  if (existingContent !== licenseFileContent) {
    fs.writeFileSync(options.outputPath, licenseFileContent);
    const outputFileName = path.basename(options.outputPath);
    console.error(
      `\x1b[33m\n${outputFileName} updated. You should commit the updated file.\n\x1b[0m`,
    );
  }
}

function collectBundledPackageNames(bundledPaths: string[]): Set<string> {
  const packageNames = new Set<string>();

  for (const bundledPath of bundledPaths) {
    if (!fs.existsSync(bundledPath)) {
      continue;
    }

    for (const filePath of walkTextFiles(bundledPath)) {
      const content = fs.readFileSync(filePath, 'utf-8');
      for (const match of content.matchAll(NODE_MODULES_REGION_RE)) {
        const packageName = extractPackageName(match[1]);
        if (packageName) {
          packageNames.add(packageName);
        }
      }
    }
  }

  return packageNames;
}

function* walkTextFiles(targetPath: string): Generator<string> {
  const stats = fs.statSync(targetPath);

  if (stats.isFile()) {
    if (isTextFile(targetPath)) {
      yield targetPath;
    }
    return;
  }

  if (!stats.isDirectory()) {
    return;
  }

  for (const entry of fs.readdirSync(targetPath, { withFileTypes: true })) {
    const entryPath = path.join(targetPath, entry.name);
    if (entry.isDirectory()) {
      yield* walkTextFiles(entryPath);
      continue;
    }

    if (entry.isFile() && isTextFile(entryPath)) {
      yield entryPath;
    }
  }
}

function isTextFile(filePath: string): boolean {
  return TEXT_FILE_SUFFIXES.some((suffix) => filePath.endsWith(suffix));
}

function extractPackageName(regionPath: string): string | undefined {
  const normalized = regionPath.replaceAll('\\', '/');
  const nodeModulesIndex = normalized.lastIndexOf('/node_modules/');
  if (nodeModulesIndex === -1) {
    return undefined;
  }

  const afterNodeModules = normalized.slice(nodeModulesIndex + '/node_modules/'.length);
  if (afterNodeModules.startsWith('@')) {
    const parts = afterNodeModules.split('/');
    if (parts.length < 2) {
      return undefined;
    }
    return `${parts[0]}/${parts[1]}`;
  }

  const packageName = afterNodeModules.split('/')[0];
  if (!packageName || packageName.startsWith('.')) {
    return undefined;
  }
  return packageName;
}

function resolvePackageDir(packageName: string, resolveFrom: string[]): string | undefined {
  const seen = new Set<string>();

  for (const startPath of resolveFrom) {
    let currentDir = path.resolve(startPath);
    while (true) {
      if (seen.has(currentDir)) {
        break;
      }
      seen.add(currentDir);

      const packageDir = path.join(currentDir, 'node_modules', packageName);
      const packageJsonPath = path.join(packageDir, 'package.json');
      if (fs.existsSync(packageJsonPath)) {
        return packageDir;
      }

      const pnpmStoreDir = path.join(currentDir, 'node_modules', '.pnpm');
      const pnpmPackageDir = resolvePackageDirInPnpmStore(pnpmStoreDir, packageName);
      if (pnpmPackageDir) {
        return pnpmPackageDir;
      }

      const parentDir = path.dirname(currentDir);
      if (parentDir === currentDir) {
        break;
      }
      currentDir = parentDir;
    }
  }

  return undefined;
}

function resolvePackageDirInPnpmStore(
  pnpmStoreDir: string,
  packageName: string,
): string | undefined {
  if (!fs.existsSync(pnpmStoreDir)) {
    return undefined;
  }

  let storeCache = pnpmStoreResolutionCache.get(pnpmStoreDir);
  if (!storeCache) {
    storeCache = new Map<string, string | null>();
    pnpmStoreResolutionCache.set(pnpmStoreDir, storeCache);
  }

  const cachedPackageDir = storeCache.get(packageName);
  if (cachedPackageDir !== undefined) {
    return cachedPackageDir ?? undefined;
  }

  for (const entry of fs.readdirSync(pnpmStoreDir, { withFileTypes: true })) {
    if (!entry.isDirectory()) {
      continue;
    }

    const packageDir = path.join(pnpmStoreDir, entry.name, 'node_modules', packageName);
    const packageJsonPath = path.join(packageDir, 'package.json');
    if (fs.existsSync(packageJsonPath)) {
      storeCache.set(packageName, packageDir);
      return packageDir;
    }
  }

  storeCache.set(packageName, null);
  return undefined;
}

function addPackageReference(
  packageRefs: Map<string, PackageReference>,
  packageRef: PackageReference,
) {
  const packageJsonPath = path.join(packageRef.packageDir, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return;
  }

  const normalizedDir = fs.realpathSync(packageRef.packageDir);
  const existing = packageRefs.get(normalizedDir);
  if (!existing) {
    packageRefs.set(normalizedDir, packageRef);
    return;
  }

  if (!existing.licensePath && packageRef.licensePath) {
    packageRefs.set(normalizedDir, packageRef);
  }
}

function readDependencyInfo(packageRef: PackageReference): DependencyInfo | null {
  const pkgJson = readPackageJson(packageRef.packageDir);
  if (!pkgJson) {
    return null;
  }

  const name =
    typeof pkgJson.name === 'string' ? pkgJson.name : path.basename(packageRef.packageDir);
  const dependency: DependencyInfo = {
    name,
    license: typeof pkgJson.license === 'string' ? pkgJson.license : undefined,
    contributors: [],
    maintainers: [],
  };

  if (pkgJson.author) {
    dependency.author =
      typeof pkgJson.author === 'string'
        ? pkgJson.author
        : (pkgJson.author as Record<string, string>).name;
  }

  if (Array.isArray(pkgJson.contributors)) {
    for (const contributor of pkgJson.contributors) {
      const name = typeof contributor === 'string' ? contributor : contributor?.name;
      if (name) {
        dependency.contributors.push(name);
      }
    }
  }

  if (Array.isArray(pkgJson.maintainers)) {
    for (const maintainer of pkgJson.maintainers) {
      const name = typeof maintainer === 'string' ? maintainer : maintainer?.name;
      if (name) {
        dependency.maintainers.push(name);
      }
    }
  }

  if (pkgJson.repository) {
    const repositoryUrl =
      typeof pkgJson.repository === 'string'
        ? pkgJson.repository
        : (pkgJson.repository as Record<string, string>).url;
    if (repositoryUrl) {
      dependency.repository = normalizeGitUrl(repositoryUrl);
    }
  }

  dependency.licenseText = readLicenseText(packageRef);
  return dependency;
}

function readPackageJson(packageDir: string): Record<string, unknown> | null {
  const packageJsonPath = path.join(packageDir, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return null;
  }

  try {
    return JSON.parse(fs.readFileSync(packageJsonPath, 'utf-8'));
  } catch {
    return null;
  }
}

function readLicenseText(packageRef: PackageReference): string | undefined {
  if (packageRef.licensePath && fs.existsSync(packageRef.licensePath)) {
    return fs.readFileSync(packageRef.licensePath, 'utf-8');
  }

  for (const licenseFileName of LICENSE_FILE_NAMES) {
    const licensePath = path.join(packageRef.packageDir, licenseFileName);
    if (fs.existsSync(licensePath)) {
      return fs.readFileSync(licensePath, 'utf-8');
    }
  }

  return undefined;
}

function sortDependencies(dependencies: DependencyInfo[]): DependencyInfo[] {
  return dependencies.toSorted((a, b) => a.name.localeCompare(b.name));
}

function sortLicenses(licenses: Set<string>): string[] {
  const withParenthesis: string[] = [];
  const withoutParenthesis: string[] = [];

  for (const license of licenses) {
    if (license.startsWith('(')) {
      withParenthesis.push(license);
    } else {
      withoutParenthesis.push(license);
    }
  }

  withParenthesis.sort();
  withoutParenthesis.sort();

  return [...withoutParenthesis, ...withParenthesis];
}

function getDependencyInformation(dependency: DependencyInfo): FormattedDependencyInfo {
  const info: FormattedDependencyInfo = {};

  if (dependency.license) {
    info.license = dependency.license;
  }

  const names = new Set<string>();
  if (dependency.author) {
    names.add(dependency.author);
  }
  for (const name of dependency.contributors) {
    names.add(name);
  }
  for (const name of dependency.maintainers) {
    names.add(name);
  }

  if (names.size > 0) {
    info.names = Array.from(names).join(', ');
  }

  if (dependency.repository) {
    info.repository = dependency.repository;
  }

  return info;
}

function normalizeGitUrl(url: string): string {
  url = url
    .replace(/^git\+/, '')
    .replace(/\.git$/, '')
    .replace(/(^|\/)[^/]+?@/, '$1')
    .replace(/(\.[^.]+?):/, '$1/')
    .replace(/^git:\/\//, 'https://')
    .replace(/^ssh:\/\//, 'https://');

  if (url.startsWith('github:')) {
    return `https://github.com/${url.slice(7)}`;
  }
  if (url.startsWith('gitlab:')) {
    return `https://gitlab.com/${url.slice(7)}`;
  }
  if (url.startsWith('bitbucket:')) {
    return `https://bitbucket.org/${url.slice(10)}`;
  }
  if (!url.includes(':') && url.split('/').length === 2) {
    return `https://github.com/${url}`;
  }
  return url.includes('://') ? url : `https://${url}`;
}
