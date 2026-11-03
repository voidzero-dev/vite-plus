import type * as t from '@oxc-project/types';

import {
  CONFIG_SOURCES,
  SourceEditor,
  importedName,
  isModuleExports,
  propertyName,
} from './ast.ts';

const CONFIG_KEYS = new Set(['root', 'test', 'extends']);

// A conditional spread of build/server options cannot change test ownership.
// Unknown spreads (including imported objects) must still be reviewed.
function unrelatedSpread(node: t.Node): boolean {
  if (node.type === 'ConditionalExpression') {
    return unrelatedSpread(node.consequent) && unrelatedSpread(node.alternate);
  }
  return (
    node.type === 'ObjectExpression' &&
    node.properties.every((property) =>
      property.type === 'SpreadElement'
        ? unrelatedSpread(property.argument)
        : !property.computed && !CONFIG_KEYS.has(propertyName(property.key) ?? 'test'),
    )
  );
}

/** Read a config without evaluating user code. Only a single unconditional
 * return is supported; nested callback returns and conditional exits are not
 * alternative configs. Return the original object so edits keep its spans. */
export function resolveConfigObject(
  editor: SourceEditor,
  node: t.Node | null | undefined,
): t.ObjectExpression | undefined {
  if (node?.type === 'Identifier') {
    const binding = editor.binding(node);
    if (
      binding?.references.some((reference) => {
        const parent = editor.parent(reference);
        return !(
          parent?.type === 'ExportDefaultDeclaration' ||
          (parent?.type === 'AssignmentExpression' &&
            parent.right === reference &&
            isModuleExports(editor, parent.left)) ||
          (parent?.type === 'CallExpression' &&
            parent.arguments[0] === reference &&
            ['defineConfig', 'defineProject'].includes(
              importedName(editor, parent.callee, CONFIG_SOURCES) ?? '',
            ))
        );
      })
    ) {
      return undefined;
    }
    node =
      binding?.constant && binding.declaration.type === 'VariableDeclarator'
        ? binding.declaration.init
        : undefined;
  }
  if (node?.type === 'CallExpression') {
    node = ['defineConfig', 'defineProject'].includes(
      importedName(editor, node.callee, CONFIG_SOURCES) ?? '',
    )
      ? node.arguments[0]
      : undefined;
  }
  if (
    node?.type === 'ArrowFunctionExpression' ||
    node?.type === 'FunctionExpression' ||
    node?.type === 'FunctionDeclaration'
  ) {
    if (node.generator) {
      return undefined;
    }
    const body = node.body;
    if (!body) {
      return undefined;
    }
    if (body.type === 'BlockStatement') {
      const last = body.body.at(-1);
      // Do not follow returned aliases: a constant binding does not establish
      // that its object was not mutated or passed to another function.
      if (
        last?.type !== 'ReturnStatement' ||
        !body.body.slice(0, -1).every((statement) => statement.type === 'VariableDeclaration')
      ) {
        return undefined;
      }
      node = last.argument;
    } else {
      node = body;
    }
  }
  if (node?.type !== 'ObjectExpression') {
    return undefined;
  }
  const names = new Set<string>();
  for (const property of node.properties) {
    if (property.type === 'SpreadElement') {
      if (!unrelatedSpread(property.argument)) {
        return undefined;
      }
      continue;
    }
    const name = propertyName(property.key);
    if (property.computed || property.kind !== 'init' || name === undefined || names.has(name)) {
      return undefined;
    }
    names.add(name);
  }
  return node;
}
