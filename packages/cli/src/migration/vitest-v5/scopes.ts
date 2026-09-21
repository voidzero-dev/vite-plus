import path from 'node:path';

import type * as t from '@oxc-project/types';
import { minimatch } from 'minimatch';

import {
  SourceEditor,
  importedName,
  isBoolean,
  isModuleExports,
  isString,
  objectProperty,
  propertyName,
  memberName,
  projectElements,
  staticObject,
  type SourceOptions,
  type VitestV5Finding,
} from './ast.ts';
import { literalTestCommands, vitestCommandArgsStart } from './commands.ts';
import { resolveConfigObject } from './config-object.ts';

interface ConfigEntry {
  implicit?: boolean;
  browserOverride?: boolean;
  file?: string;
  root: string;
  uncertain?: boolean;
  rootOverride?: string;
  dirOverride?: string;
  cwd?: string;
  projects?: string[];
}

interface TestScope {
  excludedRoots?: string[];
  root: string;
  dir?: string;
  discoveryRoot?: string;
  include?: string[];
  exclude?: string[];
  benchmark?: {
    include?: string[];
    exclude?: string[];
    includeSource?: string[];
  };
  setupFiles?: string[];
  unresolvedSetup?: boolean;
  browser?: boolean;
  globals?: boolean;
}

export type VitestV5TestMode = Pick<SourceOptions, 'browser' | 'globals' | 'reviewGlobals'> & {
  benchmark?: boolean;
};

const DEFAULT_TEST_INCLUDE = ['**/*.{test,spec}.?(c|m)[jt]s?(x)'];
const DEFAULT_TEST_EXCLUDE = ['**/node_modules/**', '**/.git/**'];
const DEFAULT_BENCHMARK_INCLUDE = ['**/*.{bench,benchmark}.?(c|m)[jt]s?(x)'];
const DEFAULT_BENCHMARK_EXCLUDE = ['**/node_modules/**', '**/.git/**'];
const TEST_CONFIG_SOURCES = new Set(['vitest/config', 'vite-plus/test/config', 'vite-plus']);
const SCOPE_KEYS = [
  'root',
  'dir',
  'include',
  'exclude',
  'includeSource',
  'setupFiles',
  'globals',
  'browser',
  'benchmark',
  'projects',
  'name',
] as const;
// Match Vitest's discovery order, including precedence across extensions.
const CONFIG_NAMES = ['vitest.config', 'vite.config'].flatMap((name) =>
  ['.ts', '.mts', '.cts', '.js', '.mjs', '.cjs'].map((extension) => name + extension),
);

function defaultConfig(sources: ReadonlyMap<string, string>, root: string): string | undefined {
  return CONFIG_NAMES.map((name) => path.join(root, name)).find((file) => sources.has(file));
}

/** Candidate config files are not all active projects. Select each package's
 * default config or literal script selections before following project edges. */
