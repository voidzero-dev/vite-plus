import type * as t from '@oxc-project/types';

import entryPoints from '../../vitest-v5-entry-points.json' with { type: 'json' };
import {
  NODE_SOURCES,
  ROOT_TEST_SOURCES,
  SourceEditor,
  importedName,
  memberName,
  objectProperty,
  propertyName,
  staticObject,
  testApiName,
  isString,
  isImportMetaVitest,
  isBoolean,
  isRegExp,
  type RewriteResult,
  type SourceOptions,
} from './ast.ts';
import { migrateBenchmarks } from './benchmarks.ts';

const RUNNER_SYMBOLS: Record<string, string> = {
  test: 'test',
  it: 'it',
  describe: 'describe',
  suite: 'suite',
  recordArtifact: 'recordArtifact',
  TestAPI: 'TestAPI',
  SuiteAPI: 'SuiteAPI',
  SuiteCollector: 'SuiteCollector',
  TestArtifact: 'TestArtifact',
  File: 'RunnerTestFile',
  Suite: 'RunnerTestSuite',
  Test: 'RunnerTestCase',
  Task: 'RunnerTask',
  VitestRunner: 'VitestTestRunner',
  VitestRunnerConfig: 'TestRunnerConfig',
};
const RUNNER_METHODS: Record<string, string> = {
  getCurrentSuite: 'getCurrentSuite',
  getCurrentTest: 'getCurrentTest',
  createTaskCollector: 'createTaskCollector',
  getFn: 'getTestFn',
  getHooks: 'getSuiteHooks',
  setFn: 'setTestFn',
  setHooks: 'setSuiteHooks',
};
const EXPECT_SYMBOLS = new Set([
  'expect',
  'assert',
  'should',
  'createExpect',
  'Assertion',
  'AsymmetricMatchersContaining',
  'ExpectStatic',
  'JestAssertion',
  'Matcher',
  'Matchers',
  'MatchersObject',
  'MatcherState',
]);
const RUNNER_SOURCES = new Set([
  '@vitest/runner',
  'vitest/suite',
  'vite-plus/test/suite',
  'vite-plus/test/plugins/runner',
]);
const EXPECT_SOURCES = new Set(['@vitest/expect', 'vite-plus/test/plugins/expect']);
const RUNNERS_SOURCES = new Set(['vitest/runners', 'vite-plus/test/runners']);
const REMOVED_SOURCES = new Set([
  'vitest/internal/module-runner',
  'vite-plus/test/internal/module-runner',
  '@vitest/runner/utils',
  '@vitest/runner/types',
  'vite-plus/test/plugins/runner-utils',
  'vite-plus/test/plugins/runner-types',
]);
const LEGACY_API_SOURCES = new Set([
  ...RUNNER_SOURCES,
  ...RUNNERS_SOURCES,
  ...EXPECT_SOURCES,
  ...REMOVED_SOURCES,
]);
const RENDER_SOURCES = new Set(['vitest-browser-vue', 'vitest-browser-svelte']);
const REGISTRATIONS = new Set(['test', 'it', 'describe', 'suite']);
const ASYNC_CALLBACKS = new Set(['test', 'it', 'beforeEach', 'afterEach', 'beforeAll', 'afterAll']);
const DOM_GLOBALS = new Set([
  'window',
  'document',
  'navigator',
  'location',
  'matchMedia',
  'innerWidth',
  'innerHeight',
  'screen',
  'Element',
  'HTMLElement',
  'Node',
  'Event',
  'MutationObserver',
  'ResizeObserver',
  'requestAnimationFrame',
  'cancelAnimationFrame',
  'getComputedStyle',
]);

const CHILD_PROCESS_SOURCES = new Set(['node:child_process', 'child_process']);
const ENVIRONMENT_SOURCES = new Set([
  'vitest/environments',
  'vite-plus/test/environments',
  'vitest/runtime',
  'vite-plus/test/runtime',
]);
const ASSERTION_SOURCES = new Set(['vitest', 'vite-plus/test', ...EXPECT_SOURCES]);

/** Follow immutable bindings, including Map iteration variables, back to
 * populateGlobal. A map merely named `originals` is not a Vitest API. */
