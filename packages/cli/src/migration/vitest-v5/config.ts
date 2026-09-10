import path from 'node:path';

import type * as t from '@babel/types';
import { minimatch } from 'minimatch';

import {
  CONFIG_SOURCES,
  SourceEditor,
  importedName,
  objectProperty,
  propertyName,
  staticObject,
  traverse,
  type SourceOptions,
} from './ast.ts';

const isTrue = (node: t.Node | undefined) => node?.type === 'BooleanLiteral' && node.value;

const CONFIG_FILENAME = /(?:^|[/\\])(?:vite|vitest)(?:\.[\w-]+)?\.config\.[cm]?[jt]s$/;
const CONFIG_HELPER_IMPORT = /['"](?:vitest\/config|vite-plus(?:\/test\/config)?)['"]/;

function importsConfigHelper(file: string, source: string): boolean {
  if (!CONFIG_HELPER_IMPORT.test(source)) {
    return false;
  }
  try {
    const editor = new SourceEditor(file, source);
    return editor.ast.program.body.some(
      (node) =>
        node.type === 'ImportDeclaration' &&
        node.importKind !== 'type' &&
        CONFIG_SOURCES.has(node.source.value) &&
        node.specifiers.some(
          (specifier) =>
            specifier.type === 'ImportNamespaceSpecifier' ||
            (specifier.type === 'ImportSpecifier' &&
              specifier.importKind !== 'type' &&
              ['defineConfig', 'defineProject', 'mergeConfig'].includes(
                propertyName(specifier.imported) ?? '',
              )),
        ),
    );
  } catch {
    // Keep candidate configs in the normal parse-diagnostic path.
    return true;
  }
}

/** Include literal referenced configs even when they use a custom filename and
 * export a raw object. Candidate files come from the owning package's scan. */
export function findVitestV5ConfigFiles(sources: ReadonlyMap<string, string>): Set<string> {
  const files = new Set(
    [...sources]
      .filter(
        ([file, source]) =>
          /\.[cm]?[jt]sx?$/.test(file) &&
          (CONFIG_FILENAME.test(file) || importsConfigHelper(file, source)),
      )
      .map(([file]) => file),
  );
  for (const file of files) {
    try {
      const editor = new SourceEditor(file, sources.get(file)!);
      const addReference = (reference: string) => {
        if (reference.startsWith('!')) {
          return;
        }
        const target = path.resolve(path.dirname(file), reference);
        if (sources.has(target) && /\.[cm]?[jt]sx?$/.test(target)) {
          files.add(target);
        } else {
          for (const candidate of sources.keys()) {
            const relative = path.relative(path.dirname(file), candidate).replaceAll('\\', '/');
            const pattern = reference.replaceAll('\\', '/').replace(/^\.\//, '');
            if (
              /\.[cm]?[jt]sx?$/.test(candidate) &&
              (minimatch(relative, pattern, { dot: true }) ||
                (CONFIG_FILENAME.test(candidate) &&
                  minimatch(path.posix.dirname(relative), pattern, { dot: true })))
            ) {
              files.add(candidate);
            }
          }
        }
      };
      const collectReferences = (config: t.Node | undefined) => {
        if (config?.type !== 'ObjectExpression') {
          return;
        }
        const base = objectProperty(config, 'extends')?.value;
        if (base?.type === 'StringLiteral') {
          addReference(base.value);
        }
        const test = objectProperty(config, 'test')?.value;
        const projects =
          test?.type === 'ObjectExpression' ? objectProperty(test, 'projects')?.value : undefined;
        if (projects?.type !== 'ArrayExpression') {
          return;
        }
        for (const project of projects.elements) {
          if (project?.type === 'StringLiteral') {
            addReference(project.value);
          } else {
            collectReferences(project ?? undefined);
          }
        }
      };
      traverse(editor.ast, {
        ExportDefaultDeclaration(p) {
          const declaration = p.node.declaration;
          if (declaration.type === 'Identifier') {
            const binding = p.scope.getBinding(declaration.name);
            if (binding?.constant && binding.path.isVariableDeclarator()) {
              collectReferences(binding.path.node.init ?? undefined);
            }
          } else {
            collectReferences(declaration);
          }
        },
        CallExpression(p) {
          if (
            ['defineConfig', 'defineProject', 'mergeConfig'].includes(
              importedName(p, p.node.callee, CONFIG_SOURCES) ?? '',
            )
          ) {
            for (const argument of p.node.arguments) {
              collectReferences(argument);
            }
          }
        },
        AssignmentExpression(p) {
          if (
            p.node.left.type === 'MemberExpression' &&
            !p.node.left.computed &&
            p.node.left.object.type === 'Identifier' &&
            p.node.left.object.name === 'module' &&
            propertyName(p.node.left.property) === 'exports' &&
            !p.scope.getBinding('module')
          ) {
            collectReferences(p.node.right);
          }
        },
      });
    } catch {
      // The migration's normal parse diagnostic owns reporting this file.
    }
  }
  return files;
}

function hasConfigMerge(editor: SourceEditor): boolean {
  let found = false;
  traverse(editor.ast, {
    CallExpression(p) {
      if (importedName(p, p.node.callee, CONFIG_SOURCES) === 'mergeConfig') {
        found = true;
      }
    },
  });
  return found;
}

/** Defaults on one merge fragment can override explicit settings in another.
 * Include local imported configs so the preflight and finalization agree. */
export function findVitestV5MergedConfigFiles(
  sources: ReadonlyMap<string, string>,
  configFiles: ReadonlySet<string>,
): Set<string> {
  const merged = new Set<string>();
  const imports = new Map<string, string[]>();
  const extensions = ['.ts', '.mts', '.cts', '.js', '.mjs', '.cjs', '.tsx', '.jsx'];
  for (const file of configFiles) {
    try {
      const editor = new SourceEditor(file, sources.get(file)!);
      if (hasConfigMerge(editor)) {
        merged.add(file);
      }
      const dependencies: string[] = [];
      for (const node of editor.ast.program.body) {
        if (
          node.type !== 'ImportDeclaration' &&
          node.type !== 'ExportNamedDeclaration' &&
          node.type !== 'ExportAllDeclaration'
        ) {
          continue;
        }
        const reference = node.source?.value;
        if (!reference?.startsWith('.')) {
          continue;
        }
        const target = path.resolve(path.dirname(file), reference);
        const dependency = [
          target,
          target.replace(/\.([cm]?)js$/, '.$1ts'),
          ...extensions.map((extension) => `${target}${extension}`),
          ...extensions.map((extension) => path.join(target, `index${extension}`)),
        ].find((candidate) => configFiles.has(candidate));
        if (dependency) {
          dependencies.push(dependency);
        }
      }
      imports.set(file, dependencies);
    } catch {
      // The normal config pass reports unsupported syntax.
    }
  }
  for (const file of merged) {
    for (const dependency of imports.get(file) ?? []) {
      merged.add(dependency);
    }
  }
  return merged;
}

interface BrowserTestScope {
  root?: string;
  include?: string[];
  exclude?: string[];
  browser?: boolean;
}

const DEFAULT_TEST_INCLUDE = ['**/*.{test,spec}.?(c|m)[jt]s?(x)'];

function staticPatterns(node: t.Node | undefined): string[] | undefined {
  return node?.type === 'ArrayExpression' &&
    node.elements.every((entry) => entry?.type === 'StringLiteral' && !entry.value.startsWith('!'))
    ? node.elements.map((entry) => (entry as t.StringLiteral).value)
    : undefined;
}

/** Resolve only literal project ownership. A file shared by Node and browser
 * projects, or covered by dynamic config, needs review before a matcher rename. */
export function resolveVitestV5BrowserModes(
  sources: ReadonlyMap<string, string>,
  configFiles: ReadonlySet<string>,
  preserveV4: boolean,
): Map<string, boolean | undefined> {
  const scopes: BrowserTestScope[] = [];

  for (const file of configFiles) {
    let found = false;
    const visited = new Set<t.ObjectExpression>();
    const unknown = () => {
      found = true;
      scopes.push({});
    };
    const walk = (object: t.Node | undefined, base?: BrowserTestScope) => {
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
      const root = objectProperty(object, 'root')?.value;
      const testRoot = test && objectProperty(test, 'root')?.value;
      if (
        (root && root.type !== 'StringLiteral') ||
        (testRoot && testRoot.type !== 'StringLiteral')
      ) {
        unknown();
        return;
      }
      const directory = path.resolve(
        path.dirname(file),
        root?.type === 'StringLiteral' ? root.value : (base?.root ?? '.'),
        testRoot?.type === 'StringLiteral' ? testRoot.value : '.',
      );
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
        if (!staticObject(browser) || (enabled && enabled.type !== 'BooleanLiteral')) {
          browserMode = undefined;
        } else if (enabled?.type === 'BooleanLiteral') {
          browserMode = enabled.value;
        }
      }
      const scope: BrowserTestScope = {
        root: directory,
        include: include ? staticPatterns(include) : (base?.include ?? DEFAULT_TEST_INCLUDE),
        exclude: exclude ? staticPatterns(exclude) : base?.exclude,
        browser: browserMode,
      };
      // Inherited arrays can merge. Preserve all possible includes and avoid
      // assuming that an exclusion removes a file from both configurations.
      if (base && include) {
        scope.include = base.include ? [...base.include, ...scope.include!] : undefined;
        scope.exclude = [];
      }
      const projects = test && objectProperty(test, 'projects')?.value;
      if (!projects) {
        scopes.push(scope);
        return;
      }
      if (projects.type !== 'ArrayExpression') {
        unknown();
        return;
      }
      for (const project of projects.elements) {
        if (project?.type === 'StringLiteral') {
          // Referenced config files have their own scope in configFiles.
          if (!project.value.startsWith('!')) {
            const pattern = project.value.replaceAll('\\', '/').replace(/^\.\//, '');
            const referenced = [...configFiles].some((candidate) => {
              const relative = path.relative(path.dirname(file), candidate).replaceAll('\\', '/');
              return (
                minimatch(relative, pattern, { dot: true }) ||
                minimatch(path.posix.dirname(relative), pattern, { dot: true })
              );
            });
            if (!referenced) {
              unknown();
            }
          }
          continue;
        }
        if (!staticObject(project)) {
          unknown();
          continue;
        }
        const extendsValue = objectProperty(project, 'extends')?.value;
        if (extendsValue && extendsValue.type !== 'BooleanLiteral') {
          unknown();
          continue;
        }
        const inherits = extendsValue?.type === 'BooleanLiteral' ? extendsValue.value : !preserveV4;
        walk(project, inherits ? scope : undefined);
      }
    };
    try {
      const editor = new SourceEditor(file, sources.get(file)!);
      traverse(editor.ast, {
        ExportDefaultDeclaration(p) {
          const declaration = p.node.declaration;
          if (declaration.type === 'CallExpression') {
            if (
              !['defineConfig', 'defineProject'].includes(
                importedName(p, declaration.callee, CONFIG_SOURCES) ?? '',
              )
            ) {
              unknown();
            }
            return;
          }
          if (declaration.type === 'Identifier') {
            const binding = p.scope.getBinding(declaration.name);
            const initializer =
              binding?.constant && binding.path.isVariableDeclarator()
                ? binding.path.node.init
                : undefined;
            if (
              initializer?.type === 'CallExpression' &&
              ['defineConfig', 'defineProject'].includes(
                importedName(binding!.path, initializer.callee, CONFIG_SOURCES) ?? '',
              )
            ) {
              walk(initializer.arguments[0]);
            } else {
              walk(initializer ?? undefined);
            }
          } else {
            walk(declaration);
          }
        },
        CallExpression(p) {
          if (
            ['defineConfig', 'defineProject'].includes(
              importedName(p, p.node.callee, CONFIG_SOURCES) ?? '',
            )
          ) {
            walk(p.node.arguments[0]);
          }
        },
        AssignmentExpression(p) {
          if (
            p.node.left.type === 'MemberExpression' &&
            !p.node.left.computed &&
            p.node.left.object.type === 'Identifier' &&
            p.node.left.object.name === 'module' &&
            propertyName(p.node.left.property) === 'exports' &&
            !p.scope.getBinding('module')
          ) {
            if (p.node.right.type === 'CallExpression') {
              if (
                !['defineConfig', 'defineProject'].includes(
                  importedName(p, p.node.right.callee, CONFIG_SOURCES) ?? '',
                )
              ) {
                unknown();
              }
            } else {
              walk(p.node.right);
            }
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
      const matching = scopes.filter((scope) => {
        if (!scope.root) {
          return true;
        }
        const relative = path.relative(scope.root, file).replaceAll('\\', '/');
        if (relative.startsWith('../') || path.isAbsolute(relative)) {
          return false;
        }
        return (
          (!scope.include ||
            scope.include.some((pattern) => minimatch(relative, pattern, { dot: true }))) &&
          !scope.exclude?.some((pattern) => minimatch(relative, pattern, { dot: true }))
        );
      });
      const modes = new Set(matching.map((scope) => scope.browser));
      return [file, modes.size === 1 ? matching[0].browser : undefined];
    }),
  );
}

export function migrateVitestV5Config(
  file: string,
  source: string,
  options: SourceOptions,
  mergedConfig = false,
) {
  const editor = new SourceEditor(file, source);
  const visited = new Set<t.ObjectExpression>();
  const merged = mergedConfig || hasConfigMerge(editor);
  const preserveDefaults = options.preserveV4 && !merged;
  if (merged && (options.preserveV4 || options.reviewV4)) {
    editor.report(
      undefined,
      'merged-config-defaults',
      'Review the effective merged config before adding v4 defaults for clearMocks, browser locators, fake timers, reporters, and projects. Defaults were not added to config fragments because they can override explicit settings in another fragment.',
    );
  }

  function nested(
    object: t.ObjectExpression,
    key: string,
    additions: string,
    apply: (node: t.ObjectExpression) => void,
  ) {
    const prop = objectProperty(object, key);
    if (!prop) {
      editor.add(object, key, `{ ${additions} }`);
    } else if (staticObject(prop.value)) {
      apply(prop.value);
    } else {
      editor.report(
        prop,
        'dynamic-config',
        `Resolve ${key} before applying Vitest v5 compatibility options.`,
      );
    }
  }

  function reporters(test: t.ObjectExpression) {
    const prop = objectProperty(test, 'reporters');
    if (!prop) {
      return;
    }
    const output = objectProperty(test, 'outputFile');
    const entries = prop.value.type === 'ArrayExpression' ? prop.value.elements : [prop.value];
    for (const entry of entries) {
      const name = entry?.type === 'ArrayExpression' ? entry.elements[0] : entry;
      if (name?.type !== 'StringLiteral') {
        editor.report(
          entry ?? prop,
          'reporter-options',
          'Review dynamic reporter options and JSON/JUnit stdout consumers.',
        );
        continue;
      }
      const opts = entry?.type === 'ArrayExpression' ? entry.elements[1] : undefined;
      if (name.value === 'html' && staticObject(opts)) {
        const old = objectProperty(opts, 'outputFile');
        if (old) {
          if (objectProperty(opts, 'outputDir')) {
            editor.report(
              old,
              'html-output',
              'Review HTML outputFile and outputDir; both are configured.',
            );
          } else if (
            old.value.type === 'StringLiteral' &&
            /(^|[/\\])index\.html$/.test(old.value.value)
          ) {
            editor.replace(old.key, 'outputDir');
            editor.replace(
              old.value,
              JSON.stringify(path.posix.dirname(old.value.value.replaceAll('\\', '/'))),
            );
          } else {
            editor.report(
              old,
              'html-output',
              'Replace the HTML outputFile with an outputDir. The v5 file name is index.html.',
            );
          }
        }
      }
      if (!preserveDefaults || !['json', 'junit'].includes(name.value)) {
        continue;
      }
      if (output && (!staticObject(output.value) || objectProperty(output.value, name.value))) {
        continue;
      }
      if (opts && !staticObject(opts)) {
        editor.report(
          opts,
          'reporter-options',
          `Review ${name.value} output: v5 writes a file unless stdout is true.`,
        );
      } else if (staticObject(opts)) {
        if (!objectProperty(opts, 'outputFile')) {
          editor.add(opts, 'stdout', 'true');
        }
      } else {
        const tuple = `[${editor.text(name)}, { stdout: true }]`;
        editor.replace(entry!, prop.value.type === 'ArrayExpression' ? tuple : `[${tuple}]`);
      }
    }
    if (
      output &&
      entries.some((entry) => entry?.type === 'StringLiteral' && entry.value === 'html')
    ) {
      editor.report(
        output,
        'html-output',
        'Move the HTML outputFile into reporter outputDir options; retain other reporter output files.',
      );
    }
  }

  function testOptions(
    test: t.ObjectExpression,
    inherits: boolean,
    parentTest?: t.ObjectExpression,
  ) {
    if (preserveDefaults && !inherits) {
      editor.add(test, 'clearMocks', 'false');
    }
    const browser = objectProperty(test, 'browser');
    if (browser && staticObject(browser.value)) {
      const value = browser.value;
      // An inherited browser config already receives its defaults at the parent.
      // Do not replace its explicit or dynamic locator setting in the child.
      const inheritsBrowser = inherits && parentTest && objectProperty(parentTest, 'browser');
      if (preserveDefaults && !inheritsBrowser) {
        nested(value, 'locators', 'exact: false', (locators) =>
          editor.add(locators, 'exact', 'false'),
        );
      }
      const api = objectProperty(value, 'api');
      if (api) {
        const target = objectProperty(test, 'api');
        if (target && editor.text(target.value) !== editor.text(api.value)) {
          editor.report(
            api,
            'api-conflict',
            'Resolve conflicting test.api and test.browser.api values before upgrading.',
            'block',
          );
        } else {
          editor.add(test, 'api', editor.text(api.value));
          editor.remove(value, api);
        }
      }
      const screenshot = objectProperty(value, 'screenshotDirectory');
      if (screenshot) {
        const setting = `screenshotDirectory: ${editor.text(screenshot.value)}`;
        nested(value, 'expect', `toMatchScreenshot: { ${setting} }`, (expect) => {
          nested(expect, 'toMatchScreenshot', setting, (match) =>
            editor.add(match, 'screenshotDirectory', editor.text(screenshot.value)),
          );
        });
      }
      if (objectProperty(value, 'commands')) {
        editor.report(
          browser,
          'locator-commands',
          'Review browser command parameters: locators are SerializedLocator objects, not selector strings.',
        );
      }
    } else if (browser) {
      editor.report(
        browser,
        'dynamic-project',
        'Review dynamic browser options and preserve locators.exact: false when migrating v4.',
      );
    }

    if (preserveDefaults && options.temporalPolyfill) {
      nested(test, 'fakeTimers', "toNotFake: ['Temporal']", (timers) =>
        editor.add(timers, 'toNotFake', "['Temporal']"),
      );
    }
    const coverage = objectProperty(test, 'coverage');
    if (coverage && staticObject(coverage.value)) {
      for (const key of ['include', 'exclude']) {
        const pattern = objectProperty(coverage.value, key);
        if (pattern) {
          editor.report(
            pattern,
            'coverage-patterns',
            `Compare v4/v5 resolved coverage.${key} file sets; v5 matches relative paths more precisely.`,
          );
        }
      }
      const thresholds = objectProperty(coverage.value, 'thresholds');
      if (
        preserveDefaults &&
        thresholds &&
        staticObject(thresholds.value) &&
        isTrue(objectProperty(thresholds.value, 'perFile')?.value)
      ) {
        for (const threshold of thresholds.value.properties) {
          if (
            threshold.type !== 'ObjectProperty' ||
            [
              'perFile',
              'lines',
              'branches',
              'functions',
              'statements',
              'autoUpdate',
              '100',
            ].includes(propertyName(threshold.key) ?? '')
          ) {
            continue;
          }
          if (staticObject(threshold.value)) {
            editor.add(threshold.value, 'perFile', 'true');
          } else {
            editor.report(
              threshold,
              'coverage-thresholds',
              'Set perFile: true on this dynamic glob threshold to retain v4 enforcement.',
            );
          }
        }
      }
    }
    reporters(test);
    const benchmark = objectProperty(test, 'benchmark');
    if (benchmark) {
      if (staticObject(benchmark.value)) {
        for (const key of ['reporters', 'outputFile', 'compare', 'outputJson']) {
          const removed = objectProperty(benchmark.value, key);
          if (removed) {
            editor.report(
              removed,
              'benchmark-api',
              `Replace removed benchmark.${key} with regular test reporters and the bench test-context fixture.`,
              'block',
            );
          }
        }
      } else {
        editor.report(
          benchmark,
          'benchmark-api',
          'Review dynamic benchmark options for removed reporters, outputFile, compare, and outputJson settings.',
        );
      }
    }

    const projects = objectProperty(test, 'projects');
    if (!projects) {
      return;
    }
    if (projects.value.type !== 'ArrayExpression') {
      editor.report(
        projects,
        'dynamic-project',
        'Resolve dynamic projects and review inheritance and sharedViteServer before upgrading.',
      );
      return;
    }
    const hasInline = projects.value.elements.some((item) => item && item.type !== 'StringLiteral');
    if (hasInline && preserveDefaults) {
      editor.add(test, 'sharedViteServer', 'false');
    }
    for (const project of projects.value.elements) {
      if (!project || project.type === 'StringLiteral') {
        continue;
      }
      if (!staticObject(project)) {
        editor.report(
          project,
          'dynamic-project',
          'Review this function, promise, or dynamic inline project. Set explicit inheritance and v4 compatibility options.',
        );
        continue;
      }
      const extendsValue = objectProperty(project, 'extends');
      if (preserveDefaults) {
        editor.add(project, 'extends', 'false');
      }
      config(project, isTrue(extendsValue?.value), test);
    }
  }

  function config(object: t.ObjectExpression, inherits = false, parentTest?: t.ObjectExpression) {
    if (visited.has(object)) {
      return;
    }
    visited.add(object);
    const test = objectProperty(object, 'test');
    if (!test) {
      if (preserveDefaults && !inherits) {
        editor.add(
          object,
          'test',
          `{ clearMocks: false${options.temporalPolyfill ? ", fakeTimers: { toNotFake: ['Temporal'] }" : ''} }`,
        );
      }
    } else if (staticObject(test.value)) {
      testOptions(test.value, inherits, parentTest);
    } else {
      editor.report(
        test,
        'dynamic-config',
        'Review dynamic test options; v4 defaults cannot be preserved automatically.',
      );
    }
  }

  traverse(editor.ast, {
    AssignmentExpression(p) {
      const left = p.node.left;
      if (
        left.type === 'MemberExpression' &&
        !left.computed &&
        left.object.type === 'Identifier' &&
        left.object.name === 'module' &&
        propertyName(left.property) === 'exports' &&
        !p.scope.getBinding('module')
      ) {
        if (staticObject(p.node.right)) {
          config(p.node.right);
        } else if (p.node.right.type !== 'CallExpression') {
          editor.report(
            p.node,
            'dynamic-config',
            'Review the effective CommonJS test config and its v4 defaults.',
          );
        }
      }
    },
    ExportDefaultDeclaration(p) {
      if (staticObject(p.node.declaration)) {
        config(p.node.declaration);
      } else if (p.node.declaration.type === 'Identifier') {
        const binding = p.scope.getBinding(p.node.declaration.name);
        if (
          binding?.constant &&
          binding.path.isVariableDeclarator() &&
          staticObject(binding.path.node.init)
        ) {
          config(binding.path.node.init);
        } else {
          editor.report(
            p.node,
            'dynamic-config',
            'Review the effective exported test config and its v4 defaults.',
          );
        }
      } else if (p.node.declaration.type !== 'CallExpression') {
        editor.report(
          p.node,
          'dynamic-config',
          'Review the effective function or promise config and its v4 defaults.',
        );
      }
    },
    CallExpression(p) {
      const name = importedName(p, p.node.callee, CONFIG_SOURCES);
      if (name === 'defineConfig' || name === 'defineProject') {
        const object = p.node.arguments[0];
        if (staticObject(object)) {
          config(object);
        } else {
          editor.report(
            object,
            'dynamic-config',
            'Review this dynamic config; explicit v4 compatibility settings need manual insertion.',
          );
        }
      }
      if (name === 'mergeConfig') {
        editor.report(
          p.node,
          'nested-project-merge',
          'Check merged root configs for test.projects; v5 permits nested projects and can recurse or duplicate them.',
        );
      }
    },
    ObjectMethod(p) {
      if (
        ['config', 'configResolved', 'configureServer', 'configureVitest'].includes(
          propertyName(p.node.key) ?? '',
        )
      ) {
        editor.report(
          p.node,
          'project-server-lifecycle',
          'Review this plugin hook for per-project config execution or Vite server state.',
        );
      }
    },
    ObjectProperty(p) {
      if (
        ['config', 'configResolved', 'configureServer', 'configureVitest'].includes(
          propertyName(p.node.key) ?? '',
        )
      ) {
        editor.report(
          p.node,
          'project-server-lifecycle',
          'Review this plugin hook for per-project config execution or Vite server state.',
        );
      }
    },
  });
  return editor.finish();
}