export function findVitestV5ConfigEntries(
  sources: ReadonlyMap<string, string>,
  directories: Iterable<string>,
): ConfigEntry[] {
  const entries: ConfigEntry[] = [];
  for (const root of directories) {
    const manifest = JSON.parse(
      (sources.get(path.join(root, 'package.json')) ?? '{}').replace(/^\uFEFF/, ''),
    );
    const commands = Object.values(manifest.scripts ?? {}).filter(
      (command): command is string =>
        typeof command === 'string' && /\bvitest\b|\bvp\s+test\b/.test(command),
    );
    if (!commands.length) {
      entries.push({ root, file: defaultConfig(sources, root), implicit: true });
      continue;
    }
    for (const command of commands) {
      const invocations = literalTestCommands(command)?.filter(
        (argv) => vitestCommandArgsStart(argv) >= 0,
      );
      if (!invocations?.length) {
        entries.push({ root, uncertain: true });
        continue;
      }
      for (const argv of invocations) {
        const start = vitestCommandArgsStart(argv);
        let config: string | undefined;
        let rootOverride: string | undefined;
        let dirOverride: string | undefined;
        let uncertain = false;
        let browserOverride = false;
        const projects: string[] = [];
        for (let index = start; index < argv.length && argv[index] !== '--'; index++) {
          const arg = argv[index];
          const option = arg.split('=')[0];
          browserOverride ||= /^--(?:no-)?browser(?:[.=]|$)/.test(arg);
          if (['--config', '-c', '--root', '-r', '--dir', '--project', '-p'].includes(option)) {
            const value = arg.includes('=') ? arg.slice(arg.indexOf('=') + 1) : argv[++index];
            if (!value || value.startsWith('-')) {
              uncertain = true;
            } else if (option === '--project' || option === '-p') {
              projects.push(value);
            } else if (option === '--config' || option === '-c') {
              config = value;
            } else if (option === '--dir') {
              dirOverride = path.resolve(root, value);
            } else {
              rootOverride = path.resolve(root, value);
            }
          } else if (
            /^--(?:no-)?(?:globals|include|exclude|workspace|setupFiles|benchmark\.(?:include|exclude|includeSource))(?:[.=]|$)/.test(
              arg,
            )
          ) {
            // These flags change ownership. Preserve globals until the command's
            // effective project options can be determined, rather than guessing.
            uncertain = true;
          }
        }
        const configRoot = rootOverride ?? root;
        const file = config ? path.resolve(configRoot, config) : defaultConfig(sources, configRoot);
        entries.push({
          root,
          file,
          rootOverride,
          dirOverride,
          projects,
          browserOverride,
          uncertain: uncertain || (!!config && !sources.has(file!)),
        });
      }
    }
  }
  return entries;
}

function staticPatterns(node: t.Node | undefined, editor: SourceEditor): string[] | undefined {
  if (
    node?.type === 'MemberExpression' &&
    importedName(editor, node.object, TEST_CONFIG_SOURCES) === 'configDefaults'
  ) {
    if (memberName(node) === 'include') {
      return DEFAULT_TEST_INCLUDE;
    }
    if (memberName(node) === 'exclude') {
      return DEFAULT_TEST_EXCLUDE;
    }
  }
  if (node?.type !== 'ArrayExpression') {
    return undefined;
  }
  const patterns: string[] = [];
  for (const entry of node.elements) {
    if (isString(entry)) {
      patterns.push(entry.value);
    } else if (entry?.type === 'SpreadElement') {
      const values = staticPatterns(entry.argument, editor);
      if (!values) {
        return undefined;
      }
      patterns.push(...values);
    } else {
      return undefined;
    }
  }
  return patterns.some((pattern) => pattern.startsWith('!')) ? undefined : patterns;
}

function booleanOption(
  node: t.Node | undefined,
  fallback: boolean | undefined,
): boolean | undefined {
  if (!node) {
    return fallback;
  }
  return isBoolean(node) ? node.value : undefined;
}

function benchmarkPatterns(
  node: t.Node | undefined,
  inherited: TestScope['benchmark'],
  editor: SourceEditor,
): TestScope['benchmark'] {
  if (!inherited || (node && !staticObject(node))) {
    return undefined;
  }
  const patterns = { ...inherited };
  for (const key of ['include', 'exclude', 'includeSource'] as const) {
    const value = node && objectProperty(node, key)?.value;
    if (value) {
      const resolved = staticPatterns(value, editor);
      if (!resolved) {
        return undefined;
      }
      patterns[key] = [...(inherited[key] ?? []), ...resolved];
    }
  }
  return patterns;
}

function matches(file: string, root: string, pattern: string): boolean {
  return minimatch(path.relative(root, file).replaceAll('\\', '/'), pattern.replace(/^\.\//, ''), {
    dot: true,
  });
}

function insideDirectory(file: string, directory: string): boolean {
  const relative = path.relative(directory, file);
  return relative !== '..' && !relative.startsWith(`..${path.sep}`) && !path.isAbsolute(relative);
}

