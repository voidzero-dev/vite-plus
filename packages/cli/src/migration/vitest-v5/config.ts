import path from 'node:path';

import type * as t from '@oxc-project/types';
import { minimatch } from 'minimatch';

import {
  CONFIG_SOURCES,
  SourceEditor,
  importedName,
  objectProperty,
  propertyName,
  staticObject,
  sameStaticValue,
  isString,
  isBoolean,
  isModuleExports,
  printAddedObject,
  type AddedProperty,
  type RewriteResult,
  type SourceOptions,
} from './ast.ts';
import { migrateBenchmarkConfig } from './benchmark-config.ts';
import { compatibilityProperty } from './compatibility.ts';

function isTrue(node: t.Node | undefined): boolean {
  return isBoolean(node) && node.value;
}

const CONFIG_FILENAME = /(?:^|[/\\])(?:vite|vitest)(?:\.[\w-]+)?\.config\.[cm]?[jt]s$/;
const CONFIG_HELPER_IMPORT = /['"](?:vite|vitest\/config|vite-plus(?:\/test\/config)?)['"]/;

function hasTestConfigHelper(file: string, source: string): boolean {
  if (!CONFIG_HELPER_IMPORT.test(source)) {
    return false;
  }
  try {
    const editor = new SourceEditor(file, source);
    const importsTestHelper = editor.ast.body.some(
      (node) =>
        node.type === 'ImportDeclaration' &&
        node.importKind !== 'type' &&
        ['vitest/config', 'vite-plus/test/config'].includes(node.source.value) &&
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
    if (importsTestHelper) {
      return true;
    }
    // Vite helpers also appear in plugin source and examples. A generic helper
    // import alone does not make that file a test config. Require explicit test
    // options; standard filenames and project references are handled separately.
    let hasTestOptions = false;
    editor.visit({
      CallExpression(node) {
        if (
          ['defineConfig', 'defineProject', 'mergeConfig'].includes(
            importedName(editor, node.callee, CONFIG_SOURCES) ?? '',
          ) &&
          node.arguments.some(
            (argument) =>
              argument.type === 'ObjectExpression' && !!objectProperty(argument, 'test'),
          )
        ) {
          hasTestOptions = true;
        }
      },
    });
    return hasTestOptions;
  } catch {
    // Keep candidate configs in the normal parse-diagnostic path.
    return true;
  }
}

/** Include literal referenced configs even when they use a custom filename and
 * export a raw object. Candidate files come from the owning package's scan. */
export function findVitestV5ConfigFiles(
  sources: ReadonlyMap<string, string>,
  entries: string[] = [],
): Set<string> {
  const files = new Set(
    [...sources]
      .filter(
        ([file, source]) =>
          /\.[cm]?[jt]sx?$/.test(file) &&
          (CONFIG_FILENAME.test(file) || hasTestConfigHelper(file, source)),
      )
      .map(([file]) => file),
  );
  for (const file of entries) {
    if (sources.has(file)) {
      files.add(file);
    }
  }
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
        if (isString(base)) {
          addReference(base.value);
        }
        const test = objectProperty(config, 'test')?.value;
        const projects =
          test?.type === 'ObjectExpression' ? objectProperty(test, 'projects')?.value : undefined;
        if (projects?.type !== 'ArrayExpression') {
          return;
        }
        for (const project of projects.elements) {
          if (isString(project)) {
            addReference(project.value);
          } else {
            collectReferences(project ?? undefined);
          }
        }
      };
      editor.visit({
        ExportDefaultDeclaration(node) {
          const declaration = node.declaration;
          if (declaration.type === 'Identifier') {
            const binding = editor.binding(declaration);
            if (binding?.constant && binding.declaration.type === 'VariableDeclarator') {
              collectReferences(binding.declaration.init ?? undefined);
            }
          } else {
            collectReferences(declaration);
          }
        },
        CallExpression(node) {
          if (
            ['defineConfig', 'defineProject', 'mergeConfig'].includes(
              importedName(editor, node.callee, CONFIG_SOURCES) ?? '',
            )
          ) {
            for (const argument of node.arguments) {
              collectReferences(argument);
            }
          }
        },
        AssignmentExpression(node) {
          if (isModuleExports(editor, node.left)) {
            collectReferences(node.right);
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
  editor.visit({
    CallExpression(node) {
      if (importedName(editor, node.callee, CONFIG_SOURCES) === 'mergeConfig') {
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
      for (const node of editor.ast.body) {
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

export function migrateVitestV5Config(
  file: string,
  source: string,
  options: SourceOptions,
  mergedConfig = false,
): RewriteResult {
  // Resolve output moves first so reporter/default edits use fresh offsets and
  // see the new JSON destinations rather than accidentally enabling stdout.
  const output = migrateConfig(file, source, options, mergedConfig, true);
  const result = migrateConfig(file, output.content, options, mergedConfig);
  return {
    content: result.content,
    findings: [
      ...output.findings.filter(({ code }) =>
        ['benchmark-output', 'overlapping-edits', 'unsafe-syntax'].includes(code),
      ),
      ...result.findings,
    ],
  };
}

function migrateConfig(
  file: string,
  source: string,
  options: SourceOptions,
  mergedConfig: boolean,
  benchmarkOnly = false,
): RewriteResult {
  const editor = new SourceEditor(file, source);
  const visited = new Set<t.ObjectExpression>();
  const merged = mergedConfig || hasConfigMerge(editor);
  const preserveDefaults = options.preserveV4 && !merged && !benchmarkOnly;
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
    additions: string | AddedProperty[],
    apply: (node: t.ObjectExpression) => void,
  ) {
    const prop = objectProperty(object, key);
    if (!prop) {
      editor.add(object, key, typeof additions === 'string' ? `{ ${additions} }` : additions);
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

  function addCompatibility(
    object: t.ObjectExpression,
    key: Parameters<typeof compatibilityProperty>[0],
  ) {
    const property = compatibilityProperty(key);
    editor.add(object, key, property.value, property.comment);
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
      if (!isString(name)) {
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
          } else if (isString(old.value) && /(^|[/\\])index\.html$/.test(old.value.value)) {
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
          addCompatibility(opts, 'stdout');
        }
      } else {
        const indent =
          source
            .slice(source.lastIndexOf('\n', entry!.start) + 1, entry!.start)
            .match(/^[\t ]*/)?.[0] ?? '';
        const object = printAddedObject(
          [compatibilityProperty('stdout')],
          indent,
          source.includes('\r\n') ? '\r\n' : '\n',
        );
        if (entry?.type === 'ArrayExpression') {
          // Retain comments and a possible trailing comma in a reporter tuple.
          editor.edit(name.end, name.end, `, ${object}`);
        } else {
          const tuple = `[${editor.text(name)}, ${object}]`;
          editor.replace(entry!, prop.value.type === 'ArrayExpression' ? tuple : `[${tuple}]`);
        }
      }
    }
    if (
      output &&
      entries.some((entry) => {
        const name = entry?.type === 'ArrayExpression' ? entry.elements[0] : entry;
        return isString(name) && name.value === 'html';
      })
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
    if (benchmarkOnly) {
      // Reporters are global; do not invent destinations on inline projects or
      // fragments whose effective root settings depend on a merge.
      if (!parentTest && !merged) {
        migrateBenchmarkConfig(editor, test);
      }
      return;
    }
    if (preserveDefaults && !inherits) {
      addCompatibility(test, 'clearMocks');
    }
    const browser = objectProperty(test, 'browser');
    if (browser && staticObject(browser.value)) {
      const value = browser.value;
      // An inherited browser config already receives its defaults at the parent.
      // Do not replace its explicit or dynamic locator setting in the child.
      const inheritsBrowser = inherits && parentTest && objectProperty(parentTest, 'browser');
      if (preserveDefaults && !inheritsBrowser) {
        nested(value, 'locators', [compatibilityProperty('exact')], (locators) =>
          addCompatibility(locators, 'exact'),
        );
      }
      const api = objectProperty(value, 'api');
      if (api) {
        const target = objectProperty(test, 'api');
        if (target && !sameStaticValue(target.value, api.value)) {
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
      nested(test, 'fakeTimers', [compatibilityProperty('toNotFake')], (timers) =>
        addCompatibility(timers, 'toNotFake'),
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
            threshold.type !== 'Property' ||
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
            addCompatibility(threshold.value, 'perFile');
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
    const hasInline = projects.value.elements.some((item) => item && !isString(item));
    if (hasInline && preserveDefaults) {
      addCompatibility(test, 'sharedViteServer');
    }
    for (const project of projects.value.elements) {
      if (!project || isString(project)) {
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
        addCompatibility(project, 'extends');
      }
      config(project, isTrue(extendsValue?.value), test);
    }
  }

  function config(object: t.ObjectExpression, inherits = false, parentTest?: t.ObjectExpression) {
    if (visited.has(object)) {
      return;
    }
    visited.add(object);
    const extendsValue = objectProperty(object, 'extends');
    if (
      extendsValue &&
      !isBoolean(extendsValue.value) &&
      (options.preserveV4 || options.reviewV4)
    ) {
      // A string refers to a different base, not parentTest. Until that base's
      // effective options are known, child defaults can override explicit values.
      editor.report(
        extendsValue,
        'project-inheritance',
        'Review this external or dynamic project base before adding v4 compatibility defaults. Inherited project settings were left unchanged.',
      );
      return;
    }
    const test = objectProperty(object, 'test');
    if (!test) {
      if (preserveDefaults && !inherits) {
        const properties = [compatibilityProperty('clearMocks')];
        if (options.temporalPolyfill) {
          properties.push({ key: 'fakeTimers', value: [compatibilityProperty('toNotFake')] });
        }
        editor.add(object, 'test', properties);
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

  editor.visit({
    AssignmentExpression(node) {
      if (isModuleExports(editor, node.left)) {
        if (staticObject(node.right)) {
          config(node.right);
        } else if (node.right.type !== 'CallExpression') {
          editor.report(
            node,
            'dynamic-config',
            'Review the effective CommonJS test config and its v4 defaults.',
          );
        }
      }
    },
    ExportDefaultDeclaration(node) {
      if (staticObject(node.declaration)) {
        config(node.declaration);
      } else if (node.declaration.type === 'Identifier') {
        const binding = editor.binding(node.declaration);
        if (
          binding?.constant &&
          binding.declaration.type === 'VariableDeclarator' &&
          staticObject(binding.declaration.init)
        ) {
          config(binding.declaration.init);
        } else {
          editor.report(
            node,
            'dynamic-config',
            'Review the effective exported test config and its v4 defaults.',
          );
        }
      } else if (node.declaration.type !== 'CallExpression') {
        editor.report(
          node,
          'dynamic-config',
          'Review the effective function or promise config and its v4 defaults.',
        );
      }
    },
    CallExpression(node) {
      const name = importedName(editor, node.callee, CONFIG_SOURCES);
      if (name === 'defineConfig' || name === 'defineProject') {
        if (benchmarkOnly && name === 'defineProject') {
          return;
        }
        const parent = editor.parent(node);
        const binding =
          parent?.type === 'VariableDeclarator' && parent.id.type === 'Identifier'
            ? editor.binding(parent.id)
            : undefined;
        const directlyExported =
          parent?.type === 'ExportDefaultDeclaration' ||
          (parent?.type === 'AssignmentExpression' &&
            parent.right === node &&
            isModuleExports(editor, parent.left)) ||
          (binding?.constant &&
            binding.references.length > 0 &&
            binding.references.every(
              (reference) => editor.parent(reference)?.type === 'ExportDefaultDeclaration',
            ));
        // Wrappers and dynamic inline projects can supply their own defaults.
        // Do not let a nested helper bypass the ownership check above.
        if (!directlyExported && !merged) {
          editor.report(
            node,
            'dynamic-config',
            'Review this config fragment in its wrapper or inline project before adding v4 compatibility settings.',
          );
          return;
        }
        const object = node.arguments[0];
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
          node,
          'nested-project-merge',
          'Check merged root configs for test.projects; v5 permits nested projects and can recurse or duplicate them.',
        );
      }
    },
    Property(node) {
      if (
        ['config', 'configResolved', 'configureServer', 'configureVitest'].includes(
          propertyName(node.key) ?? '',
        )
      ) {
        editor.report(
          node,
          'project-server-lifecycle',
          'Review this plugin hook for per-project config execution or Vite server state.',
        );
      }
    },
  });
  return editor.finish();
}