function globalRestorationOrigin(
  editor: SourceEditor,
  node: t.Node,
  seen = new Set<t.Node>(),
): 'result' | 'originals' | 'value' | undefined {
  if (seen.has(node)) {
    return undefined;
  }
  seen.add(node);
  const origin = (value: t.Node) => globalRestorationOrigin(editor, value, seen);
  if (node.type === 'CallExpression') {
    if (importedName(editor, node.callee, ENVIRONMENT_SOURCES) === 'populateGlobal') {
      return 'result';
    }
    if (
      memberName(node.callee) === 'get' &&
      node.callee.type === 'MemberExpression' &&
      origin(node.callee.object) === 'originals'
    ) {
      return 'value';
    }
  }
  if (node.type === 'MemberExpression' && memberName(node) === 'originals') {
    return origin(node.object) === 'result' ? 'originals' : undefined;
  }
  const binding = editor.binding(node);
  if (!binding?.constant) {
    return undefined;
  }
  const declaration = binding.declaration;
  const parent = editor.parent(declaration);
  if (declaration.type === 'VariableDeclarator' && declaration.init) {
    return origin(declaration.init);
  }
  if (
    declaration.type === 'Property' &&
    !declaration.computed &&
    propertyName(declaration.key) === 'originals' &&
    parent?.type === 'ObjectPattern'
  ) {
    const variable = editor.parent(parent);
    if (variable?.type === 'VariableDeclarator' && variable.init) {
      return origin(variable.init) === 'result' ? 'originals' : undefined;
    }
  }
  if (
    (declaration.type === 'ArrowFunctionExpression' || declaration.type === 'FunctionExpression') &&
    declaration.params[0]?.type === 'Identifier' &&
    editor.binding(declaration.params[0]) === binding &&
    parent?.type === 'CallExpression' &&
    parent.arguments[0] === declaration &&
    parent.callee.type === 'MemberExpression' &&
    memberName(parent.callee) === 'forEach'
  ) {
    return origin(parent.callee.object) === 'originals' ? 'value' : undefined;
  }
  if (
    declaration.type === 'ArrayPattern' &&
    declaration.elements[1]?.type === 'Identifier' &&
    editor.binding(declaration.elements[1]) === binding &&
    parent?.type === 'VariableDeclarator'
  ) {
    const variables = editor.parent(parent);
    const loop = variables && editor.parent(variables);
    if (loop?.type === 'ForOfStatement' && loop.left === variables) {
      const iterable = loop.right;
      const map =
        iterable.type === 'CallExpression' &&
        iterable.callee.type === 'MemberExpression' &&
        memberName(iterable.callee) === 'entries'
          ? iterable.callee.object
          : iterable;
      return origin(map) === 'originals' ? 'value' : undefined;
    }
  }
  return undefined;
}

function assertionTypeName(editor: SourceEditor, node: t.Node): string | undefined {
  if (node.type === 'TSQualifiedName') {
    if (
      node.left.type === 'Identifier' &&
      node.left.name === 'jest' &&
      node.right.name === 'Matchers'
    ) {
      return 'jest.Matchers';
    }
    const declaration = editor.binding(node.left)?.declaration;
    const imported = declaration && editor.parent(declaration);
    return declaration?.type === 'ImportNamespaceSpecifier' &&
      imported?.type === 'ImportDeclaration' &&
      ASSERTION_SOURCES.has(imported.source.value)
      ? node.right.name
      : undefined;
  }
  if (node.type !== 'Identifier') {
    return undefined;
  }
  const declaration = editor.binding(node)?.declaration;
  const imported = declaration && editor.parent(declaration);
  if (declaration?.type === 'ImportSpecifier' && imported?.type === 'ImportDeclaration') {
    return ASSERTION_SOURCES.has(imported.source.value)
      ? propertyName(declaration.imported)
      : undefined;
  }
  return node.name;
}

function isJestMatchers(editor: SourceEditor, node: t.TSInterfaceDeclaration): boolean {
  let block = editor.parent(node);
  if (block?.type === 'ExportNamedDeclaration') {
    block = editor.parent(block);
  }
  const namespace = block && editor.parent(block);
  return (
    node.id.name === 'Matchers' &&
    namespace?.type === 'TSModuleDeclaration' &&
    namespace.id.type === 'Identifier' &&
    namespace.id.name === 'jest'
  );
}

function replacementImport(source: string, name: string): string | undefined {
  if (RUNNER_SOURCES.has(source)) {
    return RUNNER_SYMBOLS[name];
  }
  if (RUNNERS_SOURCES.has(source) && name === 'VitestTestRunner') {
    return 'TestRunner';
  }
  if (EXPECT_SOURCES.has(source) && EXPECT_SYMBOLS.has(name)) {
    return name;
  }
  return undefined;
}

