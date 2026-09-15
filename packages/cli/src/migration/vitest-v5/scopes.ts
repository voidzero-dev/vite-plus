import path from 'node:path';

import type * as t from '@oxc-project/types';
import { minimatch } from 'minimatch';

import {
  CONFIG_SOURCES,
  SourceEditor,
  importedName,
  isBoolean,
  isString,
  objectProperty,
  propertyName,
  staticObject,
  type SourceOptions,
} from './ast.ts';
import { literalArgv } from './commands.ts';

interface ConfigEntry {
  file?: string;
  root: string;
  uncertain?: boolean;
  rootOverride?: string;
}

interface TestScope {
  root: string;
  include?: string[];
  exclude?: string[];
  setupFiles?: string[];
  browser?: boolean;
  globals?: boolean;
}

export type VitestV5TestMode = Pick<SourceOptions, 'browser' | 'globals' | 'reviewGlobals'>;

const DEFAULT_TEST_INCLUDE = ['**/*.{test,spec}.?(c|m)[jt]s?(x)'];
// Match Vitest's discovery order, including precedence across extensions.
const CONFIG_NAMES = ['vitest.config', 'vite.config'].flatMap((name) =>
  ['.ts', '.mts', '.cts', '.js', '.mjs', '.cjs'].map((extension) => name + extension),
);

function defaultConfig(sources: ReadonlyMap<string, string>, root: string) {
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
      entries.push({ root, file: defaultConfig(sources, root) });
      continue;
    }
    for (const command of commands) {
      const argv = literalArgv(command)?.map((token) => token.value);
      let start = 0;
      if (argv && ['pnpm', 'npm', 'yarn', 'bun', 'npx', 'bunx'].includes(argv[0])) {
        start++;
        if (['exec', 'x'].includes(argv[start])) {
          start++;
        }
        if (argv[start] === '--') {
          start++;
        }
      }
      if (argv?.[start] === 'vitest') {
        start++;
      } else if (argv?.[start] === 'vp' && argv[start + 1] === 'test') {
        start += 2;
      } else {
        entries.push({ root, uncertain: true });
        continue;
      }
      let config: string | undefined;
      let rootOverride: string | undefined;
      let uncertain = false;
      for (let index = start; index < argv.length && argv[index] !== '--'; index++) {
        const arg = argv[index];
        const option = arg.split('=')[0];
        if (['--config', '-c', '--root', '-r'].includes(option)) {
          const value = arg.includes('=') ? arg.slice(arg.indexOf('=') + 1) : argv[++index];
          if (!value || value.startsWith('-')) {
            uncertain = true;
          } else if (option === '--config' || option === '-c') {
            config = value;
          } else {
            rootOverride = path.resolve(root, value);
          }
        } else if (
          /^--(?:no-)?(?:globals|include|exclude|project|workspace|setupFiles)(?:[.=]|$)/.test(arg)
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
        uncertain: uncertain || (!!config && !sources.has(file!)),
      });
    }
  }
  return entries;
}

function staticPatterns(node: t.Node | undefined): string[] | undefined {
  return node?.type === 'ArrayExpression' &&
    node.elements.every((entry) => isString(entry) && !entry.value.startsWith('!'))
    ? node.elements.map((entry) => (entry as t.StringLiteral).value)
    : undefined;
}

function matches(file: string, root: string, pattern: string) {
  return minimatch(path.relative(root, file).replaceAll('\\', '/'), pattern.replace(/^\.\//, ''), {
    dot: true,
  });
}

/** Resolve only selected configs and their project references. Unknown config
 * or conflicting project globals requires review, not silent suppression. */
