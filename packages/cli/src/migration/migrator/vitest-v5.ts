import fs from 'node:fs';
import path from 'node:path';

import { parse as parseJsonc } from 'jsonc-parser';
import semver from 'semver';
import { isNode, isScalar, parseDocument, visit } from 'yaml';

import cliPackage from '../../../package.json' with { type: 'json' };
import { PackageManager, type WorkspaceInfoOptional } from '../../types/index.ts';
import { detectPackageMetadata } from '../../utils/package.ts';
import { createCatalogDependencyResolver } from '../migrator.ts';
import { type SourceOptions, type VitestV5Finding } from '../vitest-v5/ast.ts';
import { migrateVitestV5Command } from '../vitest-v5/commands.ts';
import {
  findVitestV5ConfigFiles,
  findVitestV5MergedConfigFiles,
  migrateVitestV5Config,
  resolveVitestV5BrowserModes,
} from '../vitest-v5/config.ts';
import { migrateVitestV5Source } from '../vitest-v5/source.ts';

const STATE_PATH = '.vite-plus/migrations.json';
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
  /(?:['"](?:vitest(?:\/[^'"]*)?|@vitest\/[^'"]+|vite-plus\/test[^'"]*)['"]|\bvp\s+test\b|\bvitest\s+(?:run|list|bench))/;
const VITEST_COMMAND = /\b(?:vp\s+test|vitest)(?:\s|$)/;
const BROWSER_SIGNAL =
  /@vitest\/browser|(?:vitest|vite-plus\/test)\/browser|vitest-browser-|browser\s*:\s*\{|--browser(?:[.=\s'"]|$)/;
const LEGACY_RUNNER = '@voidzero-dev/vite-plus-test';

interface ProjectState {
  sourceVersion: string;
  configless: boolean;
  pendingSourceReviews?: Record<string, string[]>;
}
interface MigrationState {
  version: 1;
  vitest5?: Record<string, ProjectState>;
}
interface ProjectPlan {
  directory: string;
  sourceVersion?: string;
  active: boolean;
  options: SourceOptions;
  configFiles: Set<string>;
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
  state: MigrationState;
}

function readJson(file: string): Record<string, unknown> {
  return JSON.parse(fs.readFileSync(file, 'utf8').replace(/^\uFEFF/, ''));
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

function projectStateKey(root: string, directory: string): string {
  return path.relative(root, directory).replaceAll('\\', '/') || '.';
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
          ['.node-version', '.nvmrc', '.gitignore'].includes(entry.name))
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
    return undefined;
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
): string | undefined {
  const installed = detectPackageMetadata(directory, name);
  if (!installed || (expectedPackage && installed.name !== expectedPackage)) {
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
  let spec = dependency(pkg, 'vitest');
  if (spec?.startsWith('catalog:')) {
    spec = catalog?.(spec, 'vitest');
  }
  const legacyAlias = spec?.startsWith(`npm:${LEGACY_RUNNER}@`);
  if (spec?.startsWith('npm:vitest@')) {
    spec = spec.slice('npm:vitest@'.length);
  }
  if (!spec && dependency(pkg, 'vite-plus')) {
    const bundled = installedSourceVersion(directory, 'vite-plus');
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
    const bundled = installedSourceVersion(directory, 'vite-plus');
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
  source = '',
  offset = 0,
) {
  const range = semver.validRange(value);
  if (!range) {
    findings.push(
      finding(
        file,
        'node-runtime',
        `Resolve ${label} (${value}) and select Node ${cliPackage.engines.node}.`,
        'review',
        source,
        offset,
      ),
    );
  } else if (!publicContract && !semver.intersects(range, cliPackage.engines.node)) {
    findings.push(
      finding(
        file,
        'node-runtime',
        `${label} (${value}) cannot run Vite+ with Vitest v5. Select Node ${cliPackage.engines.node}; use vp env pin 22 --force for a runtime pin. Do not widen a library's engines.node contract automatically.`,
        'block',
        source,
        offset,
      ),
    );
  } else if (
    !semver.subset(range, cliPackage.engines.node) &&
    (publicContract || /[<>|*]/.test(value))
  ) {
    findings.push(
      finding(
        file,
        'node-runtime',
        `${label} (${value}) includes unsupported test runtimes. Pin the test/CI runtime to Node ${cliPackage.engines.node}; keep the library's public engine contract separate.`,
        'review',
        source,
        offset,
      ),
    );
  }
}

function scanNode(file: string, source: string, findings: VitestV5Finding[]) {
  const base = path.basename(file);
  const checkImage = (image: unknown, offset = 0) => {
    if (typeof image !== 'string') {
      return;
    }
    // Only the official Node image has a tag that identifies the runtime.
    // Feature versions and custom image tags have a different meaning.
    const match = /^(?:docker\.io\/)?(?:library\/)?node(?::([^@\s]+))?(?:@\S+)?$/.exec(image);
    if (match) {
      const tag = match[1] ?? 'latest';
      const version = /^(\d+(?:\.\d+){0,2})(?:-[\w.-]+)?$/.exec(tag)?.[1] ?? tag;
      checkNodeRange(file, version, 'Node container image', findings, false, source, offset);
    }
  };
  if (base === '.node-version' || base === '.nvmrc') {
    checkNodeRange(file, source.trim().replace(/^v/, ''), base, findings);
  }
  if (base === 'package.json') {
    const pkg = JSON.parse(source);
    if (typeof pkg.engines?.node === 'string') {
      checkNodeRange(file, pkg.engines.node, 'engines.node', findings, true);
    }
    const runtime = pkg.devEngines?.runtime;
    for (const entry of Array.isArray(runtime) ? runtime : [runtime]) {
      if (entry?.name === 'node' && typeof entry.version === 'string') {
        checkNodeRange(file, entry.version, 'devEngines.runtime', findings);
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
      checkNodeRange(file, pkg.volta.node, 'volta.node', findings);
    }
  }
  if (/\.ya?ml$/.test(file)) {
    const document = parseDocument(source);
    visit(document, {
      Pair(_key, pair) {
        if (!isScalar(pair.key)) {
          return;
        }
        const key = String(pair.key.value);
        const value = isNode(pair.value) ? pair.value.toJSON() : undefined;
        if (key === 'image' || key === 'container') {
          checkImage(value, pair.key.range?.[0] ?? 0);
        }
        if (!['node', 'node-version', 'nodejs'].includes(key)) {
          return;
        }
        for (const item of Array.isArray(value) ? value : [value]) {
          if (typeof item === 'number' || typeof item === 'string') {
            checkNodeRange(
              file,
              String(item),
              key,
              findings,
              false,
              source,
              pair.key.range?.[0] ?? 0,
            );
          }
        }
      },
    });
  }
  if (/^(?:Dockerfile|Containerfile)(?:\.|$)/.test(base)) {
    for (const match of source.matchAll(/^\s*FROM[\t ]+(?:--platform=\S+[\t ]+)?(\S+)/gim)) {
      checkImage(match[1], match.index);
    }
  }
  if (base === 'devcontainer.json' || base === '.devcontainer.json') {
    const config = record(parseJsonc(source));
    checkImage(config?.image);
    for (const [feature, options] of Object.entries(record(config?.features) ?? {})) {
      if (/^ghcr\.io\/devcontainers\/features\/node(?::[^/]+)?$/.test(feature)) {
        const version = record(options)?.version;
        if (typeof version === 'string') {
          checkNodeRange(file, version, 'Dev Container Node feature', findings);
        }
      }
    }
  }
}

function scanAndRewriteFile(
  file: string,
  source: string,
  project: ProjectPlan,
  browserMode: boolean | undefined,
  mergedConfig: boolean,
): { content: string; findings: VitestV5Finding[] } {
  const findings: VitestV5Finding[] = [];
  scanNode(file, source, findings);
  if (!project.active) {
    return { content: source, findings };
  }
  let content = source;
  if (
    CODE_FILE.test(file) &&
    (VITEST_SIGNAL.test(source) || /\.(?:test|spec)\./.test(file) || project.configFiles.has(file))
  ) {
    try {
      // Canonical and symbol-specific rules run before the native generic import
      // pass. Config and source edits are separate AST passes to avoid overlap.
      const sourceResult = migrateVitestV5Source(file, content, {
        ...project.options,
        browser: browserMode,
      });
      content = sourceResult.content;
      findings.push(...sourceResult.findings);
      const pending =
        project.pendingSourceReviews?.[
          path.relative(project.directory, file).replaceAll('\\', '/')
        ];
      if (!project.options.preserveV4 && pending?.length) {
        // Re-evaluate deferred v4 findings without applying v4 edits again.
        // Successfully migrated files have no pending entry, so valid v5
        // resolveConfig consumers do not gain a warning after migration.
        const review = migrateVitestV5Source(file, content, {
          ...project.options,
          browser: browserMode,
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
    const pkg = JSON.parse(source);
    let changed = false;
    for (const [name, command] of Object.entries(pkg.scripts ?? {})) {
      if (typeof command !== 'string') {
        continue;
      }
      const result = migrateVitestV5Command(
        file,
        command,
        project.options.preserveV4,
        source.slice(0, source.indexOf(JSON.stringify(name))).split('\n').length,
      );
      findings.push(...result.findings);
      if (result.content !== command) {
        pkg.scripts[name] = result.content;
        changed = true;
      }
    }
    if (changed) {
      content = `${JSON.stringify(pkg, null, /\n([ \t]+)"/.exec(source)?.[1] ?? 2)}\n`;
    }
    for (const name of ['@vitest/ws-client', '@vitest/runner', '@vitest/expect']) {
      if (dependency(pkg, name)) {
        findings.push(
          finding(
            file,
            'legacy-dependency',
            `Review the direct ${name} dependency after migrating its imports; it is no longer part of the bundled Vitest graph.`,
          ),
        );
      }
    }
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
      ...migrateVitestV5Command(file, `(${source})`, project.options.preserveV4).findings,
    );
  }
  // CI upload paths and tools that consume reports may not invoke Vitest.
  if (!CODE_FILE.test(file) && path.basename(file) !== '.gitignore') {
    for (const match of source.matchAll(
      /\.vitest-attachements\/?|\.vitest-reports\/?|__screenshots__\/?|html\/index\.html/g,
    )) {
      findings.push(
        finding(
          file,
          'artifact-paths',
          'Review this consumer of old output paths. Artifacts now use .vitest/attachments, .vitest/blob, and .vitest/index.html.',
          'review',
          source,
          match.index,
        ),
      );
    }
  }
  return { content, findings };
}

/** Read-only preflight. Call before package-manager conversion, installs,
 * catalog updates, or generic import rewrites can erase the source version. */
export function planVitestV5Migration(
  workspace: Pick<WorkspaceInfoOptional, 'rootDir' | 'packageManager'> &
    Partial<Pick<WorkspaceInfoOptional, 'packages'>>,
): VitestV5MigrationPlan {
  const stateFile = path.join(workspace.rootDir, STATE_PATH);
  let state: MigrationState = { version: 1 };
  const findings: VitestV5Finding[] = [];
  if (fs.existsSync(stateFile)) {
    try {
      state = JSON.parse(fs.readFileSync(stateFile, 'utf8'));
      if (
        state.version !== 1 ||
        (state.vitest5 &&
          (!record(state.vitest5) ||
            Object.values(state.vitest5).some(
              (project) =>
                !project ||
                typeof project.sourceVersion !== 'string' ||
                !semver.valid(project.sourceVersion) ||
                typeof project.configless !== 'boolean' ||
                (project.pendingSourceReviews !== undefined &&
                  (!record(project.pendingSourceReviews) ||
                    Object.values(project.pendingSourceReviews).some(
                      (codes) =>
                        !Array.isArray(codes) || codes.some((code) => typeof code !== 'string'),
                    ))),
            )))
      ) {
        throw new Error('Unsupported state format');
      }
    } catch {
      findings.push(
        finding(
          stateFile,
          'migration-state',
          'Repair the existing migration state before upgrading; it will not be overwritten.',
          'block',
        ),
      );
      state = { version: 1 };
    }
  }
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
  const allConfigs = findVitestV5ConfigFiles(allSources);
  const mergedConfigs = findVitestV5MergedConfigFiles(allSources, allConfigs);
  const browserPossible = [...allSources.values()].some((source) => BROWSER_SIGNAL.test(source));
  const browserCliOverride = [...allSources].some(
    ([file, source]) =>
      /\.(?:json|ya?ml|sh)$/.test(file) &&
      /\b(?:vitest|vp\s+test)\b[^\n]*--(?:no-)?browser(?:[.=\s'"]|$)/.test(source),
  );
  const browserModesByVersion = new Map<boolean, Map<string, boolean | undefined>>();
  const inputs = new Map<string, string | null>(allSources);
  inputs.set(stateFile, fs.existsSync(stateFile) ? fs.readFileSync(stateFile, 'utf8') : null);
  for (const [directory, sources] of projectSources) {
    const pkg = readJson(path.join(directory, 'package.json'));
    const configFiles = new Set([...allConfigs].filter((file) => sources.has(file)));
    const previous = state.vitest5?.[projectStateKey(workspace.rootDir, directory)];
    const version =
      previous?.sourceVersion ??
      sourceVersion(directory, workspace.rootDir, pkg, workspace.packageManager);
    const active =
      !!previous ||
      !!dependency(pkg, 'vitest') ||
      Object.values(record(pkg.scripts) ?? {}).some(
        (command) => typeof command === 'string' && VITEST_COMMAND.test(command),
      ) ||
      [...sources].some(
        ([file, source]) =>
          (CODE_FILE.test(file) && VITEST_SIGNAL.test(source)) ||
          (configFiles.has(file) && /\btest\s*:/.test(source)),
      );
    const options: SourceOptions = {
      preserveV4: !previous && !!version && semver.major(version) < 5,
      reviewV4: !!version && semver.major(version) < 5,
      browser: [...sources.values()].some((source) => BROWSER_SIGNAL.test(source)),
      browserPossible,
      globals: [...sources].some(
        ([file, source]) => configFiles.has(file) && /\bglobals\s*:\s*true\b/.test(source),
      ),
      temporalPolyfill: [...sources.values()].some((source) =>
        /(?:['"]temporal-polyfill\/global['"]|(?:globalThis|global)\.Temporal\s*=|Object\.(?:assign|defineProperty)\(globalThis,\s*(?:\{\s*Temporal|['"]Temporal['"]))/.test(
          source,
        ),
      ),
    };
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
      pendingSourceReviews: previous?.pendingSourceReviews,
    };
    projects.push(project);
    let browserModes = browserModesByVersion.get(options.preserveV4);
    if (!browserModes) {
      browserModes = resolveVitestV5BrowserModes(allSources, allConfigs, options.preserveV4);
      browserModesByVersion.set(options.preserveV4, browserModes);
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
        browserCliOverride ? undefined : browserModes.get(file),
        mergedConfigs.has(file),
      );
      findings.push(...result.findings);
      if (source !== result.content) {
        changes.push({ file, before: source, after: result.content });
      }
    }
    if (active && (options.preserveV4 || previous?.configless) && configFiles.size === 0) {
      findings.push(
        finding(
          path.join(directory, 'package.json'),
          'configless-defaults',
          `No test config exists. Vitest v5 clears mocks by default${options.browser ? ' and uses exact browser locators' : ''}. A separate confirmed action can create compatibility config, but a new config can change config discovery and project structure.`,
        ),
      );
    }
  }
  return {
    rootDir: workspace.rootDir,
    packageManager: workspace.packageManager,
    projects,
    findings,
    changes,
    inputs,
    state,
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
    );
  }
  return lines.join('\n');
}

/** Apply a fully checked plan before the existing generic migration. Refuse
 * stale input so an editor change cannot be overwritten between phases. */
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

export function vitestV5NeedsMigration(plan: VitestV5MigrationPlan): boolean {
  return (
    plan.changes.length > 0 ||
    plan.projects.some(
      (project) =>
        project.active && !plan.state.vitest5?.[projectStateKey(plan.rootDir, project.directory)],
    )
  );
}

export function vitestV5ConfiglessProjects(plan: VitestV5MigrationPlan): string[] {
  const configs = currentProjectConfigs(plan);
  return plan.projects
    .filter(
      (project) =>
        project.active &&
        (project.options.preserveV4 ||
          plan.state.vitest5?.[projectStateKey(plan.rootDir, project.directory)]?.configless) &&
        configs.get(project.directory)!.length === 0,
    )
    .map((project) => project.directory);
}

function currentProjectConfigs(plan: VitestV5MigrationPlan): Map<string, string[]> {
  const filesByProject = new Map(
    plan.projects.map((project) => [project.directory, filesInProject(project.directory)]),
  );
  const configs = findVitestV5ConfigFiles(
    new Map(
      [...filesByProject.values()].flat().map((file) => [file, fs.readFileSync(file, 'utf8')]),
    ),
  );
  return new Map(
    [...filesByProject].map(([directory, files]) => [
      directory,
      files.filter((file) => configs.has(file)),
    ]),
  );
}

/** Run after other migration steps create or merge configs. Do not create a
 * config here. Record completion even for configless projects. */
export function finishVitestV5Migration(plan: VitestV5MigrationPlan): VitestV5Finding[] {
  const findings: VitestV5Finding[] = [];
  const state = structuredClone(plan.state);
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
    state.vitest5 ??= {};
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
    state.vitest5[projectStateKey(plan.rootDir, project.directory)] = {
      sourceVersion: project.sourceVersion,
      configless: configs.length === 0,
      ...(Object.keys(pendingSourceReviews).length ? { pendingSourceReviews } : {}),
    };
  }
  if (state.vitest5) {
    const stateFile = path.join(plan.rootDir, STATE_PATH);
    fs.mkdirSync(path.dirname(stateFile), { recursive: true });
    fs.writeFileSync(stateFile, `${JSON.stringify(state, null, 2)}\n`);
  }
  // A fresh scan keeps unresolved source risks visible after successful edits.
  const after = planVitestV5Migration({
    rootDir: plan.rootDir,
    packages: plan.projects
      .slice(1)
      .map((project) => ({ name: '', path: path.relative(plan.rootDir, project.directory) })),
    packageManager: plan.packageManager,
  });
  return [...findings, ...after.findings];
}

/** The caller must obtain a separate explicit confirmation. This is never a
 * default side effect of the versioned pass or noninteractive --yes mode. */
export function createVitestV5CompatibilityConfig(
  plan: VitestV5MigrationPlan,
  directory: string,
): string {
  const project = plan.projects.find((project) => project.directory === directory);
  if (!project?.active || !project.sourceVersion || semver.major(project.sourceVersion) >= 5) {
    throw new Error('This project does not need a v4 compatibility config.');
  }
  if (currentProjectConfigs(plan).get(directory)!.length > 0) {
    throw new Error('A test config already exists; it will not be overwritten.');
  }
  const file = path.join(directory, 'vite.config.ts');
  const browser = project.options.browser ? ', browser: { locators: { exact: false } }' : '';
  const temporal = project.options.temporalPolyfill
    ? ", fakeTimers: { toNotFake: ['Temporal'] }"
    : '';
  fs.writeFileSync(
    file,
    `import { defineConfig } from 'vite-plus';\n\nexport default defineConfig({ test: { clearMocks: false${browser}${temporal} } });\n`,
    { flag: 'wx' },
  );
  return file;
}
