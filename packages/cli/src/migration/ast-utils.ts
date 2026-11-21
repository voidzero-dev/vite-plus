import type * as t from '@oxc-project/types';

import { analyzeMigrationSource } from '../../binding/index.js';

interface SourceAnalysis {
  ast: { node: t.Program };
  comments: t.Span[];
  bindings: Array<{ start: number; references: number[]; constant: boolean }>;
}

interface ParsedSource {
  program: t.Program;
  bindings: SourceAnalysis['bindings'];
  comments: t.Span[];
}

export function isString(node: t.Node | null | undefined): node is t.StringLiteral {
  return node?.type === 'Literal' && typeof node.value === 'string';
}

export function isBoolean(node: t.Node | null | undefined): node is t.BooleanLiteral {
  return node?.type === 'Literal' && typeof node.value === 'boolean';
}

export function isRegExp(node: t.Node | null | undefined): node is t.RegExpLiteral {
  return node?.type === 'Literal' && 'regex' in node;
}

export function propertyName(node: t.Node | null | undefined): string | undefined {
  if (node?.type === 'Identifier') {
    return node.name;
  }
  if (isString(node)) {
    return node.value;
  }
  return undefined;
}

export function memberName(node: t.Node | null | undefined): string | undefined {
  if (node?.type !== 'MemberExpression') {
    return undefined;
  }
  if (node.computed && !isString(node.property)) {
    return undefined;
  }
  return propertyName(node.property);
}

export function objectProperty(
  object: t.ObjectExpression,
  key: string,
): t.ObjectProperty | undefined {
  return object.properties.find(
    (prop): prop is t.ObjectProperty =>
      prop.type === 'Property' &&
      prop.kind === 'init' &&
      !prop.computed &&
      propertyName(prop.key) === key,
  );
}

export function staticObject(
  node: t.Node | null | undefined,
  allowMethods = false,
): node is t.ObjectExpression {
  if (node?.type !== 'ObjectExpression') {
    return false;
  }
  const names = new Set<string>();
  return node.properties.every((prop) => {
    if (
      prop.type !== 'Property' ||
      (prop.method && !allowMethods) ||
      prop.kind !== 'init' ||
      prop.computed
    ) {
      return false;
    }
    const name = propertyName(prop.key);
    if (name === undefined || names.has(name)) {
      return false;
    }
    names.add(name);
    return true;
  });
}

/** Compare data literals without evaluating config code or invoking getters.
 * Property order and spelling do not change the value of a plain config object. */
export function sameStaticValue(left: t.Node, right: t.Node): boolean {
  if (left.type === 'Literal' && right.type === 'Literal') {
    return (
      !('regex' in left || 'regex' in right || 'bigint' in left || 'bigint' in right) &&
      Object.is(left.value, right.value)
    );
  }
  if (left.type === 'UnaryExpression' && right.type === 'UnaryExpression') {
    return (
      ['+', '-'].includes(left.operator) &&
      left.operator === right.operator &&
      left.argument.type === 'Literal' &&
      typeof left.argument.value === 'number' &&
      sameStaticValue(left.argument, right.argument)
    );
  }
  if (left.type === 'ArrayExpression' && right.type === 'ArrayExpression') {
    return (
      left.elements.length === right.elements.length &&
      left.elements.every((element, index) => {
        const other = right.elements[index];
        return !!element && !!other && sameStaticValue(element, other);
      })
    );
  }
  if (staticObject(left) && staticObject(right)) {
    return (
      left.properties.length === right.properties.length &&
      left.properties.every((property) => {
        const prop = property as t.ObjectProperty;
        const other = objectProperty(right, propertyName(prop.key)!);
        return !!other && sameStaticValue(prop.value, other.value);
      })
    );
  }
  return false;
}

export function parseSource(file: string, source: string): ParsedSource {
  // Native errors (including unsupported Flow) reach the existing preflight
  // diagnostic path, which preserves the original file for manual review.
  const { ast, bindings, comments } = JSON.parse(
    analyzeMigrationSource(file, source),
  ) as SourceAnalysis;
  // Migration uses source spelling and regex metadata, not the JS-only fixes
  // that reconstruct RegExp/BigInt values in Oxc's serialized AST.
  return { program: ast.node, bindings, comments };
}

export function defaultConfigValue({ program, bindings }: ParsedSource): t.Node | undefined {
  const declaration = program.body.find(
    (statement) => statement.type === 'ExportDefaultDeclaration',
  )?.declaration;
  if (!declaration) {
    return undefined;
  }
  let exported = unwrapTypeExpression(declaration);
  // Follow private constants that are used only to pass the config to its export.
  while (exported.type === 'Identifier') {
    const reference = exported.start;
    const binding = bindings.find(
      ({ constant, references }) =>
        constant && references.length === 1 && references[0] === reference,
    );
    const alias =
      binding &&
      program.body
        .flatMap((statement) =>
          statement.type === 'VariableDeclaration' && statement.kind === 'const'
            ? statement.declarations
            : [],
        )
        .find(({ id, end }) => id.start === binding.start && end <= reference);
    if (!alias?.init || alias.id.type !== 'Identifier') {
      break;
    }
    exported = unwrapTypeExpression(alias.init);
  }
  if (
    exported?.type === 'CallExpression' &&
    exported.callee.type === 'Identifier' &&
    exported.callee.name === 'defineConfig' &&
    exported.arguments.length === 1
  ) {
    return unwrapTypeExpression(exported.arguments[0]);
  }
  return exported;
}

export function unwrapTypeExpression(node: t.Node): t.Node {
  while (
    node.type === 'TSAsExpression' ||
    node.type === 'TSSatisfiesExpression' ||
    node.type === 'TSNonNullExpression' ||
    node.type === 'TSTypeAssertion'
  ) {
    node = node.expression;
  }
  return node;
}

/** Read direct callback returns without evaluating the config or entering nested functions. */
export function configReturnValues(config: t.Node | undefined): (t.Node | null)[] {
  if (!config) {
    return [];
  }
  if (config.type !== 'ArrowFunctionExpression' && config.type !== 'FunctionExpression') {
    return [config];
  }
  const body = config.body;
  if (!body) {
    return [];
  }
  if (body.type !== 'BlockStatement') {
    return [unwrapTypeExpression(body)];
  }
  // A final top-level return ensures a normal call does not fall through.
  if (body.body.at(-1)?.type !== 'ReturnStatement') {
    return [];
  }
  const values: (t.Node | null)[] = [];
  const pending: unknown[] = [...body.body];
  while (pending.length > 0) {
    const item = pending.pop();
    if (Array.isArray(item)) {
      pending.push(...item);
    } else if (typeof item === 'object' && item !== null && 'type' in item) {
      const node = item as t.Node;
      if (node.type === 'ReturnStatement') {
        values.push(node.argument ? unwrapTypeExpression(node.argument) : null);
      } else if (
        node.type !== 'ArrowFunctionExpression' &&
        node.type !== 'FunctionExpression' &&
        node.type !== 'FunctionDeclaration'
      ) {
        pending.push(...Object.values(node));
      }
    }
  }
  return values;
}

export function sameConfigModule(file: string, source: string, other: string): boolean {
  const syntax = (text: string) =>
    JSON.stringify(parseSource(file, text).program, function (this: { type?: string }, key, value) {
      if (key === 'start' || key === 'end' || (key === 'raw' && this.type === 'Literal')) {
        return undefined;
      }
      return value;
    });
  return syntax(source) === syntax(other);
}
