import type {
  BooleanLiteral,
  Node,
  ObjectExpression,
  ObjectProperty,
  Program,
  RegExpLiteral,
  Span,
  StringLiteral,
} from '@oxc-project/types';

import { analyzeMigrationSource } from '../../binding/index.js';

interface SourceAnalysis {
  ast: { node: Program };
  comments: Array<Span>;
  bindings: Array<{ start: number; references: Array<number>; constant: boolean }>;
}

interface ParsedSource {
  program: Program;
  bindings: SourceAnalysis['bindings'];
  comments: Array<Span>;
}

export function isString(node: Node | null | undefined): node is StringLiteral {
  return node?.type === 'Literal' && typeof node.value === 'string';
}

export function isBoolean(node: Node | null | undefined): node is BooleanLiteral {
  return node?.type === 'Literal' && typeof node.value === 'boolean';
}

export function isRegExp(node: Node | null | undefined): node is RegExpLiteral {
  return node?.type === 'Literal' && 'regex' in node;
}

export function propertyName(node: Node | null | undefined): string | undefined {
  if (node?.type === 'Identifier') {
    return node.name;
  }
  if (isString(node)) {
    return node.value;
  }
  return undefined;
}

export function memberName(node: Node | null | undefined): string | undefined {
  if (node?.type !== 'MemberExpression') {
    return undefined;
  }
  if (node.computed && !isString(node.property)) {
    return undefined;
  }
  return propertyName(node.property);
}

export function objectProperty(object: ObjectExpression, key: string): ObjectProperty | undefined {
  return object.properties.find(
    (prop): prop is ObjectProperty =>
      prop.type === 'Property' &&
      prop.kind === 'init' &&
      !prop.computed &&
      propertyName(prop.key) === key,
  );
}

export function staticObject(
  node: Node | null | undefined,
  allowMethods = false,
): node is ObjectExpression {
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
export function sameStaticValue(left: Node, right: Node): boolean {
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
        if (property.type !== 'Property') {
          return false;
        }
        const key = propertyName(property.key);
        const other = key === undefined ? undefined : objectProperty(right, key);
        return !!other && sameStaticValue(property.value, other.value);
      })
    );
  }
  return false;
}

export function parseSource(file: string, source: string): ParsedSource {
  // Native errors (including unsupported Flow) reach the existing preflight
  // diagnostic path, which preserves the original file for manual review.
  // The native migration binding serializes exactly SourceAnalysis.
  // oxlint-disable-next-line typescript/no-unsafe-assignment
  const { ast, bindings, comments }: SourceAnalysis = JSON.parse(
    analyzeMigrationSource(file, source),
  );
  // Migration uses source spelling and regex metadata, not the JS-only fixes
  // that reconstruct RegExp/BigInt values in Oxc's serialized AST.
  return { program: ast.node, bindings, comments };
}