function processApiName(editor: SourceEditor, node: t.Node): string | undefined {
  const imported = importedName(editor, node, CHILD_PROCESS_SOURCES);
  if (imported) {
    return imported;
  }
  let name = memberName(node);
  let receiver = node.type === 'MemberExpression' ? node.object : node;
  let declaration = editor.binding(receiver)?.declaration;
  const parent = declaration && editor.parent(declaration);
  if (
    declaration?.type === 'ImportDefaultSpecifier' &&
    parent?.type === 'ImportDeclaration' &&
    CHILD_PROCESS_SOURCES.has(parent.source.value)
  ) {
    return name;
  }
  if (declaration?.type === 'Property' && parent?.type === 'ObjectPattern') {
    name = propertyName(declaration.key);
    declaration = editor.parent(parent);
  }
  if (declaration?.type === 'VariableDeclarator' && declaration.init) {
    receiver = declaration.init;
  }
  return receiver.type === 'CallExpression' &&
    receiver.callee.type === 'Identifier' &&
    receiver.callee.name === 'require' &&
    !editor.binding(receiver.callee) &&
    isString(receiver.arguments[0]) &&
    CHILD_PROCESS_SOURCES.has(receiver.arguments[0].value)
    ? name
    : undefined;
}

/** A package-name string in plugin code or documentation is not a runner
 * dependency. Module references, in-source APIs, and bound process calls establish usage. */
