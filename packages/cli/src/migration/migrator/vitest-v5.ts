import fs from 'node:fs';
import path from 'node:path';

import { applyEdits, findNodeAtLocation, parse as parseJsonc, parseTree } from 'jsonc-parser';
import semver from 'semver';
import { isScalar, parseDocument, visit } from 'yaml';

import cliPackage from '../../../package.json' with { type: 'json' };
import { PackageManager, type WorkspaceInfoOptional } from '../../types/index.ts';
import { detectPackageMetadata } from '../../utils/package.ts';
import { createCatalogDependencyResolver, usesWebdriverioProvider } from '../migrator.ts';
import type { RewriteResult, SourceOptions, VitestV5Finding } from '../vitest-v5/ast.ts';
import { migrateVitestV5Command } from '../vitest-v5/commands.ts';
import {
  findVitestV5ConfigFiles,
  findVitestV5MergedConfigFiles,
  migrateVitestV5Config,
} from '../vitest-v5/config.ts';
import { vitestV5Documentation } from '../vitest-v5/documentation.ts';
import { lockedVitestVersion } from '../vitest-v5/lockfile.ts';
import {
  findVitestV5ConfigEntries,
  resolveVitestV5TestModes,
  type VitestV5TestMode,
} from '../vitest-v5/scopes.ts';
import { hasVitestV5SourceUsage, migrateVitestV5Source } from '../vitest-v5/source.ts';

