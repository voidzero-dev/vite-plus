import { parse } from '@babel/parser';
import traverseModule, { type NodePath } from '@babel/traverse';
import type * as t from '@babel/types';

// Babel 7's CommonJS default has one extra level under native Node ESM.
export const traverse: typeof traverseModule.default =
  typeof traverseModule === 'function' ? traverseModule : traverseModule.default;

export interface VitestV5Finding {
  file: string;
  line: number;
  column: number;
  code: string;
  severity: 'review' | 'block';
  message: string;
}

export interface SourceOptions {
  preserveV4: boolean;
  reviewV4?: boolean;
  browser?: boolean;
  browserPossible?: boolean;
  globals?: boolean;
  temporalPolyfill?: boolean;
}

export function propertyName(node: t.Node | null | undefined): string | undefined {
  if (node?.type === 'Identifier') {
    return node.name;
  }
  if (node?.type === 'StringLiteral') {
    return node.value;
  }
  return undefined;
}

export function memberName(node: t.Node | null | undefined): string | undefined {
  if (node?.type !== 'MemberExpression' && node?.type !== 'OptionalMemberExpression') {
    return undefined;
  }
  if (node.computed && node.property.type !== 'StringLiteral') {
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
      prop.type === 'ObjectProperty' && !prop.computed && propertyName(prop.key) === key,
  );
}

