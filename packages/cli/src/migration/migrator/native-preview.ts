import fs from 'node:fs';
import path from 'node:path';

import { getMajor, isRangeSubset, normalizeRange, satisfies } from 'verkit';

import { classifyNativePreviewPackConfig } from '../../../binding/index.js';
import type { WorkspaceInfoOptional } from '../../types/index.ts';
import type { CatalogDependencyResolver } from '../../utils/catalog.ts';
import { writeJsonFile } from '../../utils/json.ts';
import { detectPackageMetadata } from '../../utils/package.ts';
import { displayRelative } from '../../utils/path.ts';
import { detectConfigs } from '../detector.ts';
import type { MigrationReport } from '../report.ts';
import { createCatalogDependencyResolver } from './catalog.ts';
import { readPackageJsonIfExists, type DependencyBag } from './shared.ts';
import { getWorkspaceProjectPaths, sourceTreeMatches } from './source-scan.ts';

const PREVIEW = '@typescript/native-preview';
const TYPESCRIPT = '7.1.0-beta';
const INSTALL_FIELDS = ['devDependencies', 'dependencies', 'optionalDependencies'] as const;
type Project = { dir: string; file: string; pkg: DependencyBag };

function installField(pkg: DependencyBag, name: string) {
  return INSTALL_FIELDS.find((field) => pkg[field]?.[name] !== undefined);
}

function packGenerator(dir: string) {
  const { viteConfig, tsdownConfig } = detectConfigs(dir);
  for (const [file, standalone] of [
    [viteConfig, false],
    [tsdownConfig, true],
  ] as const) {
    if (!file) {
      continue;
    }
    const source = fs.readFileSync(path.join(dir, file), 'utf8');
    const config = file.endsWith('.json') ? `export default ${source}` : source;
    const kind = classifyNativePreviewPackConfig(config, standalone, tsdownConfig);
    if (kind === 'none') {
      return undefined;
    }
    if (kind !== 'standalone') {
      return { file: path.join(dir, file), kind };
    }
  }
  return undefined;
}

// TypeScript 7.0 already supplied the native CLI under the typescript package name.
function usesClassicCompiler(
  dir: string,
  spec?: string,
  resolveCatalog?: CatalogDependencyResolver,
) {
  if (!spec) {
    return true;
  }
  const range = spec.startsWith('catalog:') ? resolveCatalog?.(spec, 'typescript') : spec;
  if (!range || !normalizeRange(range)) {
    return undefined;
  }
  const options = { includePrerelease: true };
  if (isRangeSubset(range, '<7.0.0-0', options)) {
    return true;
  }
  if (isRangeSubset(range, '>=7.0.0-0', options)) {
    return false;
  }
  const installed = detectPackageMetadata(dir, 'typescript');
  if (installed?.name === 'typescript' && satisfies(installed.version, range, options)) {
    return getMajor(installed.version) < 7;
  }
  return undefined;
}

