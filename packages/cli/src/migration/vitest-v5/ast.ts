import type * as t from '@oxc-project/types';

import { analyzeMigrationSource } from '../../../binding/index.js';

interface SourceAnalysis {
  ast: { node: t.Program };
  comments: t.Span[];
  bindings: Array<{ start: number; references: number[]; constant: boolean }>;
}

interface Binding {
  declaration: t.Node;
  references: t.Node[];
  constant: boolean;
}

type Visitors = {
  [Type in t.Node['type']]?: (node: Extract<t.Node, { type: Type }>) => void;
};

function isNode(value: unknown): value is t.Node {
  return typeof value === 'object' && value !== null && 'type' in value && 'start' in value;
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
      !prop.method &&
      prop.kind === 'init' &&
      !prop.computed &&
      propertyName(prop.key) === key,
  );
}

export function staticObject(node: t.Node | null | undefined): node is t.ObjectExpression {
  if (node?.type !== 'ObjectExpression') {
    return false;
  }
  const names = new Set<string>();
  return node.properties.every((prop) => {
    if (prop.type !== 'Property' || prop.method || prop.kind !== 'init' || prop.computed) {
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
  editor: SourceEditor,
  node: t.Node | null | undefined,
  sources: ReadonlySet<string>,
): string | undefined {
  if (
    node?.type === 'MemberExpression' &&
    !node.optional &&
    !node.computed &&
    node.object.type === 'Identifier'
  ) {
    const declaration = editor.binding(node.object)?.declaration;
    const parent = declaration && editor.parent(declaration);
    if (
      declaration?.type === 'ImportNamespaceSpecifier' &&
      parent?.type === 'ImportDeclaration' &&
      parent.importKind !== 'type' &&
      sources.has(parent.source.value)
    ) {
      return propertyName(node.property);
    }
  }
  if (node?.type !== 'Identifier') {
    return undefined;
  }
  const declaration = editor.binding(node)?.declaration;
  const parent = declaration && editor.parent(declaration);
  if (declaration?.type !== 'ImportSpecifier' || parent?.type !== 'ImportDeclaration') {
    return undefined;
  }
  if (declaration.importKind === 'type' || parent.importKind === 'type') {
    return undefined;
  }
  if (sources.has(parent.source.value)) {
    return propertyName(declaration.imported);
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

export function parseSource(file: string, source: string) {
  // Native errors (including unsupported Flow) reach the existing preflight
  // diagnostic path, which preserves the original file for manual review.
  const { ast, bindings, comments } = JSON.parse(
    analyzeMigrationSource(file, source),
  ) as SourceAnalysis;
  // Migration uses source spelling and regex metadata, not the JS-only fixes
  // that reconstruct RegExp/BigInt values in Oxc's serialized AST.
  return { program: ast.node, bindings, comments };
}

export function testApiName(
  editor: SourceEditor,
  node: t.Node | null | undefined,
  globals = false,
): string | undefined {
  const imported = importedName(editor, node, ROOT_TEST_SOURCES);
  if (imported) {
    return imported;
  }
  if (globals && node?.type === 'Identifier' && !editor.binding(node)) {
    return node.name;
  }
  return undefined;
}

/** Offset edits retain comments and formatting outside the precise changed span. */
export class SourceEditor {
  readonly ast: t.Program;
  readonly findings: VitestV5Finding[] = [];
  private readonly nodes: t.Node[] = [];
  private readonly parents = new Map<t.Node, t.Node>();
  private readonly bindings = new Map<number, Binding>();
  private readonly names = new Set<string>();
  private readonly comments: t.Span[];
  private readonly edits: Array<{ start: number; end: number; text: string }> = [];
  private readonly additions = new Map<t.ObjectExpression, Map<string, string>>();

  constructor(
    readonly file: string,
    readonly source: string,
  ) {
    const parsed = parseSource(file, source);
    this.ast = parsed.program;
    this.comments = parsed.comments;
    const identifiers = new Map<number, t.Node>();
    const index = (node: t.Node, parent?: t.Node) => {
      this.nodes.push(node);
      if (parent) {
        this.parents.set(node, parent);
      }
      if (node.type === 'Identifier') {
        identifiers.set(node.start, node);
        this.names.add(node.name);
      }
      for (const value of Object.values(node)) {
        for (const child of Array.isArray(value) ? value : [value]) {
          if (isNode(child)) {
            index(child, node);
          }
        }
      }
    };
    index(this.ast);
    for (const binding of parsed.bindings) {
      const identifier = identifiers.get(binding.start);
      const declaration = identifier && this.parent(identifier);
      if (!declaration) {
        continue;
      }
      const references = binding.references.map((start) => identifiers.get(start)).filter(isNode);
      const resolved = { declaration, references, constant: binding.constant };
      for (const start of [binding.start, ...binding.references]) {
        this.bindings.set(start, resolved);
      }
    }
  }

  visit(visitors: Visitors): void {
    for (const node of this.nodes) {
      // Dispatch by the same discriminant that selects the visitor's node type.
      const visitor = visitors[node.type] as ((node: t.Node) => void) | undefined;
      visitor?.(node);
    }
  }

  parent(node: t.Node): t.Node | undefined {
    return this.parents.get(node);
  }

  functionParent(node: t.Node): t.Function | t.ArrowFunctionExpression | undefined {
    let parent = this.parent(node);
    while (parent) {
      if (
        parent.type === 'FunctionDeclaration' ||
        parent.type === 'FunctionExpression' ||
        parent.type === 'ArrowFunctionExpression'
      ) {
        return parent;
      }
      parent = this.parent(parent);
    }
    return undefined;
  }

  binding(node: t.Node): Binding | undefined {
    return node.type === 'Identifier' ? this.bindings.get(node.start) : undefined;
  }

  uniqueName(base: string): string {
    let name = `_${base}`;
    let suffix = 2;
    while (this.names.has(name)) {
      name = `_${base}${suffix++}`;
    }
    this.names.add(name);
    return name;
  }

  text(node: t.Node) {
    return this.source.slice(node.start, node.end);
  }

  replace(node: t.Node, text: string) {
    this.edit(node.start, node.end, text);
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
    const nextStart = object.properties[index + 1]?.start ?? object.end - 1;
    const previousEnd = object.properties[index - 1]?.end ?? object.start + 1;
    // Between complete property spans only trivia and punctuation can occur.
    // Skip Oxc's comment spans so a comma inside a comment is never removed.
    const commaBetween = (start: number, end: number) => {
      for (let index = start; index < end; index++) {
        const comment = this.comments.find((span) => span.start <= index && index < span.end);
        if (comment) {
          index = comment.end - 1;
        } else if (this.source[index] === ',') {
          return index;
        }
      }
      return undefined;
    };
    const followingComma = commaBetween(prop.end, nextStart);
    const precedingComma = commaBetween(previousEnd, prop.start);
    this.replace(prop, '');
    const comma = followingComma ?? precedingComma;
    if (comma !== undefined) {
      this.edit(comma, comma + 1, '');
    }
  }

  report(
    node: t.Node | undefined,
    code: string,
    message: string,
    severity: VitestV5Finding['severity'] = 'review',
  ) {
    const prefix = this.source.slice(0, node?.start ?? 0);
    const lines = prefix.split(/\r\n|[\r\n\u2028\u2029]/);
    const line = lines.length;
    const column = lines.at(-1)!.length + 1;
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
      const start = object.start + 1;
      const first = object.properties[0]?.start ?? object.end - 1;
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