export function hasVitestV5SourceUsage(file: string, source: string): boolean {
  if (!/vitest|vite-plus|\bvp\b/.test(source)) {
    return false;
  }
  let editor: SourceEditor;
  try {
    editor = new SourceEditor(file, source);
  } catch {
    // Do not let unsupported syntax bypass the version gate. The source pass
    // retains its parse diagnostic when the original runner can be resolved.
    return true;
  }
  let used = editor.comments.some(({ start, end }) =>
    /^\/\/\/\s*<reference\s+types\s*=\s*['"](?:vitest(?:\/[^'"]*)?|@vitest\/[^'"]+|vite-plus\/test(?:\/[^'"]*)?)['"]/.test(
      source.slice(start, end),
    ),
  );
  const moduleReference = (node: t.Node | null | undefined) => {
    if (
      isString(node) &&
      /^(?:vitest(?:\/|$)|@vitest\/|vite-plus\/test(?:\/|$))/.test(node.value)
    ) {
      used = true;
    }
  };
  editor.visit({
    MemberExpression(node) {
      if (isImportMetaVitest(node)) {
        used = true;
      }
    },
    ImportDeclaration: (node) => moduleReference(node.source),
    ExportNamedDeclaration: (node) => moduleReference(node.source),
    ExportAllDeclaration: (node) => moduleReference(node.source),
    ImportExpression: (node) => moduleReference(node.source),
    TSImportType: (node) => moduleReference(node.source),
    TSExternalModuleReference: (node) => moduleReference(node.expression),
    TSModuleDeclaration: (node) => moduleReference(node.id),
    CallExpression(node) {
      if (
        node.callee.type === 'Identifier' &&
        node.callee.name === 'require' &&
        !editor.binding(node.callee)
      ) {
        moduleReference(node.arguments[0]);
      }
      const command = node.arguments[0];
      if (
        ['exec', 'execSync', 'execFile', 'execFileSync', 'spawn', 'spawnSync'].includes(
          processApiName(editor, node.callee) ?? '',
        ) &&
        isString(command)
      ) {
        const args = node.arguments[1];
        const argv =
          args?.type === 'ArrayExpression' && args.elements.every(isString)
            ? args.elements.map((arg) => arg.value).join(' ')
            : '';
        if (/\b(?:vitest|vp\s+test)(?:\s|$)/.test(`${command.value} ${argv}`)) {
          used = true;
        }
      }
    },
  });
  return used;
}

function looksLikeConstructorMock(editor: SourceEditor, node: t.CallExpression): boolean {
  if (
    node.arguments.some(
      (argument) => argument.type === 'ClassExpression' || argument.type === 'FunctionExpression',
    )
  ) {
    return true;
  }
  if (
    memberName(node.callee) === 'spyOn' &&
    isString(node.arguments[1]) &&
    /^[A-Z]/.test(node.arguments[1].value)
  ) {
    return true;
  }
  const parent = editor.parent(node);
  if (parent?.type === 'VariableDeclarator' && parent.id.type === 'Identifier') {
    const name = parent.id.name;
    const binding = editor.binding(parent.id);
    return (
      /^[A-Z]/.test(name) ||
      !!binding?.references.some((ref) => editor.parent(ref)?.type === 'NewExpression')
    );
  }
  return false;
}

function chain(
  node: t.Node,
  isApi: (node: t.Node) => boolean,
): { root: t.Node; members: string[]; calls: t.CallExpression[] } {
  const members: string[] = [];
  const calls: t.CallExpression[] = [];
  while (node.type === 'MemberExpression' || node.type === 'CallExpression') {
    if (node.optional) {
      break;
    }
    if (isApi(node)) {
      break;
    }
    if (node.type === 'MemberExpression') {
      const name = memberName(node);
      if (name === undefined) {
        break;
      }
      members.push(name);
      node = node.object;
    } else {
      calls.push(node);
      node = node.callee;
    }
  }
  return { root: node, members, calls };
}

export function migrateVitestV5Source(
  file: string,
  source: string,
  options: SourceOptions,
): RewriteResult {
  const result = rewriteSource(file, source, options);
  if (options.reviewGlobals && !options.globals) {
    // Probe without applying edits: unknown ownership matters only when it
    // prevents a compatibility edit or diagnostic, not for every test/expect.
    const withGlobals = rewriteSource(file, source, { ...options, globals: true });
    if (
      result.content !== withGlobals.content ||
      withGlobals.findings.some(
        (finding) =>
          !result.findings.some(
            (existing) =>
              existing.code === finding.code &&
              existing.line === finding.line &&
              existing.column === finding.column &&
              existing.message === finding.message,
          ),
      )
    ) {
      result.findings.push({
        file,
        line: 1,
        column: 1,
        severity: 'review',
        code: 'global-api-ownership',
        message:
          'Resolve the Vitest project ownership of this file before migrating its affected global APIs. Global API edits were not applied because config selection, file scope, or globals settings are unresolved or conflicting.',
      });
    }
  }
  return result;
}

function rewriteSource(file: string, source: string, options: SourceOptions): RewriteResult {
  const editor = new SourceEditor(file, source);
  const reviewV4 = options.preserveV4 || options.reviewV4;
  const benchmarks = migrateBenchmarks(editor, options.globals);
  const runnerAliases: string[] = [];
  const asyncFunctions = new Set<t.Node>();
  // Import edits are offset-based, so bindings still refer to the old module
  // while this traversal visits the assertions that must migrate with them.
  function apiName(node: t.Node): string | undefined {
    return (
      testApiName(editor, node, options.globals) ??
      (importedName(editor, node, EXPECT_SOURCES) === 'expect' ? 'expect' : undefined)
    );
  }
  function canAwait(node: t.Node): boolean {
    const fn = editor.functionParent(node);
    if (!fn) {
      return false;
    }
    if (fn.async) {
      return true;
    }
    if (
      (fn.type !== 'ArrowFunctionExpression' && fn.type !== 'FunctionExpression') ||
      fn.generator ||
      fn.returnType
    ) {
      return false;
    }
    const parent = editor.parent(fn);
    if (parent?.type !== 'CallExpression' || !parent.arguments.includes(fn)) {
      return false;
    }
    const { root, members } = chain(parent.callee, (node) => !!apiName(node));
    if (!ASYNC_CALLBACKS.has(apiName(root) ?? '') || members.includes('extend')) {
      return false;
    }
    if (!asyncFunctions.has(fn)) {
      editor.edit(fn.start, fn.start, 'async ');
      asyncFunctions.add(fn);
    }
    return true;
  }

  function unsupported(node: t.Node, source: string, symbol: string, typeOnly: boolean) {
    editor.report(
      node,
      'removed-api',
      `Migrate ${symbol} from ${source} manually; no reviewed root v5 replacement exists.`,
      typeOnly ? 'review' : 'block',
    );
  }

  function dynamicImport(node: t.Node, source: string): void {
    if (LEGACY_API_SOURCES.has(source)) {
      unsupported(node, source, 'dynamic/CommonJS import', false);
    }
  }

  editor.visit({
    ImportDeclaration(node) {
      const source = node.source.value;
      if (ROOT_TEST_SOURCES.has(source)) {
        for (const specifier of node.specifiers) {
          if (
            specifier.type === 'ImportSpecifier' &&
            propertyName(specifier.imported) === 'bench' &&
            node.importKind !== 'type' &&
            specifier.importKind !== 'type' &&
            !benchmarks.imports.has(specifier)
          ) {
            editor.report(
              specifier,
              'benchmark-api',
              'Migrate these bench references manually: automatic migration requires direct calls with inline or locally resolved zero-argument callbacks and no benchmark options. Review wrappers, comparison groups, and escaped references.',
              'block',
            );
          }
        }
      }
      if (source in entryPoints) {
        editor.replace(
          node.source,
          JSON.stringify(entryPoints[source as keyof typeof entryPoints]),
        );
        return;
      }
      if (!LEGACY_API_SOURCES.has(source)) {
        return;
      }
      const imports: string[] = [];
      const constants: string[] = [];
      const remaining: string[] = [];
      let runnerName: string | undefined;
      for (const specifier of node.specifiers) {
        const typeOnly =
          node.importKind === 'type' ||
          (specifier.type === 'ImportSpecifier' && specifier.importKind === 'type');
        const name = specifier.type === 'ImportSpecifier' ? propertyName(specifier.imported)! : '*';
        const target = replacementImport(source, name);
        if (target) {
          imports.push(
            `${typeOnly ? 'type ' : ''}${target}${specifier.local.name === target ? '' : ` as ${specifier.local.name}`}`,
          );
        } else if (RUNNER_SOURCES.has(source) && RUNNER_METHODS[name] && !typeOnly) {
          if (!runnerName) {
            runnerName = editor.uniqueName('VitestTestRunner');
            imports.push(`TestRunner as ${runnerName}`);
          }
          // Static fields are the original functions, so aliases retain function
          // identity and also work as callback values (not only direct calls).
          constants.push(`const ${specifier.local.name} = ${runnerName}.${RUNNER_METHODS[name]};`);
        } else {
          unsupported(specifier, source, name, typeOnly);
          remaining.push(editor.text(specifier));
        }
      }
      if (!node.specifiers.length) {
        unsupported(node, source, 'side-effect import', false);
      }
      if (imports.length) {
        // A default or namespace import cannot be emitted inside named-import
        // braces. Keep this blocked declaration intact for manual migration.
        if (node.specifiers.some((specifier) => specifier.type !== 'ImportSpecifier')) {
          return;
        }
        const kept = remaining.length
          ? `import ${node.importKind === 'type' ? 'type ' : ''}{ ${remaining.join(', ')} } from ${JSON.stringify(source)};\n`
          : '';
        editor.replace(node, `${kept}import { ${imports.join(', ')} } from 'vite-plus/test';`);
        runnerAliases.push(...constants);
      }
    },
    ExportNamedDeclaration(node) {
      const source = node.source?.value;
      if (!source) {
        return;
      }
      if (source in entryPoints) {
        editor.replace(
          node.source!,
          JSON.stringify(entryPoints[source as keyof typeof entryPoints]),
        );
      } else if (LEGACY_API_SOURCES.has(source)) {
        // Re-exports can expose a library contract. Do not silently replace it.
        for (const specifier of node.specifiers) {
          unsupported(
            specifier,
            source,
            're-export',
            node.exportKind === 'type' ||
              (specifier.type === 'ExportSpecifier' && specifier.exportKind === 'type'),
          );
        }
      }
    },
    ExportAllDeclaration(node) {
      const source = node.source.value;
      if (LEGACY_API_SOURCES.has(source)) {
        unsupported(node, source, 'export *', node.exportKind === 'type');
      }
    },
    CallExpression(node) {
      // Babel represented these as OptionalCallExpression, outside this pass.
      // Retain that conservative behavior for Oxc's optional CallExpression.
      if (node.optional) {
        return;
      }
      const parent = editor.parent(node);
      const { root, members } = chain(node.callee, (node) => !!apiName(node));
      const name = apiName(root);
      if (
        name === 'bench' &&
        (root.type !== 'Identifier' || !editor.binding(root)) &&
        !benchmarks.calls.has(node)
      ) {
        editor.report(
          node,
          'benchmark-api',
          'Migrate this bench call manually: automatic migration requires a direct call with an inline or locally resolved zero-argument callback and no benchmark options.',
          'block',
        );
      }

      if (REGISTRATIONS.has(name ?? '')) {
        const seqMember =
          node.callee.type === 'MemberExpression' && memberName(node.callee) === 'sequential'
            ? node.callee
            : undefined;
        const opts = node.arguments[1];
        const sequentialOption = staticObject(opts)
          ? objectProperty(opts, 'sequential')
          : undefined;
        let unresolvedSequential = members.includes('sequential');
        if (seqMember && node.arguments.length >= 2) {
          if (
            staticObject(opts) &&
            !objectProperty(opts, 'concurrent') &&
            (!sequentialOption ||
              (isBoolean(sequentialOption.value) && sequentialOption.value.value))
          ) {
            editor.edit(seqMember.object.end, seqMember.end, '');
            if (!sequentialOption) {
              editor.add(opts, 'concurrent', 'false');
            }
            unresolvedSequential = false;
          } else if (
            (opts?.type === 'ArrowFunctionExpression' || opts?.type === 'FunctionExpression') &&
            (node.arguments.length === 2 ||
              (node.arguments.length === 3 &&
                node.arguments[2].type === 'Literal' &&
                typeof node.arguments[2].value === 'number' &&
                /^\s*,\s*$/.test(editor.source.slice(opts.end, node.arguments[2].start))))
          ) {
            const timeout = node.arguments[2];
            editor.edit(seqMember.object.end, seqMember.end, '');
            editor.edit(
              opts.start,
              opts.start,
              `{ concurrent: false${timeout ? `, timeout: ${editor.text(timeout)}` : ''} }, `,
            );
            if (timeout) {
              // Vitest accepts three arguments; a fourth timeout is ignored.
              editor.edit(opts.end, timeout.end, '');
            }
            unresolvedSequential = false;
          } else {
            editor.report(
              node,
              'sequential-api',
              'Replace sequential with concurrent: false after resolving the options and callback.',
            );
          }
        } else if (members.includes('sequential')) {
          editor.report(
            node,
            'sequential-api',
            'Review the sequential modifier chain and replace it with concurrent: false.',
          );
        }
        for (const argument of node.arguments.slice(1)) {
          if (unresolvedSequential || !staticObject(argument)) {
            continue;
          }
          const sequential = objectProperty(argument, 'sequential');
          if (!sequential) {
            continue;
          }
          if (
            isBoolean(sequential.value) &&
            sequential.value.value &&
            !objectProperty(argument, 'concurrent')
          ) {
            editor.replace(sequential.key, 'concurrent');
            editor.replace(sequential.value, 'false');
          } else {
            editor.report(
              sequential,
              'sequential-api',
              'Resolve sequential/concurrent options explicitly before removing sequential.',
            );
          }
        }
      }

      if (name === 'vi' || name === 'vitest') {
        const method = memberName(node.callee);
        if (['mock', 'unmock', 'hoisted'].includes(method ?? '')) {
          let statement = parent;
          while (
            statement &&
            !statement.type.endsWith('Statement') &&
            statement.type !== 'VariableDeclaration' &&
            statement.type !== 'Program'
          ) {
            statement = editor.parent(statement);
          }
          // A top-level variable initializer for vi.hoisted is valid too.
          if (
            editor.functionParent(node) ||
            !statement ||
            editor.parent(statement)?.type !== 'Program'
          ) {
            editor.report(
              node,
              'nested-hoisted-mock',
              'Move this hoisted mock to the top level after reviewing captured scope.',
            );
          }
        }
        if (
          reviewV4 &&
          (options.browser || (options.browser === undefined && options.browserPossible)) &&
          method === 'mock' &&
          node.arguments.length === 1
        ) {
          editor.report(
            node,
            'browser-automock',
            'Browser automocks now keep mock defaults; choose { spy: true } if real implementations are required.',
          );
        }
        if (
          reviewV4 &&
          (method === 'fn' || method === 'spyOn') &&
          looksLikeConstructorMock(editor, node)
        ) {
          editor.report(
            node,
            'class-mock',
            'If this mock replaces a constructor, review its prototype, methods, and instanceof behavior.',
          );
        }
        if (
          reviewV4 &&
          method === 'setSystemTime' &&
          (options.temporalPolyfill || /\bTemporal\b/.test(source))
        ) {
          editor.report(
            node,
            'temporal-system-time',
            'vi.setSystemTime now changes Temporal even without fake timers; toNotFake does not preserve this behavior.',
          );
        }
      }
      if (
        reviewV4 &&
        memberName(node.callee) === 'mockImplementation' &&
        looksLikeConstructorMock(editor, node)
      ) {
        editor.report(
          node,
          'class-mock',
          'Review constructor mock implementations and their inherited prototypes.',
        );
      }

      if (name === 'expect') {
        const matcher = memberName(node.callee);
        const first = node.arguments[0];
        if (
          options.preserveV4 &&
          ['toThrow', 'toThrowError'].includes(matcher ?? '') &&
          isString(first) &&
          first.value === ''
        ) {
          editor.replace(first, '/^$/');
        }
        if (reviewV4 && matcher === 'toHaveTextContent') {
          const browserAssertion = members.includes('element') || options.browser === true;
          const literal = isString(first) || isRegExp(first);
          if (options.preserveV4 && browserAssertion && literal) {
            const callee = node.callee as t.MemberExpression;
            editor.replace(
              callee.property,
              callee.computed ? JSON.stringify('toMatchTextContent') : 'toMatchTextContent',
            );
          } else if (browserAssertion && !literal) {
            editor.report(
              node,
              'text-content',
              'Choose toMatchTextContent for v4 partial/regex matching after resolving the expected value.',
            );
          } else if (options.browser === undefined && options.browserPossible) {
            editor.report(
              node,
              'text-content-project',
              options.ownershipReason
                ? `${options.ownershipReason} Browser assertions need toMatchTextContent for v4 partial matching; keep Node jest-dom assertions unchanged.`
                : "Resolve this assertion's test project: browser assertions need toMatchTextContent for v4 partial matching; keep Node jest-dom assertions unchanged.",
            );
          }
        }
        if (reviewV4 && members.includes('poll') && matcher !== 'poll') {
          editor.report(
            node,
            'poll-timeout',
            'Review the configured expect.poll timeout; v5 rejects assertions that finish after it.',
          );
        }
        const asynchronous = members.some((member) =>
          [
            'resolves',
            'rejects',
            'poll',
            'element',
            'toMatchFileSnapshot',
            'toMatchScreenshot',
          ].includes(member),
        );
        if (asynchronous && parent?.type === 'ExpressionStatement') {
          if (canAwait(node)) {
            editor.edit(node.start, node.start, 'await ');
          } else {
            editor.report(
              node,
              'unawaited-assertion',
              'Await or return this asynchronous assertion in an async-compatible function.',
            );
          }
        } else if (
          asynchronous &&
          parent?.type !== 'MemberExpression' &&
          parent?.type !== 'AwaitExpression' &&
          parent?.type !== 'ReturnStatement'
        ) {
          editor.report(
            node,
            'unawaited-assertion',
            'Check that the result of this asynchronous assertion is awaited or returned.',
          );
        }
      }

      if (
        importedName(editor, node.callee, RENDER_SOURCES) === 'render' &&
        parent?.type !== 'AwaitExpression' &&
        parent?.type !== 'ReturnStatement'
      ) {
        if (canAwait(node)) {
          editor.edit(node.start, node.start, '(await ');
          editor.edit(node.end, node.end, ')');
        } else {
          editor.report(
            node,
            'async-render',
            'Await render from vitest-browser-vue/svelte after making the enclosing contract async-compatible.',
          );
        }
      }

      if (
        importedName(editor, node.callee, NODE_SOURCES) === 'resolveConfig' &&
        options.preserveV4
      ) {
        const declaration = parent && editor.parent(parent);
        if (
          parent?.type === 'AwaitExpression' &&
          declaration?.type === 'VariableDeclarator' &&
          declaration.id.type === 'ObjectPattern'
        ) {
          const props = declaration.id.properties;
          if (
            props.every(
              (prop) =>
                prop.type === 'Property' &&
                !prop.computed &&
                prop.value.type === 'Identifier' &&
                ['viteConfig', 'vitestConfig'].includes(propertyName(prop.key) ?? ''),
            )
          ) {
            const vite = props.find(
              (prop) => prop.type === 'Property' && propertyName(prop.key) === 'viteConfig',
            ) as t.BindingProperty | undefined;
            const vitest = props.find(
              (prop) => prop.type === 'Property' && propertyName(prop.key) === 'vitestConfig',
            ) as t.BindingProperty | undefined;
            const local = vite ? editor.text(vite.value) : editor.uniqueName('viteConfig');
            editor.replace(declaration.id, local);
            if (vitest) {
              editor.edit(
                declaration.end,
                declaration.end,
                `, ${editor.text(vitest.value)} = ${local}.test`,
              );
            }
          } else {
            editor.report(
              declaration,
              'resolve-config',
              'Replace resolveConfig pair destructuring with the Vite config return value and its .test property.',
            );
          }
        } else {
          editor.report(
            node,
            'resolve-config',
            'Review resolveConfig consumers; the v5 return value is the Vite config with Vitest options under .test.',
          );
        }
      }

      if (memberName(node.callee) === 'collect') {
        const object = (node.callee as t.MemberExpression).object;
        const binding = editor.binding(object);
        let initializer =
          binding?.declaration.type === 'VariableDeclarator' ? binding.declaration.init : undefined;
        if (initializer?.type === 'AwaitExpression') {
          initializer = initializer.argument;
        }
        const vitestRunner =
          initializer?.type === 'CallExpression' &&
          ['createVitest', 'startVitest'].includes(
            importedName(editor, initializer.callee, NODE_SOURCES) ?? '',
          );
        const known = vitestRunner && binding?.constant;
        // collect(filters?, options?) places staticParse in the second argument.
        const opts = node.arguments[1];
        if (
          known &&
          options.preserveV4 &&
          node.arguments.length <= 2 &&
          !node.arguments.some((argument) => argument.type === 'SpreadElement') &&
          (!opts || staticObject(opts))
        ) {
          if (staticObject(opts)) {
            editor.add(opts, 'staticParse', 'false');
          } else {
            editor.edit(
              node.end - 1,
              node.end - 1,
              `${node.arguments.length ? ', ' : 'undefined, '}{ staticParse: false }`,
            );
          }
        } else if (
          reviewV4 &&
          vitestRunner &&
          (!known ||
            node.arguments.some((argument) => argument.type === 'SpreadElement') ||
            (opts && !staticObject(opts)))
        ) {
          editor.report(
            node,
            'static-collect',
            'If this is Vitest.collect(), review staticParse and set it to false to retain runtime collection.',
          );
        }
      }
      if (
        node.callee.type === 'Identifier' &&
        node.callee.name === 'require' &&
        !editor.binding(node.callee) &&
        isString(node.arguments[0])
      ) {
        dynamicImport(node, node.arguments[0].value);
      }
    },
    ImportExpression(node) {
      if (isString(node.source)) {
        dynamicImport(node, node.source.value);
      }
    },
    MemberExpression(node) {
      if (reviewV4 && ['VITEST_POOL_ID', 'VITEST_WORKER_ID'].includes(memberName(node) ?? '')) {
        editor.report(
          node,
          'worker-id',
          'Review worker/pool ID arithmetic and indexing: IDs now start at 1, not 0.',
        );
      }
    },
    VariableDeclarator(node) {
      if (reviewV4 && node.id.type === 'ObjectPattern') {
        for (const prop of node.id.properties) {
          if (
            prop.type === 'Property' &&
            ['VITEST_POOL_ID', 'VITEST_WORKER_ID'].includes(propertyName(prop.key) ?? '')
          ) {
            editor.report(
              prop,
              'worker-id',
              'Review worker/pool ID arithmetic and indexing: IDs now start at 1, not 0.',
            );
          }
        }
      }
    },
    AssignmentExpression(node) {
      if (reviewV4 && globalRestorationOrigin(editor, node.right) === 'value') {
        editor.report(
          node,
          'global-descriptors',
          'populateGlobal().originals stores descriptors; restore them with Object.defineProperty, not assignment.',
        );
      }
      const left = node.left;
      if (
        reviewV4 &&
        ((left.type === 'MemberExpression' &&
          left.object.type === 'Identifier' &&
          !editor.binding(left.object) &&
          ['window', 'globalThis', 'global'].includes(editor.text(left.object)) &&
          DOM_GLOBALS.has(memberName(left) ?? '')) ||
          (left.type === 'Identifier' && DOM_GLOBALS.has(left.name) && !editor.binding(left)))
      ) {
        editor.report(
          node,
          'dom-global',
          'In jsdom/happy-dom, global assignment also updates the window; review this DOM override.',
        );
      }
    },
    TSInterfaceDeclaration(node) {
      if (
        reviewV4 &&
        (isJestMatchers(editor, node) ||
          (['Assertion', 'Matchers'].includes(node.id.name) &&
            (node.typeParameters?.params.length ?? 0) < 2))
      ) {
        editor.report(
          node,
          'assertion-types',
          'Update custom matcher declarations for the v5 return and received type parameters; review jest.Matchers augmentations.',
        );
      }
    },
    TSTypeReference(node) {
      const name = assertionTypeName(editor, node.typeName);
      if (
        reviewV4 &&
        (name === 'jest.Matchers' ||
          (['Assertion', 'Matchers'].includes(name ?? '') &&
            (node.typeArguments?.params.length ?? 0) < 2))
      ) {
        editor.report(
          node,
          'assertion-types',
          'Review old assertion generics; v5 includes return and received types.',
        );
      }
    },
    TSImportType(node) {
      const source = node.source.value;
      if (LEGACY_API_SOURCES.has(source)) {
        unsupported(node, source, 'type import', true);
      }
    },
    Literal(node) {
      if (!isString(node)) {
        return;
      }
      const value = node.value;
      if (/\/__vitest__\//.test(value) && !/[?&]token=/.test(value)) {
        editor.report(
          node,
          'ui-token',
          'Use the authenticated UI URL printed by Vitest, including its token.',
        );
      }
      if (/\/__vitest_test__\//.test(value) && !/[?&]sessionId=/.test(value)) {
        editor.report(
          node,
          'browser-session',
          'Use the browser orchestrator URL opened by Vitest, including its sessionId.',
        );
      }
    },
  });
  if (runnerAliases.length) {
    // Imports are hoisted, but these aliases are not. Initialize them before
    // any executable statement, even when an import occurs later in the file.
    // Retain directives (and the shebang) at the start of the module.
    const firstStatement = editor.ast.body.find(
      (node) =>
        node.type !== 'ImportDeclaration' &&
        !(node.type === 'ExpressionStatement' && node.directive),
    );
    const offset = firstStatement?.start ?? source.length;
    editor.edit(offset, offset, `${firstStatement ? '' : '\n'}${runnerAliases.join('\n')}\n`);
  }
  const result = editor.finish();
  const benchmark =
    benchmarks.imports.values().next().value ??
    benchmarks.bindings.values().next().value ??
    benchmarks.calls.values().next().value;
  if (benchmark && result.content === source) {
    // Suppress removed-API findings only for edits that survive validation.
    // A rollback retains the legacy API and must stop dependency upgrades.
    editor.report(
      benchmark,
      'benchmark-api',
      'The benchmark rewrite was discarded. Migrate the remaining legacy bench API manually before upgrading to Vitest v5.',
      'block',
    );
  }
  return result;
}