// The preview also ships a standalone tsgo command and an experimental JS API.
// A workspace member can use the dependency installed by its parent.
function hasOtherPreviewUses(dir: string): boolean {
  return sourceTreeMatches(
    dir,
    (source, file) => {
      const name = path.basename(file);
      if (
        ['pnpm-workspace.yaml', '.yarnrc.yml', 'pnpm-lock.yaml', 'package-lock.json'].includes(name)
      ) {
        return false;
      }
      if (source.includes(PREVIEW)) {
        return true;
      }
      if (name === 'package.json') {
        const { scripts } = JSON.parse(source) as { scripts?: Record<string, string> };
        return Object.values(scripts ?? {}).some((script) => /\btsgo\b/.test(script));
      }
      return (
        /(?:^|[\s"'`])tsgo(?:\s+-|\s*$)/m.test(source) ||
        /\b(?:spawn|execFile)(?:Sync)?\s*\(\s*['"]tsgo['"]/.test(source)
      );
    },
    {
      crossPackageBoundaries: true,
      includeExtensionless: true,
      includePackageReferences: true,
      matchesPackage: (_dir, pkg) => !!pkg.peerDependencies?.[PREVIEW],
      fileNamePattern: /\.(?:sh|ps1|cmd|bat|jsonc?|ya?ml)$|^Makefile$/,
    },
  );
}

// A shared TypeScript install can also supply a library's peer or an existing
// build script that imports the classic compiler API. TypeScript 7 drops that API.
function classicCompilerConsumer(compiler: Project, root: Project, projects: Project[]) {
  return projects.find(({ dir, pkg }) => {
    if (dir !== compiler.dir && (compiler !== root || installField(pkg, 'typescript'))) {
      return false;
    }
    const peer = pkg.peerDependencies?.typescript;
    return (
      (peer && !satisfies(TYPESCRIPT, peer)) ||
      sourceTreeMatches(dir, (source) =>
        /\b(?:from\s*|(?:import|require)\s*\(\s*)['"]typescript(?:\/lib\/typescript(?:\.js)?)?['"]/.test(
          source,
        ),
      )
    );
  });
}

/** Migrate only packages that selected the preview compiler for declaration emit. */
export function migrateNativePreviewPack(
  workspace: Pick<
    WorkspaceInfoOptional,
    'rootDir' | 'packages' | 'packageManager' | 'packageManagerVersion'
  >,
  report: MigrationReport,
): { changed: boolean; warnings: string[] } {
  const warnings: string[] = [];
  let changed = false;
  let resolveCatalog: CatalogDependencyResolver | undefined;
  const projects = getWorkspaceProjectPaths(workspace).flatMap((dir) => {
    const file = path.join(dir, 'package.json');
    const pkg = readPackageJsonIfExists(file);
    return pkg ? [{ dir, file, pkg }] : [];
  });
  const root = projects.find(({ dir }) => dir === workspace.rootDir);
  if (!root) {
    return { changed, warnings };
  }
  const rootProvider = installField(root.pkg, PREVIEW) ? root : undefined;
  const rootTypeScript =
    root.pkg[installField(root.pkg, 'typescript') ?? 'devDependencies']?.typescript;
  const candidates = new Map<Project, Set<Project>>();
  const reviewCompiler = new Set<Project>();
  const retainPreview = new Set<Project>();

  for (const project of projects) {
    const { dir, pkg } = project;
    const owner: Project | undefined =
      installField(pkg, PREVIEW) || pkg.peerDependencies?.[PREVIEW] ? project : rootProvider;
    if (!owner) {
      continue;
    }
    const pack = packGenerator(dir);
    if (!pack) {
      continue;
    }
    // A member with its own compiler or toolchain gets its own version. A member
    // that inherits both from the root must upgrade the shared compiler there.
    const compiler: Project =
      owner === project || installField(pkg, 'typescript') || installField(pkg, 'vite-plus')
        ? project
        : owner;
    const tsField = installField(compiler.pkg, 'typescript') ?? 'devDependencies';
    const spec = compiler.pkg[tsField]?.typescript;
    const effective = spec ?? rootTypeScript;
    if (effective?.startsWith('catalog:') && workspace.packageManager) {
      resolveCatalog ??= createCatalogDependencyResolver(
        workspace.rootDir,
        workspace.packageManager,
        workspace.packageManagerVersion,
      );
    }
    const classic = usesClassicCompiler(compiler.dir, effective, resolveCatalog);
    if (classic === false) {
      continue;
    }
    if (pack.kind === 'manual' || classic === undefined || owner.pkg.peerDependencies?.[PREVIEW]) {
      retainPreview.add(owner);
      reviewCompiler.add(compiler);
      warnings.push(
        `Review ${displayRelative(pack.file, workspace.rootDir)}. For native declaration emit, install typescript@${TYPESCRIPT} or set dts.tsgo.moduleUrl to a compatible compiler. Check custom tsgo.path and other uses of ${PREVIEW} before removing it.`,
      );
      continue;
    }
    const owners = candidates.get(compiler) ?? new Set<Project>();
    owners.add(owner);
    candidates.set(compiler, owners);
  }

  const upgrades = new Set<Project>();
  const removePreview = new Set<Project>();
  for (const [compiler, owners] of candidates) {
    const consumer = classicCompilerConsumer(compiler, root, projects);
    if (consumer) {
      reviewCompiler.add(compiler);
      warnings.push(
        `Review ${displayRelative(consumer.file, workspace.rootDir)}. This package needs classic TypeScript or declares a peer range that excludes ${TYPESCRIPT}. Keep its compiler and point pack.dts.tsgo.moduleUrl at a separate TypeScript 7.1+ installation.`,
      );
    }
    if (!reviewCompiler.has(compiler)) {
      upgrades.add(compiler);
    }
    for (const owner of owners) {
      (upgrades.has(compiler) ? removePreview : retainPreview).add(owner);
    }
  }
  for (const project of projects) {
    const { dir, file, pkg } = project;
    const tsField = upgrades.has(project) && (installField(pkg, 'typescript') ?? 'devDependencies');
    let remove = removePreview.has(project) && !retainPreview.has(project);
    if (remove && hasOtherPreviewUses(dir)) {
      warnings.push(
        `Kept ${PREVIEW} in ${displayRelative(file, workspace.rootDir)} for other uses of its CLI or API.`,
      );
      remove = false;
    }
    if (!tsField && !remove) {
      continue;
    }
    if (tsField) {
      const deps = (pkg[tsField] ??= {});
      report.dependencyUpgrades.push({
        name:
          project === root
            ? 'typescript'
            : `typescript (${displayRelative(dir, workspace.rootDir)})`,
        from: deps.typescript,
        to: TYPESCRIPT,
      });
      deps.typescript = TYPESCRIPT;
    }
    if (remove) {
      for (const field of INSTALL_FIELDS) {
        delete pkg[field]?.[PREVIEW];
      }
    }
    writeJsonFile(file, pkg);
    changed = true;
  }
  return { changed, warnings };
}
