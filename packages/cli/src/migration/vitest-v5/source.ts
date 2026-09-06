import type { NodePath } from '@babel/traverse';
import type * as t from '@babel/types';

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
  traverse,
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

function looksLikeConstructorMock(p: NodePath<t.CallExpression>): boolean {
  if (
    p.node.arguments.some(
      (argument) => argument.type === 'ClassExpression' || argument.type === 'FunctionExpression',
    )
  ) {
    return true;
  }
  if (
    memberName(p.node.callee) === 'spyOn' &&
    p.node.arguments[1]?.type === 'StringLiteral' &&
    /^[A-Z]/.test(p.node.arguments[1].value)
  ) {
    return true;
  }
  if (p.parentPath.isVariableDeclarator() && p.parentPath.node.id.type === 'Identifier') {
    const name = p.parentPath.node.id.name;
    const binding = p.scope.getBinding(name);
    return (
      /^[A-Z]/.test(name) ||
      !!binding?.referencePaths.some((ref) => ref.parentPath?.isNewExpression())
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
  const apiName = (p: NodePath, node: t.Node) => testApiName(p, node, options.globals);
  const canAwait = (p: NodePath) => {
    const fn = p.getFunctionParent();
    if (!fn) {
      return false;
    }
    if (fn.node.async) {
      return true;
    }
    if (
      (!fn.isArrowFunctionExpression() && !fn.isFunctionExpression()) ||
      fn.node.generator ||
      fn.node.returnType
    ) {
      return false;
    }
    const parent = fn.parentPath;
    if (!parent.isCallExpression() || !parent.node.arguments.includes(fn.node)) {
      return false;
    }
    const { root, members } = chain(parent.node.callee, (node) => !!apiName(parent, node));
    if (!ASYNC_CALLBACKS.has(apiName(parent, root) ?? '') || members.includes('extend')) {
      return false;
    }
    if (!asyncFunctions.has(fn.node)) {
      editor.edit(fn.node.start!, fn.node.start!, 'async ');
      asyncFunctions.add(fn.node);
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

  traverse(editor.ast, {
    ImportDeclaration(p) {
      const source = p.node.source.value;
      if (ROOT_TEST_SOURCES.has(source)) {
        for (const specifier of p.node.specifiers) {
          if (
            specifier.type === 'ImportSpecifier' &&
            propertyName(specifier.imported) === 'bench' &&
            p.node.importKind !== 'type' &&
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
          p.node,
          'ws-client',
          'Replace direct @vitest/ws-client use; it does not receive Vitest v5 features.',
        );
        return;
      }
      if (source in entryPoints) {
        editor.replace(
          p.node.source,
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
      for (const specifier of p.node.specifiers) {
        const typeOnly =
          p.node.importKind === 'type' ||
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
            runnerName = p.scope.generateUidIdentifier('VitestTestRunner').name;
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
      if (!p.node.specifiers.length) {
        unsupported(p.node, source, 'side-effect import', false);
      }
      if (imports.length) {
        const kept = remaining.length
          ? `import ${p.node.importKind === 'type' ? 'type ' : ''}{ ${remaining.join(', ')} } from ${JSON.stringify(source)};\n`
          : '';
        editor.replace(
          p.node,
          `${kept}import { ${imports.join(', ')} } from 'vite-plus/test';${constants.length ? `\n${constants.join('\n')}` : ''}`,
        );
      }
    },
    ExportNamedDeclaration(p) {
      const source = p.node.source?.value;
      if (!source) {
        return;
      }
      if (source in entryPoints) {
        editor.replace(
          p.node.source!,
          JSON.stringify(entryPoints[source as keyof typeof entryPoints]),
        );
      } else if (
        RUNNER_SOURCES.has(source) ||
        EXPECT_SOURCES.has(source) ||
        REMOVED_SOURCES.has(source) ||
        RUNNERS_SOURCES.has(source)
      ) {
        // Re-exports can expose a library contract. Do not silently replace it.
        for (const specifier of p.node.specifiers) {
          unsupported(
            specifier,
            source,
            're-export',
            p.node.exportKind === 'type' ||
              (specifier.type === 'ExportSpecifier' && specifier.exportKind === 'type'),
          );
        }
      }
    },
    ExportAllDeclaration(p) {
      const source = p.node.source.value;
      if (
        RUNNER_SOURCES.has(source) ||
        EXPECT_SOURCES.has(source) ||
        REMOVED_SOURCES.has(source) ||
        RUNNERS_SOURCES.has(source)
      ) {
        unsupported(p.node, source, 'export *', p.node.exportKind === 'type');
      }
    },
    CallExpression(p) {
      const { root, members } = chain(p.node.callee, (node) => !!apiName(p, node));
      const name = apiName(p, root);
      if (name === 'bench' && (root.type !== 'Identifier' || !p.scope.getBinding(root.name))) {
        editor.report(
          p.node,
          'benchmark-api',
          'Replace the removed top-level bench API with the bench test-context fixture.',
          'block',
        );
      }

      if (REGISTRATIONS.has(name ?? '')) {
        const seqMember =
          p.node.callee.type === 'MemberExpression' && memberName(p.node.callee) === 'sequential'
            ? p.node.callee
            : undefined;
        const opts = p.node.arguments[1];
        if (seqMember && p.node.arguments.length >= 2) {
          if (
            staticObject(opts) &&
            !objectProperty(opts, 'concurrent') &&
            !objectProperty(opts, 'sequential')
          ) {
            editor.edit(seqMember.object.end!, seqMember.end!, '');
            editor.add(opts, 'concurrent', 'false');
          } else if (
            opts?.type === 'ArrowFunctionExpression' ||
            opts?.type === 'FunctionExpression'
          ) {
            editor.edit(seqMember.object.end!, seqMember.end!, '');
            editor.edit(opts.start!, opts.start!, '{ concurrent: false }, ');
          } else {
            editor.report(
              p.node,
              'sequential-api',
              'Replace sequential with concurrent: false after resolving the options and callback.',
            );
          }
        } else if (members.includes('sequential')) {
          editor.report(
            p.node,
            'sequential-api',
            'Review the sequential modifier chain and replace it with concurrent: false.',
          );
        }
        for (const argument of p.node.arguments.slice(1)) {
          if (!staticObject(argument)) {
            continue;
          }
          const sequential = objectProperty(argument, 'sequential');
          if (!sequential) {
            continue;
          }
          if (
            sequential.value.type === 'BooleanLiteral' &&
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
            p.node,
            'formatted-titles',
            'Review generated test-title snapshots; v5 uses pretty-format and different string placeholders.',
          );
        }
      }

      if (name === 'vi' || name === 'vitest') {
        const method = memberName(p.node.callee);
        if (
          ['mock', 'unmock', 'hoisted'].includes(method ?? '') &&
          !p.parentPath.isExpressionStatement()
        ) {
          // A top-level variable initializer for vi.hoisted is valid too.
          const statement = p.getStatementParent();
          if (p.getFunctionParent() || !statement?.parentPath.isProgram()) {
            editor.report(
              p.node,
              'nested-hoisted-mock',
              'Move this hoisted mock to the top level after reviewing captured scope.',
            );
          }
        } else if (
          ['mock', 'unmock', 'hoisted'].includes(method ?? '') &&
          !p.parentPath.parentPath?.isProgram()
        ) {
          editor.report(
            p.node,
            'nested-hoisted-mock',
            'Move this hoisted mock to the top level after reviewing captured scope.',
          );
        }
        if (
          (options.browser || (options.browser === undefined && options.browserPossible)) &&
          method === 'mock' &&
          p.node.arguments.length === 1
        ) {
          editor.report(
            p.node,
            'browser-automock',
            'Browser automocks now keep mock defaults; choose { spy: true } if real implementations are required.',
          );
        }
        if ((method === 'fn' || method === 'spyOn') && looksLikeConstructorMock(p)) {
          editor.report(
            p.node,
            'class-mock',
            'If this mock replaces a constructor, review its prototype, methods, and instanceof behavior.',
          );
        }
        if (method === 'setSystemTime' && /\bTemporal\b/.test(source)) {
          editor.report(
            p.node,
            'temporal-system-time',
            'vi.setSystemTime now changes Temporal even without fake timers; toNotFake does not preserve this behavior.',
          );
        }
      }
      if (memberName(p.node.callee) === 'mockImplementation' && looksLikeConstructorMock(p)) {
        editor.report(
          p.node,
          'class-mock',
          'Review constructor mock implementations and their inherited prototypes.',
        );
      }

      if (name === 'expect') {
        const matcher = memberName(p.node.callee);
        const first = p.node.arguments[0];
        if (
          options.preserveV4 &&
          ['toThrow', 'toThrowError'].includes(matcher ?? '') &&
          first?.type === 'StringLiteral' &&
          first.value === ''
        ) {
          editor.replace(first, '/^$/');
        }
        if ((options.preserveV4 || options.reviewV4) && matcher === 'toHaveTextContent') {
          const browserAssertion = members.includes('element') || options.browser === true;
          const literal = first?.type === 'StringLiteral' || first?.type === 'RegExpLiteral';
          if (options.preserveV4 && browserAssertion && literal) {
            const callee = p.node.callee as t.MemberExpression;
            editor.replace(
              callee.property,
              callee.computed ? JSON.stringify('toMatchTextContent') : 'toMatchTextContent',
            );
          } else if (browserAssertion && !literal) {
            editor.report(
              p.node,
              'text-content',
              'Choose toMatchTextContent for v4 partial/regex matching after resolving the expected value.',
            );
          } else if (options.browser === undefined && options.browserPossible) {
            editor.report(
              p.node,
              'text-content-project',
              "Resolve this assertion's test project: browser assertions need toMatchTextContent for v4 partial matching; keep Node jest-dom assertions unchanged.",
            );
          }
        }
        if (members.includes('poll') && matcher !== 'poll') {
          editor.report(
            p.node,
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
        if (asynchronous && p.parentPath.isExpressionStatement()) {
          if (canAwait(p)) {
            editor.edit(p.node.start!, p.node.start!, 'await ');
          } else {
            editor.report(
              p.node,
              'unawaited-assertion',
              'Await or return this asynchronous assertion in an async-compatible function.',
            );
          }
        } else if (
          asynchronous &&
          !p.parentPath.isMemberExpression() &&
          !p.parentPath.isAwaitExpression() &&
          !p.parentPath.isReturnStatement()
        ) {
          editor.report(
            p.node,
            'unawaited-assertion',
            'Check that the result of this asynchronous assertion is awaited or returned.',
          );
        }
      }

      if (
        importedName(p, p.node.callee, RENDER_SOURCES) === 'render' &&
        !p.parentPath.isAwaitExpression() &&
        !p.parentPath.isReturnStatement()
      ) {
        if (canAwait(p)) {
          editor.edit(p.node.start!, p.node.start!, '(await ');
          editor.edit(p.node.end!, p.node.end!, ')');
        } else {
          editor.report(
            p.node,
            'async-render',
            'Await render from vitest-browser-vue/svelte after making the enclosing contract async-compatible.',
          );
        }
      }

      if (importedName(p, p.node.callee, NODE_SOURCES) === 'resolveConfig' && options.preserveV4) {
        const awaitPath = p.parentPath;
        const declaration = awaitPath.parentPath;
        if (
          awaitPath.isAwaitExpression() &&
          declaration?.isVariableDeclarator() &&
          declaration.node.id.type === 'ObjectPattern'
        ) {
          const props = declaration.node.id.properties;
          if (
            props.every(
              (prop) =>
                prop.type === 'ObjectProperty' &&
                !prop.computed &&
                prop.value.type === 'Identifier' &&
                ['viteConfig', 'vitestConfig'].includes(propertyName(prop.key) ?? ''),
            )
          ) {
            const vite = props.find(
              (prop) => prop.type === 'ObjectProperty' && propertyName(prop.key) === 'viteConfig',
            ) as t.ObjectProperty | undefined;
            const vitest = props.find(
              (prop) => prop.type === 'ObjectProperty' && propertyName(prop.key) === 'vitestConfig',
            ) as t.ObjectProperty | undefined;
            const local = vite
              ? editor.text(vite.value)
              : declaration.scope.generateUidIdentifier('viteConfig').name;
            editor.replace(declaration.node.id, local);
            if (vitest) {
              editor.edit(
                declaration.node.end!,
                declaration.node.end!,
                `, ${editor.text(vitest.value)} = ${local}.test`,
              );
            }
          } else {
            editor.report(
              declaration.node,
              'resolve-config',
              'Replace resolveConfig pair destructuring with the Vite config return value and its .test property.',
            );
          }
        } else {
          editor.report(
            p.node,
            'resolve-config',
            'Review resolveConfig consumers; the v5 return value is the Vite config with Vitest options under .test.',
          );
        }
      }

      if (memberName(p.node.callee) === 'collect') {
        const object = (p.node.callee as t.MemberExpression).object;
        const binding = object.type === 'Identifier' ? p.scope.getBinding(object.name) : undefined;
        let initializer =
          binding?.constant && binding.path.isVariableDeclarator()
            ? binding.path.node.init
            : undefined;
        if (initializer?.type === 'AwaitExpression') {
          initializer = initializer.argument;
        }
        const known =
          initializer?.type === 'CallExpression' &&
          ['createVitest', 'startVitest'].includes(
            importedName(binding!.path, initializer.callee, NODE_SOURCES) ?? '',
          );
        // collect(filters?, options?) places staticParse in the second argument.
        const opts = p.node.arguments[1];
        if (
          known &&
          options.preserveV4 &&
          p.node.arguments.length <= 2 &&
          !p.node.arguments.some((argument) => argument.type === 'SpreadElement') &&
          (!opts || staticObject(opts))
        ) {
          if (staticObject(opts)) {
            editor.add(opts, 'staticParse', 'false');
          } else {
            editor.edit(
              p.node.end! - 1,
              p.node.end! - 1,
              `${p.node.arguments.length ? ', ' : 'undefined, '}{ staticParse: false }`,
            );
          }
        } else if (
          !known ||
          p.node.arguments.some((argument) => argument.type === 'SpreadElement') ||
          (opts && !staticObject(opts))
        ) {
          editor.report(
            p.node,
            'static-collect',
            'If this is Vitest.collect(), review staticParse and set it to false to retain runtime collection.',
          );
        }
      }
      if (
        (p.node.callee.type === 'Import' ||
          (p.node.callee.type === 'Identifier' &&
            p.node.callee.name === 'require' &&
            !p.scope.getBinding('require'))) &&
        p.node.arguments[0]?.type === 'StringLiteral'
      ) {
        const source = p.node.arguments[0].value;
        if (
          REMOVED_SOURCES.has(source) ||
          RUNNER_SOURCES.has(source) ||
          EXPECT_SOURCES.has(source) ||
          RUNNERS_SOURCES.has(source)
        ) {
          unsupported(p.node, source, 'dynamic/CommonJS import', false);
        }
        if (source === '@vitest/ws-client') {
          editor.report(
            p.node,
            'ws-client',
            'Replace direct @vitest/ws-client use; it does not receive Vitest v5 features.',
          );
        }
      }
    },
    MemberExpression(p) {
      if (['VITEST_POOL_ID', 'VITEST_WORKER_ID'].includes(memberName(p.node) ?? '')) {
        editor.report(
          p.node,
          'worker-id',
          'Review worker/pool ID arithmetic and indexing: IDs now start at 1, not 0.',
        );
      }
    },
    VariableDeclarator(p) {
      if (p.node.id.type === 'ObjectPattern') {
        for (const prop of p.node.id.properties) {
          if (
            prop.type === 'ObjectProperty' &&
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
    AssignmentExpression(p) {
      if (/\boriginals\b/.test(editor.text(p.node.right))) {
        editor.report(
          p.node,
          'global-descriptors',
          'populateGlobal().originals stores descriptors; restore them with Object.defineProperty, not assignment.',
        );
      }
      const left = p.node.left;
      if (
        (left.type === 'MemberExpression' &&
          ['window', 'globalThis', 'global'].includes(editor.text(left.object)) &&
          DOM_GLOBALS.has(memberName(left) ?? '')) ||
        (left.type === 'Identifier' && DOM_GLOBALS.has(left.name))
      ) {
        editor.report(
          p.node,
          'dom-global',
          'In jsdom/happy-dom, global assignment also updates the window; review this DOM override.',
        );
      }
    },
    TSInterfaceDeclaration(p) {
      if (
        ['Assertion', 'Matchers'].includes(p.node.id.name) &&
        (p.node.typeParameters?.params.length ?? 0) < 2
      ) {
        editor.report(
          p.node,
          'assertion-types',
          'Update custom matcher declarations for the v5 return and received type parameters; review jest.Matchers augmentations.',
        );
      }
    },
    TSTypeReference(p) {
      const name = editor.text(p.node.typeName);
      if (
        ['Assertion', 'Matchers', 'jest.Matchers'].includes(name) &&
        (p.node.typeParameters?.params.length ?? 0) < 2
      ) {
        editor.report(
          p.node,
          'assertion-types',
          'Review old assertion generics; v5 includes return and received types.',
        );
      }
    },
    TSImportType(p) {
      const source = p.node.argument.value;
      if (
        RUNNER_SOURCES.has(source) ||
        EXPECT_SOURCES.has(source) ||
        REMOVED_SOURCES.has(source) ||
        RUNNERS_SOURCES.has(source)
      ) {
        unsupported(p.node, source, 'type import', true);
      }
    },
    StringLiteral(p) {
      const value = p.node.value;
      if (/\/__vitest__\//.test(value) && !/[?&]token=/.test(value)) {
        editor.report(
          p.node,
          'ui-token',
          'Use the authenticated UI URL printed by Vitest, including its token.',
        );
      }
      if (/\/__vitest_test__\//.test(value) && !/[?&]sessionId=/.test(value)) {
        editor.report(
          p.node,
          'browser-session',
          'Use the browser orchestrator URL opened by Vitest, including its sessionId.',
        );
      }
      if (/\.vitest-attachements|\.vitest-reports|__screenshots__|html\/index\.html/.test(value)) {
        editor.report(
          p.node,
          'artifact-paths',
          'Review this old artifact/report path. Generated output moved under .vitest; reference screenshots remain separate.',
        );
      }
    },
  });
  return editor.finish();
}