function matchesProject(name: string, pattern: string): boolean {
  // Vitest supports only '*', case-insensitively; other glob syntax is literal.
  const expression = pattern
    .split('*')
    .map((part) => part.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'))
    .join('.*');
  return new RegExp(`^${expression}$`, 'i').test(name);
}

function projectSelected(
  test: t.ObjectExpression | undefined,
  filters: string[],
  inheritedBrowser: boolean | undefined,
): boolean | undefined {
  if (!filters.length) {
    return true;
  }
  const name = test && objectProperty(test, 'name')?.value;
  if (!isString(name)) {
    return undefined;
  }
  const positives = filters.filter((pattern) => !pattern.startsWith('!'));
  const excluded = (name: string) =>
    filters.some((pattern) => pattern.startsWith('!') && matchesProject(name, pattern.slice(1)));
  if (excluded(name.value)) {
    return false;
  }
  if (!positives.length || positives.some((pattern) => matchesProject(name.value, pattern))) {
    return true;
  }
  const browser = test && objectProperty(test, 'browser')?.value;
  if (!browser) {
    return inheritedBrowser === false ? false : undefined;
  }
  if (!staticObject(browser)) {
    return undefined;
  }
  const enabled = objectProperty(browser, 'enabled')?.value;
  if (isBoolean(enabled) && !enabled.value) {
    return false;
  }
  const instances = objectProperty(browser, 'instances')?.value;
  if (instances?.type !== 'ArrayExpression') {
    return undefined;
  }
  for (const instance of instances.elements) {
    if (!staticObject(instance)) {
      return undefined;
    }
    const explicitName = objectProperty(instance, 'name')?.value;
    const browserName = objectProperty(instance, 'browser')?.value;
    let instanceName: string | undefined;
    if (isString(explicitName)) {
      instanceName = explicitName.value;
    } else if (!explicitName && isString(browserName)) {
      instanceName = name.value ? `${name.value} (${browserName.value})` : browserName.value;
    }
    // Instance-specific overrides need review; never silently skip an instance
    // selected by its name rather than by its parent project name.
    if (
      !instanceName ||
      (!excluded(instanceName) &&
        positives.some((pattern) => matchesProject(instanceName, pattern)))
    ) {
      return undefined;
    }
  }
  return false;
}

/** Vitest resolves local setup entries through local-pkg/mlly with import
 * conditions, then falls back to the literal path. Do not guess TypeScript
 * extensions or package exports that cannot be resolved from scanned files. */
function resolveSetupFile(
  sources: ReadonlyMap<string, string>,
  root: string,
  reference: string,
): string | undefined {
  const target = path.resolve(root, reference);
  if (!/^\.\.?(?:[/\\]|$)/.test(reference) && !path.isAbsolute(reference)) {
    // A bare-looking path may be a root-relative file, but do not confuse a
    // declared package with a same-named local path.
    const packageName = reference
      .split('/')
      .slice(0, reference.startsWith('@') ? 2 : 1)
      .join('/');
    for (const parent of sources.keys()) {
      if (
        path.basename(parent) !== 'package.json' ||
        !insideDirectory(root, path.dirname(parent))
      ) {
        continue;
      }
      const manifest = JSON.parse(sources.get(parent)!.replace(/^\uFEFF/, ''));
      if (
        ['dependencies', 'devDependencies', 'peerDependencies', 'optionalDependencies'].some(
          (key) => manifest[key]?.[packageName],
        )
      ) {
        return undefined;
      }
    }
    // After package resolution fails, Vitest falls back to the literal path,
    // without trying local extensions or directory indexes for a bare entry.
    return sources.has(target) ? target : undefined;
  }
  const extensions = ['.mjs', '.cjs', '.js', '.json'];
  return [
    target,
    ...extensions.map((extension) =>
      /[/\\]$/.test(reference) ? path.join(target, extension) : target + extension,
    ),
    ...extensions.map((extension) => path.join(target, 'index' + extension)),
  ].find((candidate) => sources.has(candidate));
}

/** Resolve only selected configs and their project references. Unknown config
 * or conflicting project globals requires review, not silent suppression. */
export function resolveVitestV5TestModes(
  sources: ReadonlyMap<string, string>,
  configFiles: ReadonlySet<string>,
  preserveV4: boolean,
  entries: ConfigEntry[],
): { modes: Map<string, VitestV5TestMode>; findings: VitestV5Finding[] } {
  const scopes: TestScope[] = [];
  const findings: VitestV5Finding[] = [];
  const queue = [...entries];
  const packageRoots = [...new Set(entries.map((entry) => entry.root))];
  const visitedEntries = new Set<string>();
  for (const entry of queue) {
    const key = JSON.stringify(entry);
    if (visitedEntries.has(key)) {
      continue;
    }
    visitedEntries.add(key);
    const unknown = () => scopes.push({ root: entry.root });
    if (entry.uncertain) {
      unknown();
      continue;
    }
    if (!entry.file) {
      continue;
    }
    let found = false;
    let editor: SourceEditor;
    const visited = new Set<t.ObjectExpression>();
    const walk = (
      object: t.Node | undefined,
      directory = entry.root,
      base?: TestScope,
      inline = false,
    ) => {
      found = true;
      const resolved = resolveConfigObject(editor, object);
      if (!resolved) {
        unknown();
        return;
      }
      object = resolved;
      if (visited.has(object)) {
        return;
      }
      visited.add(object);
      const test = objectProperty(object, 'test')?.value;
      if (test && !staticObject(test, true)) {
        unknown();
        return;
      }
      // test.root overrides root. Inline roots are relative to the declaring
      // project's root even when extends:false disables option inheritance.
      const root =
        (test && objectProperty(test, 'root')?.value) ?? objectProperty(object, 'root')?.value;
      if (root && !isString(root)) {
        unknown();
        return;
      }
      const resolvedRoot =
        (!inline && entry.rootOverride) ||
        path.resolve(directory, isString(root) ? root.value : '.');
      const projects = test && objectProperty(test, 'projects')?.value;
      const selected = projects
        ? true
        : projectSelected(test, entry.projects ?? [], base ? base.browser : false);
      if (selected === false) {
        return;
      }
      const dirValue = test && objectProperty(test, 'dir')?.value;
      let dir = base ? base.dir : '';
      if (dirValue) {
        dir = isString(dirValue) ? dirValue.value : undefined;
      }
      // Unlike globals, --dir is not a per-project CLI override in Vitest.
      const effectiveDir = (!inline ? entry.dirOverride : undefined) ?? dir;
      const include = test && objectProperty(test, 'include')?.value;
      const exclude = test && objectProperty(test, 'exclude')?.value;
      const includePatterns = include ? staticPatterns(include, editor) : undefined;
      const excludePatterns = exclude ? staticPatterns(exclude, editor) : undefined;
      if ((include && !includePatterns) || (exclude && !excludePatterns)) {
        unknown();
        return;
      }
      const browser = test && objectProperty(test, 'browser')?.value;
      const enabled = staticObject(browser) ? objectProperty(browser, 'enabled')?.value : undefined;
      let browserMode = base ? base.browser : false;
      if (browser) {
        browserMode = staticObject(browser) ? booleanOption(enabled, browserMode) : undefined;
      }
      const globals = test && objectProperty(test, 'globals')?.value;
      const benchmark = test && objectProperty(test, 'benchmark')?.value;
      const setup = test && objectProperty(test, 'setupFiles')?.value;
      let setupFiles: string[] | undefined = [];
      if (setup) {
        setupFiles = isString(setup) ? [setup.value] : staticPatterns(setup, editor);
      }
      // Vitest passes dir to the globber as cwd, independently of root.
      // Relative values use the command's cwd, including in project configs.
      let discoveryRoot: string | undefined;
      if (effectiveDir !== undefined) {
        discoveryRoot = effectiveDir
          ? path.resolve(entry.cwd ?? entry.root, effectiveDir)
          : resolvedRoot;
      }
      const scope: TestScope = {
        // Merely discovering a workspace's tooling config does not make it a
        // test invocation over every child package. Explicit scripts, Vitest
        // configs, and ownership options still authorize cross-package scopes.
        // Compatibility defaults inserted on a later pass do not change this.
        excludedRoots:
          entry.implicit &&
          !inline &&
          !root &&
          path.basename(entry.file!).startsWith('vite.config.') &&
          (!test ||
            test.properties.every(
              (property) =>
                property.type === 'Property' &&
                ['clearMocks', 'fakeTimers'].includes(propertyName(property.key) ?? ''),
            ))
            ? packageRoots.filter(
                (root) => root !== entry.root && insideDirectory(root, entry.root),
              )
            : undefined,
        root: resolvedRoot,
        dir,
        discoveryRoot,
        include: includePatterns ? [...(base?.include ?? []), ...includePatterns] : base?.include,
        exclude: excludePatterns ? [...(base?.exclude ?? []), ...excludePatterns] : base?.exclude,
        // Apply defaults only when matching: implicit parent defaults must not
        // be appended to an inline project's explicit benchmark patterns.
        benchmark: benchmarkPatterns(benchmark, base ? base.benchmark : {}, editor),
        setupFiles:
          setupFiles && (!base || base.setupFiles)
            ? [...(base?.setupFiles ?? []), ...setupFiles]
            : undefined,
        browser: entry.browserOverride ? undefined : browserMode,
        globals:
          selected === undefined ? undefined : booleanOption(globals, base ? base.globals : false),
      };
      const addScope = () => {
        const resolvedSetup = scope.setupFiles?.map((reference) =>
          resolveSetupFile(sources, scope.root, reference),
        );
        const unresolvedSetup = !resolvedSetup || resolvedSetup.some((file) => !file);
        if (!scope.discoveryRoot) {
          editor.report(
            dirValue ?? object,
            'global-api-ownership',
            'Resolve test.dir before migrating global APIs. The test discovery directory is not statically known.',
          );
        }
        if (unresolvedSetup) {
          editor.report(
            setup ?? object,
            'global-api-ownership',
            'Resolve setupFiles before migrating global APIs. A setup entry cannot be resolved safely from the scanned files.',
          );
        }
        if (!scope.benchmark) {
          editor.report(
            benchmark ?? object,
            'global-api-ownership',
            'Resolve test.benchmark file patterns before migrating global APIs. Benchmark file ownership is not statically known.',
          );
        }
        scopes.push({
          ...scope,
          setupFiles: resolvedSetup?.filter((file): file is string => !!file),
          unresolvedSetup,
        });
      };
      if (!projects) {
        addScope();
        return;
      }
      if (projects.type !== 'ArrayExpression') {
        unknown();
        return;
      }
      const elements = projectElements(projects);
      const excluded = elements
        .filter(isString)
        .map((node) => node.value)
        .filter((value) => value.startsWith('!'))
        .map((value) => value.slice(1));
      for (const project of elements) {
        if (isString(project)) {
          if (project.value.startsWith('!')) {
            continue;
          }
          const reference = project.value.replaceAll('\\', '/').replace('<rootDir>', scope.root);
          const target = path.resolve(scope.root, reference);
          const candidates = new Set<string>();
          if (sources.has(target)) {
            candidates.add(target);
          }
          const directoryConfig = defaultConfig(sources, target);
          if (directoryConfig) {
            candidates.add(directoryConfig);
          }
          for (const file of configFiles) {
            if (
              matches(file, scope.root, reference) ||
              (matches(path.dirname(file), scope.root, reference) &&
                defaultConfig(sources, path.dirname(file)) === file)
            ) {
              candidates.add(file);
            }
          }
          if (!candidates.size) {
            unknown();
            continue;
          }
          for (const file of candidates) {
            if (
              excluded.some(
                (pattern) =>
                  matches(file, scope.root, pattern) ||
                  matches(path.dirname(file), scope.root, pattern),
              )
            ) {
              continue;
            }
            if (file === entry.file) {
              addScope();
            } else {
              queue.push({
                file,
                root: path.dirname(file),
                cwd: entry.cwd ?? entry.root,
                projects: entry.projects,
                browserOverride: entry.browserOverride,
              });
            }
          }
          continue;
        }
        if (!staticObject(project)) {
          unknown();
          continue;
        }
        const extendsValue = objectProperty(project, 'extends')?.value;
        if (extendsValue && !isBoolean(extendsValue)) {
          // A build-only base has no scope options to inherit. Otherwise its
          // dir or setupFiles may reach outside the inline root: retain review
          // instead of guessing that all inherited files live beneath it.
          const baseFile = isString(extendsValue) && path.resolve(scope.root, extendsValue.value);
          const baseSource = baseFile && sources.get(baseFile);
          if (baseSource) {
            const baseEditor = new SourceEditor(baseFile, baseSource);
            const declaration = baseEditor.ast.body.find(
              (node) => node.type === 'ExportDefaultDeclaration',
            );
            const baseObject = resolveConfigObject(baseEditor, declaration?.declaration);
            if (baseObject) {
              const baseTest = objectProperty(baseObject, 'test')?.value;
              // Compatibility defaults such as clearMocks do not change file
              // ownership. Accept them on later migration passes as well.
              if (
                !baseTest ||
                (staticObject(baseTest, true) &&
                  !SCOPE_KEYS.some((key) => objectProperty(baseTest, key)))
              ) {
                walk(project, scope.root, undefined, true);
                continue;
              }
            }
          }
          unknown();
          continue;
        }
        const inherits = isBoolean(extendsValue) ? extendsValue.value : !preserveV4;
        walk(project, scope.root, inherits ? scope : undefined, true);
      }
    };
    try {
      editor = new SourceEditor(entry.file, sources.get(entry.file)!);
      editor.visit({
        ExportDefaultDeclaration(node) {
          walk(node.declaration);
        },
        AssignmentExpression(node) {
          if (isModuleExports(editor, node.left)) {
            walk(node.right);
          }
        },
      });
      findings.push(...editor.findings);
    } catch {
      unknown();
    }
    if (!found) {
      unknown();
    }
  }
  const modes = new Map<string, VitestV5TestMode>(
    [...sources.keys()].map((file) => {
      const matching: Array<{ scope: TestScope; certain: boolean }> = [];
      let benchmarkFile = false;
      for (const scope of scopes) {
        if (scope.excludedRoots?.some((root) => insideDirectory(file, root))) {
          continue;
        }
        // Setup files execute in this project regardless of include/exclude.
        const setup = scope.setupFiles?.includes(file);
        const inside = insideDirectory(file, scope.root);
        const discoveryRoot = scope.discoveryRoot;
        const test =
          discoveryRoot &&
          insideDirectory(file, discoveryRoot) &&
          (scope.include ?? DEFAULT_TEST_INCLUDE).some((pattern) =>
            matches(file, discoveryRoot, pattern),
          ) &&
          !(scope.exclude ?? DEFAULT_TEST_EXCLUDE).some((pattern) =>
            matches(file, discoveryRoot, pattern),
          );
        const benchmark =
          discoveryRoot &&
          insideDirectory(file, discoveryRoot) &&
          scope.benchmark &&
          !(scope.benchmark.exclude ?? DEFAULT_BENCHMARK_EXCLUDE).some((pattern) =>
            matches(file, discoveryRoot, pattern),
          ) &&
          ((scope.benchmark.include ?? DEFAULT_BENCHMARK_INCLUDE).some((pattern) =>
            matches(file, discoveryRoot, pattern),
          ) ||
            (scope.benchmark.includeSource?.some((pattern) =>
              matches(file, discoveryRoot, pattern),
            ) &&
              sources.get(file)?.includes('import.meta.vitest')));
        benchmarkFile ||= !!benchmark;
        if (setup || test || benchmark) {
          matching.push({ scope, certain: true });
        } else if (
          ((!discoveryRoot || !scope.setupFiles || scope.unresolvedSetup) && inside) ||
          (!scope.benchmark && discoveryRoot && insideDirectory(file, discoveryRoot))
        ) {
          matching.push({ scope, certain: false });
        }
      }
      const browserModes = new Set(
        matching.map(({ scope, certain }) => (certain ? scope.browser : undefined)),
      );
      const globals =
        matching.length > 0 &&
        matching.every(({ scope, certain }) => certain && scope.globals === true);
      return [
        file,
        {
          benchmark: benchmarkFile,
          browser: browserModes.size === 1 ? browserModes.values().next().value : undefined,
          globals,
          reviewGlobals:
            !globals && matching.some(({ scope, certain }) => !certain || scope.globals !== false),
        },
      ];
    }),
  );
  return { modes, findings };
}