export function resolveVitestV5TestModes(
  sources: ReadonlyMap<string, string>,
  configFiles: ReadonlySet<string>,
  preserveV4: boolean,
  entries: ConfigEntry[],
): Map<string, VitestV5TestMode> {
  const scopes: TestScope[] = [];
  const queue = [...entries];
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
    const visited = new Set<t.ObjectExpression>();
    const walk = (
      object: t.Node | undefined,
      directory = entry.root,
      base?: TestScope,
      inline = false,
    ) => {
      found = true;
      if (!staticObject(object)) {
        unknown();
        return;
      }
      if (visited.has(object)) {
        return;
      }
      visited.add(object);
      const test = objectProperty(object, 'test')?.value;
      if (test && !staticObject(test)) {
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
      const include = test && objectProperty(test, 'include')?.value;
      const exclude = test && objectProperty(test, 'exclude')?.value;
      if ((include && !staticPatterns(include)) || (exclude && !staticPatterns(exclude))) {
        unknown();
        return;
      }
      const browser = test && objectProperty(test, 'browser')?.value;
      const enabled = staticObject(browser) ? objectProperty(browser, 'enabled')?.value : undefined;
      let browserMode = base ? base.browser : false;
      if (browser) {
        browserMode =
          !staticObject(browser) || (enabled && !isBoolean(enabled))
            ? undefined
            : isBoolean(enabled)
              ? enabled.value
              : browserMode;
      }
      const globals = test && objectProperty(test, 'globals')?.value;
      const setup = test && objectProperty(test, 'setupFiles')?.value;
      const setupFiles = setup ? (isString(setup) ? [setup.value] : staticPatterns(setup)) : [];
      const scope: TestScope = {
        root: resolvedRoot,
        include: include
          ? [...(base?.include ?? []), ...staticPatterns(include)!]
          : (base?.include ?? DEFAULT_TEST_INCLUDE),
        exclude: exclude ? [...(base?.exclude ?? []), ...staticPatterns(exclude)!] : base?.exclude,
        setupFiles:
          setupFiles && (!base || base.setupFiles)
            ? [...(base?.setupFiles ?? []), ...setupFiles]
            : undefined,
        browser: browserMode,
        globals: globals
          ? isBoolean(globals)
            ? globals.value
            : undefined
          : base
            ? base.globals
            : false,
      };
      const projects = test && objectProperty(test, 'projects')?.value;
      if (!projects) {
        scopes.push(scope);
        return;
      }
      if (projects.type !== 'ArrayExpression') {
        unknown();
        return;
      }
      const excluded = projects.elements
        .filter(isString)
        .map((node) => node.value)
        .filter((value) => value.startsWith('!'))
        .map((value) => value.slice(1));
      for (const project of projects.elements) {
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
              scopes.push(scope);
            } else {
              queue.push({ file, root: path.dirname(file) });
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
          unknown();
          continue;
        }
        const inherits = isBoolean(extendsValue) ? extendsValue.value : !preserveV4;
        walk(project, scope.root, inherits ? scope : undefined, true);
      }
    };
    try {
      const editor = new SourceEditor(entry.file, sources.get(entry.file)!);
      const exported = (node: t.Node | undefined) => {
        if (node?.type === 'Identifier') {
          const binding = editor.binding(node);
          node =
            binding?.constant && binding.declaration.type === 'VariableDeclarator'
              ? (binding.declaration.init ?? undefined)
              : undefined;
        }
        if (node?.type === 'CallExpression') {
          node = ['defineConfig', 'defineProject'].includes(
            importedName(editor, node.callee, CONFIG_SOURCES) ?? '',
          )
            ? node.arguments[0]
            : undefined;
        }
        walk(node);
      };
      editor.visit({
        ExportDefaultDeclaration(node) {
          exported(node.declaration);
        },
        AssignmentExpression(node) {
          if (
            node.left.type === 'MemberExpression' &&
            !node.left.computed &&
            node.left.object.type === 'Identifier' &&
            node.left.object.name === 'module' &&
            propertyName(node.left.property) === 'exports' &&
            !editor.binding(node.left.object)
          ) {
            exported(node.right);
          }
        },
      });
    } catch {
      unknown();
    }
    if (!found) {
      unknown();
    }
  }
  return new Map(
    [...sources.keys()].map((file) => {
      const matching: Array<{ scope: TestScope; certain: boolean }> = [];
      for (const scope of scopes) {
        // Setup files execute in this project regardless of include/exclude.
        const setup = scope.setupFiles?.some((reference) => {
          const target = path.resolve(scope.root, reference);
          const resolved = [
            target,
            ...['.js', '.ts', '.mjs', '.mts', '.cjs', '.cts'].map(
              (extension) => target + extension,
            ),
          ].find((candidate) => sources.has(candidate));
          return resolved === file;
        });
        const relative = path.relative(scope.root, file);
        const inside = !relative.startsWith(`..${path.sep}`) && !path.isAbsolute(relative);
        const test =
          inside &&
          (!scope.include || scope.include.some((pattern) => matches(file, scope.root, pattern))) &&
          !scope.exclude?.some((pattern) => matches(file, scope.root, pattern));
        if (setup || test) {
          matching.push({ scope, certain: !!setup || !!scope.include });
        } else if (!scope.setupFiles && inside) {
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
          browser: browserModes.size === 1 ? browserModes.values().next().value : undefined,
          globals,
          reviewGlobals:
            !globals && matching.some(({ scope, certain }) => !certain || scope.globals !== false),
        },
      ];
    }),
  );
}
