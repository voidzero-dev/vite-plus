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
  isBoolean,
  isRegExp,
  type SourceOptions,
} from './ast.ts';

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

export function migrateVitestV5Source(file: string, source: string, options: SourceOptions) {
  const editor = new SourceEditor(file, source);
  const asyncFunctions = new Set<t.Node>();
  // Import edits are offset-based, so bindings still refer to the old module
  // while this traversal visits the assertions that must migrate with them.
  const apiName = (node: t.Node) =>
    testApiName(editor, node, options.globals) ??
    (importedName(editor, node, EXPECT_SOURCES) === 'expect' ? 'expect' : undefined);
  const canAwait = (node: t.Node) => {
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
  };

  function unsupported(node: t.Node, source: string, symbol: string, typeOnly: boolean) {
    editor.report(
      node,
      'removed-api',
      `Migrate ${symbol} from ${source} manually; no reviewed root v5 replacement exists.`,
      typeOnly ? 'review' : 'block',
    );
  }

  function dynamicImport(node: t.Node, source: string): void {
    if (
      REMOVED_SOURCES.has(source) ||
      RUNNER_SOURCES.has(source) ||
      EXPECT_SOURCES.has(source) ||
      RUNNERS_SOURCES.has(source)
    ) {
      unsupported(node, source, 'dynamic/CommonJS import', false);
    }
    if (source === '@vitest/ws-client') {
      editor.report(
        node,
        'ws-client',
        'Replace direct @vitest/ws-client use; it does not receive Vitest v5 features.',
      );
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
            specifier.importKind !== 'type'
          ) {
            editor.report(
              specifier,
              'benchmark-api',
              'Replace the removed top-level bench import with the bench test-context fixture.',
              'block',
            );
          }
        }
      }
      if (source === '@vitest/ws-client') {
        editor.report(
          node,
          'ws-client',
          'Replace direct @vitest/ws-client use; it does not receive Vitest v5 features.',
        );
        return;
      }
      if (source in entryPoints) {
        editor.replace(
          node.source,
          JSON.stringify(entryPoints[source as keyof typeof entryPoints]),
        );
        return;
      }
      const runners = RUNNERS_SOURCES.has(source);
      const runner = RUNNER_SOURCES.has(source);
      const expect = EXPECT_SOURCES.has(source);
      if (!runner && !runners && !expect && !REMOVED_SOURCES.has(source)) {
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
        const target = runner
          ? RUNNER_SYMBOLS[name]
          : runners && name === 'VitestTestRunner'
            ? 'TestRunner'
            : expect && EXPECT_SYMBOLS.has(name)
              ? name
              : undefined;
        if (target) {
          imports.push(
            `${typeOnly ? 'type ' : ''}${target}${specifier.local.name === target ? '' : ` as ${specifier.local.name}`}`,
          );
        } else if (runner && RUNNER_METHODS[name] && !typeOnly) {
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
        const kept = remaining.length
          ? `import ${node.importKind === 'type' ? 'type ' : ''}{ ${remaining.join(', ')} } from ${JSON.stringify(source)};\n`
          : '';
        editor.replace(
          node,
          `${kept}import { ${imports.join(', ')} } from 'vite-plus/test';${constants.length ? `\n${constants.join('\n')}` : ''}`,
        );
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
      } else if (
        RUNNER_SOURCES.has(source) ||
        EXPECT_SOURCES.has(source) ||
        REMOVED_SOURCES.has(source) ||
        RUNNERS_SOURCES.has(source)
      ) {
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
      if (
        RUNNER_SOURCES.has(source) ||
        EXPECT_SOURCES.has(source) ||
        REMOVED_SOURCES.has(source) ||
        RUNNERS_SOURCES.has(source)
      ) {
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
      if (name === 'bench' && (root.type !== 'Identifier' || !editor.binding(root))) {
        editor.report(
          node,
          'benchmark-api',
          'Replace the removed top-level bench API with the bench test-context fixture.',
          'block',
        );
      }

      if (REGISTRATIONS.has(name ?? '')) {
        const seqMember =
          node.callee.type === 'MemberExpression' && memberName(node.callee) === 'sequential'
            ? node.callee
            : undefined;
        const opts = node.arguments[1];
        if (seqMember && node.arguments.length >= 2) {
          if (
            staticObject(opts) &&
            !objectProperty(opts, 'concurrent') &&
            !objectProperty(opts, 'sequential')
          ) {
            editor.edit(seqMember.object.end, seqMember.end, '');
            editor.add(opts, 'concurrent', 'false');
          } else if (
            opts?.type === 'ArrowFunctionExpression' ||
            opts?.type === 'FunctionExpression'
          ) {
            editor.edit(seqMember.object.end, seqMember.end, '');
            editor.edit(opts.start, opts.start, '{ concurrent: false }, ');
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
          if (!staticObject(argument)) {
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
        if (members.includes('each') || members.includes('for')) {
          editor.report(
            node,
            'formatted-titles',
            'Review generated test-title snapshots; v5 uses pretty-format and different string placeholders.',
          );
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
        if ((method === 'fn' || method === 'spyOn') && looksLikeConstructorMock(editor, node)) {
          editor.report(
            node,
            'class-mock',
            'If this mock replaces a constructor, review its prototype, methods, and instanceof behavior.',
          );
        }
        if (method === 'setSystemTime' && /\bTemporal\b/.test(source)) {
          editor.report(
            node,
            'temporal-system-time',
            'vi.setSystemTime now changes Temporal even without fake timers; toNotFake does not preserve this behavior.',
          );
        }
      }
      if (
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
        if ((options.preserveV4 || options.reviewV4) && matcher === 'toHaveTextContent') {
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
              "Resolve this assertion's test project: browser assertions need toMatchTextContent for v4 partial matching; keep Node jest-dom assertions unchanged.",
            );
          }
        }
        if (members.includes('poll') && matcher !== 'poll') {
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
          binding?.constant && binding.declaration.type === 'VariableDeclarator'
            ? binding.declaration.init
            : undefined;
        if (initializer?.type === 'AwaitExpression') {
          initializer = initializer.argument;
        }
        const known =
          initializer?.type === 'CallExpression' &&
          ['createVitest', 'startVitest'].includes(
            importedName(editor, initializer.callee, NODE_SOURCES) ?? '',
          );
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
          !known ||
          node.arguments.some((argument) => argument.type === 'SpreadElement') ||
          (opts && !staticObject(opts))
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
      if (['VITEST_POOL_ID', 'VITEST_WORKER_ID'].includes(memberName(node) ?? '')) {
        editor.report(
          node,
          'worker-id',
          'Review worker/pool ID arithmetic and indexing: IDs now start at 1, not 0.',
        );
      }
    },
    VariableDeclarator(node) {
      if (node.id.type === 'ObjectPattern') {
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
      if (/\boriginals\b/.test(editor.text(node.right))) {
        editor.report(
          node,
          'global-descriptors',
          'populateGlobal().originals stores descriptors; restore them with Object.defineProperty, not assignment.',
        );
      }
      const left = node.left;
      if (
        (left.type === 'MemberExpression' &&
          ['window', 'globalThis', 'global'].includes(editor.text(left.object)) &&
          DOM_GLOBALS.has(memberName(left) ?? '')) ||
        (left.type === 'Identifier' && DOM_GLOBALS.has(left.name))
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
        ['Assertion', 'Matchers'].includes(node.id.name) &&
        (node.typeParameters?.params.length ?? 0) < 2
      ) {
        editor.report(
          node,
          'assertion-types',
          'Update custom matcher declarations for the v5 return and received type parameters; review jest.Matchers augmentations.',
        );
      }
    },
    TSTypeReference(node) {
      const name = editor.text(node.typeName);
      if (
        ['Assertion', 'Matchers', 'jest.Matchers'].includes(name) &&
        (node.typeArguments?.params.length ?? 0) < 2
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
      if (
        RUNNER_SOURCES.has(source) ||
        EXPECT_SOURCES.has(source) ||
        REMOVED_SOURCES.has(source) ||
        RUNNERS_SOURCES.has(source)
      ) {
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
      if (/\.vitest-attachements|\.vitest-reports|__screenshots__|html\/index\.html/.test(value)) {
        editor.report(
          node,
          'artifact-paths',
          'Review this old artifact/report path. Generated output moved under .vitest; reference screenshots remain separate.',
        );
      }
    },
  });
  return editor.finish();
}