export function staticObject(node: t.Node | null | undefined): node is t.ObjectExpression {
  if (node?.type !== 'ObjectExpression') {
    return false;
  }
  const names = new Set<string>();
  return node.properties.every((prop) => {
    if (prop.type !== 'ObjectProperty' || prop.computed) {
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

export function importedName(
  path: NodePath,
  node: t.Node | null | undefined,
  sources: ReadonlySet<string>,
): string | undefined {
  if (node?.type === 'MemberExpression' && !node.computed && node.object.type === 'Identifier') {
    const binding = path.scope.getBinding(node.object.name);
    if (
      binding?.path.isImportNamespaceSpecifier() &&
      binding.path.parentPath.isImportDeclaration() &&
      sources.has(binding.path.parentPath.node.source.value)
    ) {
      return propertyName(node.property);
    }
  }
  if (node?.type !== 'Identifier') {
    return undefined;
  }
  const binding = path.scope.getBinding(node.name);
  if (!binding?.path.isImportSpecifier() || !binding.path.parentPath.isImportDeclaration()) {
    return undefined;
  }
  if (
    binding.path.node.importKind === 'type' ||
    binding.path.parentPath.node.importKind === 'type'
  ) {
    return undefined;
  }
  if (sources.has(binding.path.parentPath.node.source.value)) {
    return propertyName(binding.path.node.imported);
  }
  return undefined;
}

export const ROOT_TEST_SOURCES = new Set(['vitest', 'vite-plus/test', '@vitest/runner']);
export const CONFIG_SOURCES = new Set([
  'vitest/config',
  'vite-plus',
  'vite-plus/test/config',
  'vite',
]);
export const NODE_SOURCES = new Set(['vitest/node', 'vite-plus/test/node']);

function parseSource(file: string, source: string) {
  return parse(source, {
    sourceType: 'unambiguous',
    plugins: [/\.[cm]?tsx?$/.test(file) ? 'typescript' : 'flow', 'jsx', 'decorators-legacy'],
    tokens: true,
  });
}

export function testApiName(
  path: NodePath,
  node: t.Node | null | undefined,
  globals = false,
): string | undefined {
  const imported = importedName(path, node, ROOT_TEST_SOURCES);
  if (imported) {
    return imported;
  }
  if (globals && node?.type === 'Identifier' && !path.scope.getBinding(node.name)) {
    return node.name;
  }
  return undefined;
}

/** Offset edits retain comments and formatting outside the precise changed span. */
export class SourceEditor {
  readonly ast: ReturnType<typeof parse>;
  readonly findings: VitestV5Finding[] = [];
  private readonly edits: Array<{ start: number; end: number; text: string }> = [];
  private readonly additions = new Map<t.ObjectExpression, Map<string, string>>();

  constructor(
    readonly file: string,
    readonly source: string,
  ) {
    this.ast = parseSource(file, source);
  }

  text(node: t.Node) {
    return this.source.slice(node.start!, node.end!);
  }

  replace(node: t.Node, text: string) {
    this.edit(node.start!, node.end!, text);
  }

  edit(start: number, end: number, text: string) {
    const same = this.edits.find((edit) => edit.start === start && edit.end === end);
    if (same?.text === text) {
      return;
    }
    if (same && start === end) {
      same.text += text;
      return;
    }
    this.edits.push({ start, end, text });
  }

  add(object: t.ObjectExpression, key: string, value: string) {
    if (objectProperty(object, key)) {
      return;
    }
    let additions = this.additions.get(object);
    if (!additions) {
      this.additions.set(object, (additions = new Map()));
    }
    additions.set(key, value);
  }

  remove(object: t.ObjectExpression, prop: t.ObjectProperty) {
    const index = object.properties.indexOf(prop);
    const nextStart = object.properties[index + 1]?.start ?? object.end! - 1;
    const previousEnd = object.properties[index - 1]?.end ?? object.start! + 1;
    // Use punctuation tokens, not a text search that could consume a comment.
    const tokens = this.ast.tokens ?? [];
    const followingComma = tokens.find(
      (token) =>
        token.start >= prop.end! &&
        token.end <= nextStart &&
        this.source.slice(token.start, token.end) === ',',
    );
    const precedingComma = tokens.find(
      (token) =>
        token.start >= previousEnd &&
        token.end <= prop.start! &&
        this.source.slice(token.start, token.end) === ',',
    );
    this.replace(prop, '');
    const comma = followingComma ?? precedingComma;
    if (comma) {
      this.edit(comma.start, comma.end, '');
    }
  }

  report(
    node: t.Node | undefined,
    code: string,
    message: string,
    severity: VitestV5Finding['severity'] = 'review',
  ) {
    const line = node?.loc?.start.line ?? 1;
    const column = (node?.loc?.start.column ?? 0) + 1;
    if (
      !this.findings.some(
        (item) =>
          item.line === line &&
          item.column === column &&
          item.code === code &&
          item.message === message,
      )
    ) {
      this.findings.push({ file: this.file, line, column, code, severity, message });
    }
  }

  finish() {
    for (const [object, additions] of this.additions) {
      const start = object.start! + 1;
      const first = object.properties[0]?.start ?? object.end! - 1;
      const prefix = this.source.slice(start, first);
      const newline = prefix.includes('\r\n') ? '\r\n' : '\n';
      const multiline = prefix.indexOf('\n') !== -1;
      const indent =
        this.source.slice(this.source.lastIndexOf('\n', first) + 1, first).match(/^[\t ]*/)?.[0] ??
        '';
      const propertyIndent = object.properties.length ? indent : `${indent}  `;
      const entries = [...additions].map(([key, value]) => `${key}: ${value}`);
      this.edit(
        start,
        start,
        multiline
          ? `${newline}${propertyIndent}${entries.join(`,${newline}${propertyIndent}`)}${object.properties.length ? ',' : ''}`
          : ` ${entries.join(', ')}${object.properties.length ? ',' : ''} `,
      );
    }
    const edits = this.edits.toSorted((a, b) => b.start - a.start || b.end - a.end);
    let result = this.source;
    let previousStart = this.source.length + 1;
    for (const edit of edits) {
      if (edit.end > previousStart) {
        this.report(
          undefined,
          'overlapping-edits',
          'Review the overlapping Vitest transformations; this file was not changed.',
        );
        return { content: this.source, findings: this.findings };
      }
      result = result.slice(0, edit.start) + edit.text + result.slice(edit.end);
      previousStart = edit.start;
    }
    if (result !== this.source) {
      try {
        parseSource(this.file, result);
      } catch {
        this.report(
          undefined,
          'unsafe-syntax',
          'The proposed transform did not parse. Review this file manually; it was not changed.',
        );
        return { content: this.source, findings: this.findings };
      }
    }
    return { content: result, findings: this.findings };
  }
}