const SKIP_DIRS = new Set([
  'node_modules',
  '.git',
  '.vite',
  '.vitest',
  '.vite-plus',
  '.cache',
  'dist',
  'build',
  'out',
  'coverage',
  '.next',
  '.nuxt',
  '.svelte-kit',
]);
const CODE_FILE = /\.[cm]?[jt]sx?$/;
const VITEST_SIGNAL =
  /(?:['"](?:vitest(?:\/[^'"]*)?|@vitest\/[^'"]+|vite-plus\/test[^'"]*)['"]|\bvp\s+test\b|\bvitest\s+(?:run|list|bench)|\bimport\s*\.\s*meta\s*\.\s*vitest\b)/;
const VITEST_COMMAND = /\b(?:vp\s+test|vitest)(?:\s|$)/;
const BROWSER_SIGNAL =
  /@vitest\/browser|(?:vitest|vite-plus\/test)\/browser|vitest-browser-|browser\s*:\s*\{|--browser(?:[.=\s'"]|$)/;
const LEGACY_RUNNER = '@voidzero-dev/vite-plus-test';

interface ProjectPlan {
  directory: string;
  sourceVersion?: string;
  active: boolean;
  options: SourceOptions;
  configFiles: Set<string>;
  // Retain deferred findings only through the current invocation's final scan.
  pendingSourceReviews?: Record<string, string[]>;
}

const DEFERRED_SOURCE_REVIEWS = new Set(['resolve-config', 'overlapping-edits', 'unsafe-syntax']);
interface FileChange {
  file: string;
  before: string;
  after: string;
}
export interface VitestV5MigrationPlan {
  rootDir: string;
  packageManager?: PackageManager;
  projects: ProjectPlan[];
  findings: VitestV5Finding[];
  changes: FileChange[];
  inputs: ReadonlyMap<string, string | null>;
}

type MigrationWorkspace = Pick<WorkspaceInfoOptional, 'rootDir' | 'packageManager'> &
  Partial<Pick<WorkspaceInfoOptional, 'packages'>>;

function workspaceFromPlan(plan: VitestV5MigrationPlan): MigrationWorkspace {
  return {
    rootDir: plan.rootDir,
    packageManager: plan.packageManager,
    packages: plan.projects.slice(1).map((project) => ({
      name: '',
      path: path.relative(plan.rootDir, project.directory),
    })),
  };
}

function readJson(file: string): Record<string, unknown> {
  return parseJson(fs.readFileSync(file, 'utf8'));
}
function parseJson(source: string) {
  return JSON.parse(source.replace(/^\uFEFF/, ''));
}
function record(value: unknown): Record<string, unknown> | undefined {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : undefined;
}
function dependency(pkg: Record<string, unknown>, name: string): string | undefined {
  for (const field of [
    'dependencies',
    'devDependencies',
    'optionalDependencies',
    'peerDependencies',
  ]) {
    const spec = record(pkg[field])?.[name];
    if (typeof spec === 'string') {
      return spec;
    }
  }
  return undefined;
}

function filesInProject(directory: string): string[] {
  const files: string[] = [];
  const walk = (current: string) => {
    if (current !== directory && fs.existsSync(path.join(current, 'package.json'))) {
      return;
    }
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const file = path.join(current, entry.name);
      // Never follow symlinks into another package or outside the workspace.
      if (entry.isDirectory() && !SKIP_DIRS.has(entry.name)) {
        walk(file);
      } else if (
        entry.isFile() &&
        (CODE_FILE.test(file) ||
          /\.(?:json|ya?ml|sh)$/.test(file) ||
          /^Dockerfile(?:\.|$)|^Containerfile(?:\.|$)/.test(entry.name) ||
          ['.node-version', '.nvmrc', '.gitignore', 'yarn.lock', 'bun.lock'].includes(entry.name))
      ) {
        files.push(file);
      }
    }
  };
  walk(directory);
  return files.toSorted();
}

function finding(
  file: string,
  code: string,
  message: string,
  severity: VitestV5Finding['severity'] = 'review',
  source = '',
  offset = 0,
): VitestV5Finding {
  const prefix = source.slice(0, offset);
  return {
    file,
    line: prefix.split('\n').length,
    column: offset - prefix.lastIndexOf('\n'),
    code,
    severity,
    message,
  };
}

/** Follow this project's dependency edges, never an unrelated transitive runner. */
function lockedSourceVersion(
  directory: string,
  root: string,
  pkg: Record<string, unknown>,
  manager?: PackageManager,
  expectedRunnerPackage?: string,
): string | undefined {
  if (manager !== undefined && manager !== PackageManager.pnpm) {
    const spec = dependency(pkg, 'vitest');
    return spec && !expectedRunnerPackage
      ? lockedVitestVersion(root, directory, manager, spec)
      : undefined;
  }
  const lockfile = path.join(root, 'pnpm-lock.yaml');
  if (!fs.existsSync(lockfile)) {
    return undefined;
  }
  try {
    const document = parseDocument(fs.readFileSync(lockfile, 'utf8'));
    if (document.errors.length) {
      return undefined;
    }
    const lock = record(document.toJS());
    const importer = record(
      record(lock?.importers)?.[path.relative(root, directory).replaceAll('\\', '/') || '.'],
    );
    if (!importer) {
      return undefined;
    }
    const name = dependency(pkg, 'vitest') ? 'vitest' : 'vite-plus';
    const spec = dependency(pkg, name);
    if (!spec) {
      return undefined;
    }
    const entry = ['dependencies', 'devDependencies', 'optionalDependencies']
      .map((field) => record(importer[field])?.[name])
      .find((value) => value !== undefined);
    const metadata = record(entry);
    if (typeof metadata?.specifier === 'string' && metadata.specifier !== spec) {
      return undefined;
    }
    const follow = (name: string, value: unknown, depth = 0): string | undefined => {
      const reference = typeof value === 'string' ? value : record(value)?.version;
      if (typeof reference !== 'string' || depth > 3) {
        return undefined;
      }
      if (
        name === 'vitest' &&
        expectedRunnerPackage &&
        !reference.startsWith(`${expectedRunnerPackage}@`)
      ) {
        return undefined;
      }
      if (reference.startsWith(`${LEGACY_RUNNER}@`)) {
        return follow(LEGACY_RUNNER, reference.slice(LEGACY_RUNNER.length + 1), depth + 1);
      }
      const version = reference.split('(')[0];
      if (!semver.valid(version)) {
        return undefined;
      }
      if (name === 'vitest') {
        return version;
      }
      if (name === LEGACY_RUNNER) {
        // The wrapper has a Vite+ version; its exact UI peer records the
        // upstream runner version, including when the optional UI is absent.
        const legacy = record(record(lock?.packages)?.[`${name}@${version}`]);
        const runnerVersion = record(legacy?.peerDependencies)?.['@vitest/ui'];
        return typeof runnerVersion === 'string'
          ? (semver.valid(runnerVersion) ?? undefined)
          : undefined;
      }
      const snapshot = record(record(lock?.snapshots)?.[`${name}@${reference}`]);
      const dependencies = record(snapshot?.dependencies);
      return (
        follow('vitest', dependencies?.vitest, depth + 1) ??
        follow(LEGACY_RUNNER, dependencies?.[LEGACY_RUNNER], depth + 1)
      );
    };
    return follow(name, entry);
  } catch {
    // An unreadable or unsupported lockfile is not evidence of a v4 runner.
    return undefined;
  }
}

function unambiguousRunnerVersion(range: string): string | undefined {
  if (
    !semver.validRange(range) ||
    (!semver.subset(range, '<5.0.0-0', { includePrerelease: true }) &&
      !semver.subset(range, '>=5.0.0-0', { includePrerelease: true }))
  ) {
    // A range that crosses the v4/v5 boundary cannot establish which defaults
    // the project used. Require its installed runner or an exact lockfile edge.
    return undefined;
  }
  return semver.minVersion(range)?.version;
}

function installedSourceVersion(
  directory: string,
  name: string,
  expectedPackage?: string,
  expectedRange?: string,
): string | undefined {
  const installed = detectPackageMetadata(directory, name);
  if (!installed || (expectedPackage && installed.name !== expectedPackage)) {
    return undefined;
  }
  if (
    expectedRange &&
    semver.validRange(expectedRange) &&
    !semver.satisfies(installed.version, expectedRange, { includePrerelease: true })
  ) {
    return undefined;
  }
  if (installed.name === 'vitest') {
    return semver.valid(installed.version) ?? undefined;
  }
  const pkg = readJson(path.join(installed.path, 'package.json'));
  if (installed.name === LEGACY_RUNNER) {
    const version = record(pkg.peerDependencies)?.['@vitest/ui'];
    return typeof version === 'string' ? (semver.valid(version) ?? undefined) : undefined;
  }
  if (installed.name !== 'vite-plus') {
    return undefined;
  }
  const bundled = dependency(pkg, 'vitest');
  if (bundled?.startsWith('catalog:')) {
    // Linked workspace builds retain catalog references in their manifest.
    // Follow that build's installed runner, not the migrating project's catalog
    // or the CLI version: repeat runs must identify v5 without saved metadata.
    return installedSourceVersion(installed.path, 'vitest');
  }
  if (bundled && semver.validRange(bundled)) {
    const version = installedSourceVersion(installed.path, 'vitest');
    return version && semver.satisfies(version, bundled, { includePrerelease: true })
      ? version
      : unambiguousRunnerVersion(bundled);
  }
  return dependency(pkg, LEGACY_RUNNER)
    ? installedSourceVersion(installed.path, LEGACY_RUNNER)
    : undefined;
}

function sourceVersion(
  directory: string,
  root: string,
  pkg: Record<string, unknown>,
  manager?: PackageManager,
): string | undefined {
  const catalog = createCatalogDependencyResolver(root, manager ?? PackageManager.pnpm);
  const vitePlus = dependency(pkg, 'vite-plus');
  const vitePlusRange = vitePlus?.startsWith('catalog:')
    ? catalog?.(vitePlus, 'vite-plus')
    : vitePlus;
  let spec = dependency(pkg, 'vitest');
  if (spec?.startsWith('catalog:')) {
    spec = catalog?.(spec, 'vitest');
  }
  const legacyAlias = spec?.startsWith(`npm:${LEGACY_RUNNER}@`);
  if (spec?.startsWith('npm:vitest@')) {
    spec = spec.slice('npm:vitest@'.length);
  }
  const hasOwnVitest = ['dependencies', 'devDependencies', 'optionalDependencies'].some(
    (field) => typeof record(pkg[field])?.vitest === 'string',
  );
  // A retained peer range describes the library's consumers, not the runner
  // used by its installed Vite+ toolchain. Prefer that runner on stateless
  // reruns; an explicit runtime/dev Vitest dependency still takes precedence.
  if (!hasOwnVitest && vitePlus) {
    const bundled = installedSourceVersion(directory, 'vite-plus', 'vite-plus', vitePlusRange);
    if (bundled) {
      return bundled;
    }
  }
  // A stale install or catalog lock entry can still resolve upstream Vitest
  // under this name. Its version is not evidence for the declared wrapper.
  const expectedRunnerPackage = legacyAlias ? LEGACY_RUNNER : undefined;
  const locked = lockedSourceVersion(directory, root, pkg, manager, expectedRunnerPackage);
  if (
    locked &&
    (!spec ||
      legacyAlias ||
      !semver.validRange(spec) ||
      semver.satisfies(locked, spec, { includePrerelease: true }))
  ) {
    return locked;
  }
  const installed = installedSourceVersion(directory, 'vitest', expectedRunnerPackage);
  if (
    installed &&
    (!spec || legacyAlias || semver.satisfies(installed, spec, { includePrerelease: true }))
  ) {
    return installed;
  }
  if (legacyAlias) {
    // This removed wrapper only shipped the v4 runner. Prefer its exact
    // version from the original install or lockfile when one is available.
    return '4.0.0';
  }
  if (spec && semver.validRange(spec)) {
    return unambiguousRunnerVersion(spec);
  }
  if (dependency(pkg, 'vite-plus')) {
    const bundled = installedSourceVersion(directory, 'vite-plus', 'vite-plus', vitePlusRange);
    if (bundled) {
      return bundled;
    }
  }
  // A workspace member commonly uses the root's test command and dependencies.
  if (directory !== root) {
    return sourceVersion(root, root, readJson(path.join(root, 'package.json')), manager);
  }
  return undefined;
}

function checkNodeRange(
  file: string,
  value: string,
  label: string,
  findings: VitestV5Finding[],
  publicContract = false,
): string | undefined {
  // At the v5 upgrade baseline, both the latest LTS and Current Node releases
  // meet our minimum versions; future majors are covered by >=26. These
  // forward-moving runtime aliases do not need a numeric pin. Named/relative
  // LTS aliases and public engines ranges still need the checks below.
  if (!publicContract && ['lts/*', 'lts', 'latest', 'current', 'node', 'stable'].includes(value)) {
    return undefined;
  }
  const range = semver.validRange(value);
  if (publicContract && range) {
    // A public engine contract is not a runtime pin. A supported minimum
    // is sufficient; open ranges need not exclude every unsupported major.
    const minimum = semver.minVersion(range);
    if (minimum) {
      // A whole-major range such as 24.x also permits a supported release.
      // Do not mistake its implicit 24.0.0 minimum for an exact runtime pin.
      const majorRange = `${minimum.major}.x`;
      if (
        semver.satisfies(minimum, cliPackage.engines.node) ||
        (semver.subset(majorRange, range) && semver.intersects(majorRange, cliPackage.engines.node))
      ) {
        return undefined;
      }
    }
  }
  let message = `Resolve ${label} (${value}) and select Node ${cliPackage.engines.node}.`;
  if (range && !publicContract && !semver.intersects(range, cliPackage.engines.node)) {
    const current = semver.minVersion(range);
    const upgrade =
      current &&
      new semver.Range(cliPackage.engines.node).set
        .map((comparators) => semver.minVersion(comparators.map(({ value }) => value).join(' ')))
        .filter((version): version is semver.SemVer => version !== null)
        .toSorted(semver.compare)
        .find((version) => semver.gte(version, current));
    if (upgrade) {
      // Choose the first supported minimum at or above the requested version.
      // For example, 20 -> 22.18.0, 24.10 -> 24.11.0, and 25 -> 26.0.0.
      return upgrade.version;
    }
  } else if (range) {
    if (
      semver.subset(range, cliPackage.engines.node) ||
      (!publicContract && !/[<>|*]/.test(value))
    ) {
      return undefined;
    }
    message = `${label} (${value}) includes unsupported test runtimes. Pin the test/CI runtime to Node ${cliPackage.engines.node}; keep the library's public engine contract separate.`;
  }
  findings.push(finding(file, 'node-runtime', message));
  return undefined;
}

function migrateNode(file: string, source: string, findings: VitestV5Finding[]): string {
  // Limit compatibility checks to project runtime declarations. Inferring
  // runtimes from CI workflows, containers, or other files is out of scope.
  const base = path.basename(file);
  if (base === '.node-version' || base === '.nvmrc') {
    const value = source.trim();
    const upgrade = checkNodeRange(file, value.replace(/^v/, ''), base, findings);
    if (upgrade) {
      const start = source.indexOf(value);
      return (
        source.slice(0, start) +
        (value.startsWith('v') ? 'v' : '') +
        upgrade +
        source.slice(start + value.length)
      );
    }
  }
  if (base === 'package.json') {
    const pkg = parseJson(source);
    const tree = parseTree(source.replace(/^\uFEFF/, ' '));
    const edits: Array<{ offset: number; length: number; content: string }> = [];
    const checkRuntime = (value: string, label: string, keys: Array<string | number>) => {
      const upgrade = checkNodeRange(file, value, label, findings);
      const node = tree && findNodeAtLocation(tree, keys);
      if (upgrade && node) {
        edits.push({ offset: node.offset, length: node.length, content: JSON.stringify(upgrade) });
      }
    };
    if (typeof pkg.engines?.node === 'string') {
      checkNodeRange(file, pkg.engines.node, 'engines.node', findings, true);
    }
    const runtime = pkg.devEngines?.runtime;
    for (const [index, entry] of (Array.isArray(runtime) ? runtime : [runtime]).entries()) {
      if (entry?.name === 'node' && typeof entry.version === 'string') {
        checkRuntime(entry.version, 'devEngines.runtime', [
          'devEngines',
          'runtime',
          ...(Array.isArray(runtime) ? [index] : []),
          'version',
        ]);
      }
    }
    // The existing setup migration uses Volta only when neither pin file
    // exists. Check that future .node-version before any project edits.
    if (
      typeof pkg.volta?.node === 'string' &&
      !['.node-version', '.nvmrc'].some((name) =>
        fs.existsSync(path.join(path.dirname(file), name)),
      )
    ) {
      checkRuntime(pkg.volta.node, 'volta.node', ['volta', 'node']);
    }
    return applyEdits(source, edits);
  }
  return source;
}

function scanAndRewriteFile(
  file: string,
  source: string,
  project: ProjectPlan,
  testMode: VitestV5TestMode,
  mergedConfig: boolean,
): RewriteResult {
  const findings: VitestV5Finding[] = [];
  if (!project.active) {
    return { content: migrateNode(file, source, findings), findings };
  }
  let content = source;
  if (
    CODE_FILE.test(file) &&
    (testMode.benchmark ||
      testMode.globals ||
      testMode.reviewGlobals ||
      VITEST_SIGNAL.test(source) ||
      /\.(?:test|spec)\./.test(file) ||
      project.configFiles.has(file))
  ) {
    try {
      // Canonical and symbol-specific rules run before the native generic import
      // pass. Config and source edits are separate AST passes to avoid overlap.
      const sourceResult = migrateVitestV5Source(file, content, {
        ...project.options,
        ...testMode,
      });
      content = sourceResult.content;
      findings.push(...sourceResult.findings);
      const pending =
        project.pendingSourceReviews?.[
          path.relative(project.directory, file).replaceAll('\\', '/')
        ];
      if (!project.options.preserveV4 && pending?.length) {
        // Re-evaluate this invocation's deferred v4 findings after its edits.
        // A later invocation starts from the resolved runner version instead.
        const review = migrateVitestV5Source(file, content, {
          ...project.options,
          ...testMode,
          preserveV4: true,
        });
        findings.push(...review.findings.filter(({ code }) => pending.includes(code)));
      }
      if (project.configFiles.has(file)) {
        const configResult = migrateVitestV5Config(file, content, project.options, mergedConfig);
        content = configResult.content;
        findings.push(...configResult.findings);
      }
    } catch (error) {
      content = source;
      findings.push(
        finding(
          file,
          'source-parse',
          `Parse and review this file before migration: ${error instanceof Error ? error.message : String(error)}`,
        ),
      );
      if (
        /['"](?:@vitest\/(?:runner|expect)|(?:vitest|vite-plus\/test)\/(?:runners|suite|internal\/module-runner))/.test(
          source,
        )
      ) {
        findings.push(
          finding(
            file,
            'removed-api',
            'This unparsed file references removed APIs. Resolve it before changing dependencies.',
            'block',
          ),
        );
      }
    }
  } else if (path.basename(file) === 'package.json') {
    const pkg = parseJson(source);
    // Replace the BOM with one space for offset lookup, but retain it in the
    // file. Edit only changed script values, preserving all other JSON trivia.
    const tree = parseTree(source.replace(/^\uFEFF/, ' '));
    const edits = [];
    for (const [name, command] of Object.entries(record(pkg.scripts) ?? {})) {
      if (typeof command !== 'string') {
        continue;
      }
      const node = tree && findNodeAtLocation(tree, ['scripts', name]);
      if (!node) {
        continue;
      }
      const result = migrateVitestV5Command(
        file,
        command,
        project.options.preserveV4,
        source.slice(0, node.offset).split('\n').length,
        project.options.preserveV4 || project.options.reviewV4,
      );
      findings.push(...result.findings);
      if (result.content !== command) {
        edits.push({
          offset: node.offset,
          length: node.length,
          content: JSON.stringify(result.content),
        });
      }
    }
    content = applyEdits(source, edits);
  } else if (/^tsconfig(?:\..+)?\.json$/.test(path.basename(file))) {
    const types = record(record(parseJsonc(source))?.compilerOptions)?.types;
    if (
      Array.isArray(types) &&
      types.includes('@testing-library/jest-dom') &&
      !types.includes('@testing-library/jest-dom/vitest')
    ) {
      findings.push(
        finding(
          file,
          'jest-dom-types',
          'If this config checks Vitest tests, load @testing-library/jest-dom/vitest in compilerOptions.types or an included TypeScript setup file. The root jest-dom type entry augments Jest, not Vitest v5.',
        ),
      );
    }
  } else if (/\.ya?ml$/.test(file)) {
    const document = parseDocument(source);
    let changed = false;
    visit(document, {
      Pair(_key, pair) {
        if (
          !isScalar(pair.key) ||
          !['run', 'script', 'command'].includes(String(pair.key.value)) ||
          !isScalar(pair.value) ||
          typeof pair.value.value !== 'string'
        ) {
          return;
        }
        const value = pair.value.value;
        // A multiline shell block can contain functions or continued commands;
        // report it as one compound command, not independent argv lines.
        const result = migrateVitestV5Command(
          file,
          value.includes('\n') ? `(${value})` : value,
          project.options.preserveV4,
          source.slice(0, pair.key.range?.[0] ?? 0).split('\n').length,
          project.options.preserveV4 || project.options.reviewV4,
        );
        findings.push(...result.findings);
        if (!value.includes('\n') && result.content !== value) {
          pair.value.value = result.content;
          changed = true;
        }
      },
    });
    if (changed) {
      content = document.toString();
    }
  } else if (file.endsWith('.sh')) {
    findings.push(
      ...migrateVitestV5Command(
        file,
        `(${source})`,
        project.options.preserveV4,
        1,
        project.options.preserveV4 || project.options.reviewV4,
      ).findings,
    );
  }
  // Apply Node value edits after script rewriting so manifest offsets refer
  // to the current text and neither pass can overwrite the other's changes.
  return { content: migrateNode(file, content, findings), findings };
}

/** Read-only preflight. Call before package-manager conversion, installs,
 * catalog updates, or generic import rewrites can erase the source version. */
export function planVitestV5Migration(
  workspace: MigrationWorkspace,
  originalProjects?: ReadonlyMap<string, ProjectPlan>,
): VitestV5MigrationPlan {
  const findings: VitestV5Finding[] = [];
  const directories = [
    workspace.rootDir,
    ...(workspace.packages ?? []).map((pkg) => path.resolve(workspace.rootDir, pkg.path)),
  ];
  const projects: ProjectPlan[] = [];
  const changes: FileChange[] = [];
  const projectSources = new Map(
    [...new Set(directories)].map((directory) => [
      directory,
      new Map(filesInProject(directory).map((file) => [file, fs.readFileSync(file, 'utf8')])),
    ]),
  );
  const allSources = new Map<string, string>();
  for (const sources of projectSources.values()) {
    for (const [file, source] of sources) {
      allSources.set(file, source);
    }
  }
  const configEntries = findVitestV5ConfigEntries(allSources, projectSources.keys());
  const allConfigs = findVitestV5ConfigFiles(
    allSources,
    configEntries.flatMap((entry) => (entry.file ? [entry.file] : [])),
  );
  const mergedConfigs = findVitestV5MergedConfigFiles(allSources, allConfigs);
  const browserPossible = [...allSources.values()].some((source) => BROWSER_SIGNAL.test(source));
  const browserCliOverride = [...allSources].some(
    ([file, source]) =>
      /\.(?:json|ya?ml|sh)$/.test(file) &&
      /\b(?:vitest|vp\s+test)\b[^\n]*--(?:no-)?browser(?:[.=\s'"]|$)/.test(source),
  );
  const testModesByVersion = new Map<boolean, ReturnType<typeof resolveVitestV5TestModes>>();
  const inputs = new Map<string, string | null>(allSources);
  for (const [directory, sources] of projectSources) {
    const pkg = readJson(path.join(directory, 'package.json'));
    const configFiles = new Set([...allConfigs].filter((file) => sources.has(file)));
    const original = originalProjects?.get(directory);
    const version = original
      ? original.sourceVersion
      : sourceVersion(directory, workspace.rootDir, pkg, workspace.packageManager);
    const active =
      original?.active ??
      (!!dependency(pkg, 'vitest') ||
        Object.values(record(pkg.scripts) ?? {}).some(
          (command) => typeof command === 'string' && VITEST_COMMAND.test(command),
        ) ||
        [...sources].some(
          ([file, source]) =>
            (CODE_FILE.test(file) && hasVitestV5SourceUsage(file, source)) ||
            (configFiles.has(file) && /\btest\s*:/.test(source)),
        ));
    const options: SourceOptions = {
      // Only the resolved source runner authorizes v4 compatibility edits.
      // The final scan disables edits while retaining this run's review context.
      preserveV4: original?.options.preserveV4 ?? (!!version && semver.major(version) < 5),
      reviewV4: !!version && semver.major(version) < 5,
      browser: [...sources.values()].some((source) => BROWSER_SIGNAL.test(source)),
      browserPossible,
      temporalPolyfill: [...sources.values()].some((source) =>
        /(?:['"]temporal-polyfill\/global['"]|(?:globalThis|global)\.Temporal\s*=|Object\.(?:assign|defineProperty)\(globalThis,\s*(?:\{\s*Temporal|['"]Temporal['"]))/.test(
          source,
        ),
      ),
    };
    // The community provider is user-managed. Do not invent a version when
    // restoring a removed Vite+ shim to a direct provider import.
    if (
      active &&
      !dependency(pkg, '@vitest/browser-webdriverio') &&
      !dependency(
        readJson(path.join(workspace.rootDir, 'package.json')),
        '@vitest/browser-webdriverio',
      ) &&
      usesWebdriverioProvider(directory)
    ) {
      findings.push(
        finding(
          path.join(directory, 'package.json'),
          'browser-provider',
          'Add @vitest/browser-webdriverio and its required peers using versions compatible with your tests. Vite+ no longer exports or manages this community provider.',
          'review',
        ),
      );
    }
    if (active && !version) {
      findings.push(
        finding(
          path.join(directory, 'package.json'),
          'source-version',
          'Cannot determine the original Vitest version. Install the original lockfile, then re-run migration so v4 defaults are not applied to a v5 project.',
          'block',
        ),
      );
    }
    if (active && version && semver.major(version) < 4) {
      findings.push(
        finding(
          path.join(directory, 'package.json'),
          'source-version',
          'Upgrade the original project to Vitest 4 before running this migration.',
          'block',
        ),
      );
    }
    const project = {
      directory,
      sourceVersion: version,
      active,
      options,
      configFiles,
      pendingSourceReviews: original?.pendingSourceReviews,
    };
    projects.push(project);
    let testModes = testModesByVersion.get(options.preserveV4);
    if (!testModes) {
      testModes = resolveVitestV5TestModes(
        allSources,
        allConfigs,
        options.preserveV4,
        configEntries,
      );
      testModesByVersion.set(options.preserveV4, testModes);
    }
    for (const [file, source] of sources) {
      // Ignore lockfile package snapshots; runtime and source checks belong to
      // project files, not thousands of transitive package metadata entries.
      if (/[/\\](?:pnpm-lock\.yaml|package-lock\.json|yarn\.lock|bun\.lock)$/.test(file)) {
        continue;
      }
      const result = scanAndRewriteFile(
        file,
        source,
        project,
        { ...testModes.modes.get(file), ...(browserCliOverride ? { browser: undefined } : {}) },
        mergedConfigs.has(file),
      );
      findings.push(...testModes.findings.filter((item) => item.file === file));
      findings.push(...result.findings);
      if (source !== result.content) {
        changes.push({ file, before: source, after: result.content });
      }
    }
  }
  return {
    rootDir: workspace.rootDir,
    packageManager: workspace.packageManager,
    projects,
    findings,
    changes,
    inputs,
  };
}

export function formatVitestV5Findings(
  plan: Pick<VitestV5MigrationPlan, 'rootDir' | 'findings'>,
): string {
  const unique = [
    ...new Map(
      plan.findings.map((item) => [
        `${item.file}:${item.line}:${item.column}:${item.code}:${item.message}`,
        item,
      ]),
    ).values(),
  ];
  if (!unique.length) {
    return '';
  }
  const blocks = unique.filter((item) => item.severity === 'block').length;
  const lines = [
    `Vitest v5: ${unique.length} review item${unique.length === 1 ? '' : 's'}${blocks ? ` (${blocks} block dependency updates)` : ''}`,
  ];
  let previous = '';
  for (const item of unique.toSorted(
    (a, b) => a.file.localeCompare(b.file) || a.line - b.line || a.column - b.column,
  )) {
    if (item.file !== previous) {
      lines.push(`\n${path.relative(plan.rootDir, item.file) || '.'}`);
      previous = item.file;
    }
    lines.push(
      `  ${item.line}:${item.column} ${item.severity === 'block' ? 'BLOCK' : 'REVIEW'} [${item.code}] ${item.message}`,
      `    Docs: ${vitestV5Documentation(item.code)}`,
    );
  }
  return lines.join('\n');
}

/** Apply a fully checked compatibility plan. Refuse stale input so an editor
 * change cannot be overwritten between phases. */
export function applyVitestV5Migration(plan: VitestV5MigrationPlan): number {
  if (plan.findings.some((item) => item.severity === 'block')) {
    throw new Error('Vitest v5 preflight has blocking findings. No migration edits were applied.');
  }
  for (const [file, before] of plan.inputs) {
    const current = fs.existsSync(file) ? fs.readFileSync(file, 'utf8') : null;
    if (current !== before) {
      throw new Error(`Migration input changed: ${file}. Re-run vp migrate.`);
    }
  }
  for (const change of plan.changes) {
    fs.writeFileSync(change.file, change.after);
  }
  return plan.changes.length;
}

/** Upgrade runtime declarations before installers or tool migrators start.
 * Keep source/config edits deferred until the tool version gates pass. */
export function applyVitestV5NodeMigration(plan: VitestV5MigrationPlan): number {
  const changes: FileChange[] = [];
  for (const [file, before] of plan.inputs) {
    if (
      before === null ||
      !['.node-version', '.nvmrc', 'package.json'].includes(path.basename(file))
    ) {
      continue;
    }
    const after = migrateNode(file, before, []);
    if (after !== before) {
      changes.push({ file, before, after });
    }
  }
  return applyVitestV5Migration({ ...plan, changes });
}

/** Earlier setup/install steps can change manifests and configs. Re-plan from
 * their current text without losing the runner version captured before install.
 * Apply immediately afterward, so the normal stale-input guard still protects
 * every write. Run this after the earlier tool migration gates have passed. */
export function refreshVitestV5Migration(plan: VitestV5MigrationPlan): VitestV5MigrationPlan {
  return planVitestV5Migration(
    workspaceFromPlan(plan),
    // Preserve both activity and version: removing a redundant runner must not
    // erase pending compatibility work, and adding one to satisfy a peer does
    // not prove the project previously used v4 defaults.
    new Map(plan.projects.map((project) => [project.directory, project])),
  );
}

export function vitestV5NeedsMigration(plan: VitestV5MigrationPlan): boolean {
  return (
    plan.changes.length > 0 ||
    plan.projects.some((project) => project.active && project.options.preserveV4)
  );
}

function currentProjectConfigs(plan: VitestV5MigrationPlan): Map<string, string[]> {
  const filesByProject = new Map(
    plan.projects.map((project) => [project.directory, filesInProject(project.directory)]),
  );
  const sources = new Map(
    [...filesByProject.values()].flat().map((file) => [file, fs.readFileSync(file, 'utf8')]),
  );
  const entries = findVitestV5ConfigEntries(sources, filesByProject.keys());
  const configs = findVitestV5ConfigFiles(
    sources,
    entries.flatMap((entry) => (entry.file ? [entry.file] : [])),
  );
  return new Map(
    [...filesByProject].map(([directory, files]) => [
      directory,
      files.filter((file) => configs.has(file)),
    ]),
  );
}

/** Run after other migration steps create or merge configs. Keep the source
 * version and deferred reviews in memory until the final report; no state file. */
export function finishVitestV5Migration(plan: VitestV5MigrationPlan): VitestV5Finding[] {
  const findings: VitestV5Finding[] = [];
  const completedProjects = new Map<string, ProjectPlan>();
  const projectConfigs = currentProjectConfigs(plan);
  const configSources = new Map(
    [...projectConfigs.values()].flat().map((file) => [file, fs.readFileSync(file, 'utf8')]),
  );
  const mergedConfigs = findVitestV5MergedConfigFiles(configSources, new Set(configSources.keys()));
  for (const project of plan.projects) {
    if (!project.active || !project.sourceVersion) {
      continue;
    }
    const configs = projectConfigs.get(project.directory)!;
    for (const file of configs) {
      const before = fs.readFileSync(file, 'utf8');
      try {
        const result = migrateVitestV5Config(
          file,
          before,
          project.options,
          mergedConfigs.has(file),
        );
        findings.push(...result.findings);
        if (
          !result.findings.some((finding) => finding.severity === 'block') &&
          result.content !== before
        ) {
          fs.writeFileSync(file, result.content);
        }
      } catch (error) {
        findings.push(finding(file, 'source-parse', `Review this config: ${String(error)}`));
      }
    }
    const ignore = path.join(project.directory, '.gitignore');
    const ignored = fs.existsSync(ignore) ? fs.readFileSync(ignore, 'utf8') : '';
    if (!ignored.split(/\r?\n/).some((line) => /^\/?\.vitest\/?$/.test(line.trim()))) {
      fs.writeFileSync(
        ignore,
        `${ignored}${ignored && !ignored.endsWith('\n') ? '\n' : ''}.vitest/\n`,
      );
    }
    const pendingSourceReviews: Record<string, string[]> = {};
    for (const item of plan.findings) {
      if (
        !DEFERRED_SOURCE_REVIEWS.has(item.code) ||
        !item.file.startsWith(`${project.directory}${path.sep}`)
      ) {
        continue;
      }
      if (
        plan.projects.some(
          (other) =>
            other !== project &&
            other.directory.startsWith(`${project.directory}${path.sep}`) &&
            item.file.startsWith(`${other.directory}${path.sep}`),
        )
      ) {
        continue;
      }
      const relative = path.relative(project.directory, item.file).replaceAll('\\', '/');
      pendingSourceReviews[relative] ??= [];
      if (!pendingSourceReviews[relative].includes(item.code)) {
        pendingSourceReviews[relative].push(item.code);
      }
    }
    completedProjects.set(project.directory, {
      ...project,
      options: { ...project.options, preserveV4: false },
      pendingSourceReviews,
    });
  }
  // Preserve inactive projects too: newly installed peers do not establish
  // original test usage. The next invocation redetects versions from scratch.
  const after = planVitestV5Migration(
    workspaceFromPlan(plan),
    new Map(
      plan.projects.map((project) => [
        project.directory,
        completedProjects.get(project.directory) ?? project,
      ]),
    ),
  );
  return [...findings, ...after.findings];
}
