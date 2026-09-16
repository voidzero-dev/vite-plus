import type * as t from '@oxc-project/types';

import { SourceEditor, isString, memberName, propertyName, testApiName } from './ast.ts';

interface BenchmarkCall {
  call: t.CallExpression;
  modifier?: string;
}

/** Keep the workload as a separate callback: making it the test callback
 * would measure nothing, and inlining it into a new closure can change scope. */
export function migrateBenchmarks(
  editor: SourceEditor,
  globals = false,
): { imports: Set<t.ImportSpecifier>; calls: Set<t.CallExpression> } {
  const imports = new Set<t.ImportSpecifier>();
  const calls = new Set<t.CallExpression>();
  const benchIdentifiers: t.Span[] = [];
  let fixtureAlias: string | undefined;
  let shadowedGlobalThis = false;
  editor.visit({
    Identifier(node) {
      if (node.name === 'bench') {
        benchIdentifiers.push(node);
      }
      if (node.name === 'globalThis' && editor.binding(node)) {
        shadowedGlobalThis = true;
      }
    },
  });

  function workload(node: t.Node, seen = new Set<t.Node>()): boolean {
    if (
      node.type === 'ArrowFunctionExpression' ||
      node.type === 'FunctionExpression' ||
      node.type === 'FunctionDeclaration'
    ) {
      return !!node.body && node.params.length === 0 && !node.generator;
    }
    if (node.type !== 'Identifier' || seen.has(node)) {
      return false;
    }
    seen.add(node);
    const binding = editor.binding(node);
    if (!binding?.constant) {
      return false;
    }
    const declaration = binding.declaration;
    if (declaration.type === 'FunctionDeclaration') {
      return workload(declaration, seen);
    }
    return (
      declaration.type === 'VariableDeclarator' &&
      !!declaration.init &&
      workload(declaration.init, seen)
    );
  }

  function directCall(reference: t.Node): BenchmarkCall | undefined {
    let callee = reference;
    let modifier: string | undefined;
    const member = editor.parent(reference);
    if (member?.type === 'MemberExpression' && member.object === reference) {
      modifier = memberName(member);
      if (member.optional || !modifier || !['skip', 'only', 'todo'].includes(modifier)) {
        return undefined;
      }
      callee = member;
    }
    const call = editor.parent(callee);
    if (
      call?.type !== 'CallExpression' ||
      call.callee !== callee ||
      call.optional ||
      call.typeArguments ||
      editor.parent(call)?.type !== 'ExpressionStatement' ||
      !call.arguments[0] ||
      call.arguments[0].type === 'SpreadElement'
    ) {
      return undefined;
    }
    const callback = call.arguments[1];
    if (!(modifier === 'todo' && call.arguments.length === 1)) {
      if (call.arguments.length !== 2 || !callback || !workload(callback)) {
        return undefined;
      }
    }
    // Register only at module scope or inside ordinary describe/suite
    // callbacks. Calls from helpers, hooks, or other benchmarks need review.
    let fn = editor.functionParent(call);
    while (fn) {
      const registration = editor.parent(fn);
      if (
        fn.async ||
        fn.generator ||
        registration?.type !== 'CallExpression' ||
        registration.optional ||
        !registration.arguments.includes(fn)
      ) {
        return undefined;
      }
      let suite = registration.callee;
      if (!['describe', 'suite'].includes(testApiName(editor, suite, globals) ?? '')) {
        if (
          suite.type !== 'MemberExpression' ||
          suite.optional ||
          !['skip', 'only'].includes(memberName(suite) ?? '')
        ) {
          return undefined;
        }
        suite = suite.object;
      }
      if (!['describe', 'suite'].includes(testApiName(editor, suite, globals) ?? '')) {
        return undefined;
      }
      fn = editor.functionParent(registration);
    }
    return { call, modifier };
  }

  function rewrite({ call, modifier }: BenchmarkCall, test: string) {
    calls.add(call);
    editor.replace(call.callee, `${test}${modifier ? `.${modifier}` : ''}`);
    const callback = call.arguments[1];
    if (!callback) {
      return;
    }
    let name = editor.text(call.arguments[0]);
    const captures: string[] = [];
    if (!isString(call.arguments[0])) {
      const captured = editor.uniqueName('benchName');
      captures.push(`const ${captured} = (${name});`);
      editor.replace(call.arguments[0], captured);
      name = captured;
    }
    if (callback.type === 'Identifier') {
      const captured = editor.uniqueName('benchFn');
      captures.push(`const ${captured} = ${editor.text(callback)};`);
      editor.replace(callback, captured);
    }
    if (captures.length) {
      // Capture at registration, in the original lexical scope and argument
      // order. A test executes later, after names or callback bindings may change.
      const statement = editor.parent(call)!;
      editor.edit(statement.start, statement.start, `{ ${captures.join(' ')} `);
      editor.edit(statement.end, statement.end, ' }');
    }
    // Only inline workloads move into the fixture's scope. Named workloads
    // keep their original scope through the callback captured above.
    const shadowsBench =
      callback.type !== 'Identifier' &&
      benchIdentifiers.some((node) => node.start >= callback.start && node.end <= callback.end);
    let fixtureName = 'bench';
    if (shadowsBench) {
      fixtureName = fixtureAlias ??= editor.uniqueName('bench');
    }
    const fixture = fixtureName === 'bench' ? 'bench' : `bench: ${fixtureName}`;
    editor.edit(
      callback.start,
      callback.start,
      `async ({ ${fixture} }) => { await ${fixtureName}(${name}, `,
    );
    editor.edit(call.end - 1, call.end - 1, ').run(); }');
  }

  editor.visit({
    ImportDeclaration(node) {
      if (!['vitest', 'vite-plus/test'].includes(node.source.value) || node.importKind === 'type') {
        return;
      }
      for (const specifier of node.specifiers) {
        if (
          specifier.type !== 'ImportSpecifier' ||
          specifier.importKind === 'type' ||
          propertyName(specifier.imported) !== 'bench'
        ) {
          continue;
        }
        const binding = editor.binding(specifier.local);
        if (!binding) {
          continue;
        }
        const registrations = binding.references.map(directCall);
        // Retarget an import only when all uses are understood. A callback
        // passed to a wrapper or a re-export must not become a test function.
        if (registrations.some((registration) => !registration)) {
          continue;
        }
        const test = editor.uniqueName('test', true);
        editor.replace(specifier, test === 'test' ? 'test' : `test as ${test}`);
        imports.add(specifier);
        for (const registration of registrations) {
          rewrite(registration!, test);
        }
      }
    },
    Identifier(node) {
      if (!globals || node.name !== 'bench' || editor.binding(node) || shadowedGlobalThis) {
        return;
      }
      const registration = directCall(node);
      if (registration) {
        rewrite(registration, 'globalThis.test');
      }
    },
    MemberExpression(node) {
      if (testApiName(editor, node, globals) !== 'bench') {
        return;
      }
      const registration = directCall(node);
      if (registration && node.object.type === 'Identifier') {
        rewrite(registration, `${editor.text(node.object)}.test`);
      }
    },
  });
  return { imports, calls };
}
