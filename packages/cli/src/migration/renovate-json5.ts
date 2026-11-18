import type * as t from '@oxc-project/types';

import { detectFormattingOptions } from '../utils/json.ts';
import { isString, objectProperty, staticObject } from './ast-utils.ts';

// Oxc parses JavaScript. JSON5 has a smaller set of numeric literals and
// uses four hex digits for Unicode escapes, including in unquoted keys.
const JSON5_NUMBER =
  /^[+-]?(?:Infinity|NaN|0[xX][\da-fA-F]+|(?:0|[1-9]\d*)(?:\.\d*)?(?:[eE][+-]?\d+)?|\.\d+(?:[eE][+-]?\d+)?)$/;
const CODE_POINT_ESCAPE = /(?:^|[^\\])(?:\\\\)*\\u\{/;

function isJson5Value(node: t.Node, source: string): boolean {
  switch (node.type) {
    case 'ObjectExpression':
      return (
        staticObject(node) &&
        node.properties.every(
          (prop) =>
            prop.type === 'Property' &&
            !prop.shorthand &&
            !CODE_POINT_ESCAPE.test(source.slice(prop.key.start, prop.key.end)) &&
            isJson5Value(prop.value, source),
        )
      );
    case 'ArrayExpression':
      return node.elements.every((item) => item !== null && isJson5Value(item, source));
    case 'Literal':
      if (isString(node)) {
        return !CODE_POINT_ESCAPE.test(source.slice(node.start, node.end));
      }
      if (node.raw === 'null' || typeof node.value === 'boolean') {
        return true;
      }
      break;
    case 'UnaryExpression':
    case 'Identifier':
      break;
    default:
      return false;
  }
  return JSON5_NUMBER.test(source.slice(node.start, node.end));
}

export async function withJson5Preset(source: string, preset: string): Promise<string | undefined> {
  // Load the Rust-backed Vite parser only for .json5 files. Module mode rejects
  // legacy JavaScript escapes. The newline also terminates a final line comment.
  const { parseSync } = await import('vite');
  const wrapped = '(' + source + '\n)';
  const parsed = parseSync('renovate.mjs', wrapped, { showSemanticErrors: true });
  if (parsed.errors.length) {
    return undefined;
  }
  const { body } = parsed.program;
  const [statement] = body;
  if (
    body.length !== 1 ||
    statement.type !== 'ExpressionStatement' ||
    statement.expression.type !== 'ParenthesizedExpression'
  ) {
    return undefined;
  }
  const object = statement.expression.expression;
  if (object.type !== 'ObjectExpression' || !isJson5Value(object, wrapped)) {
    return undefined;
  }
  const array = objectProperty(object, 'extends')?.value;
  if (
    array !== undefined &&
    (array.type !== 'ArrayExpression' ||
      !array.elements.every((item) => isString(item) && item.value !== preset))
  ) {
    return undefined;
  }
  return appendRenovateJson5Preset(wrapped, object, array, parsed.comments, preset).slice(1, -2);
}

function quoteAt(source: string, node?: t.Span | null): string {
  const char = node && source[node.start];
  return char === "'" || char === '"' ? char : '';
}

const LINE_INDENT = /(\r\n|[\r\n\u2028\u2029])([^\S\r\n\u2028\u2029]*)$/;

/** All spans refer to source, including any parser-supplied expression wrapper. */
function appendRenovateJson5Preset(
  source: string,
  object: t.ObjectExpression,
  array: t.ArrayExpression | undefined,
  comments: readonly t.Span[],
  preset: string,
): string {
  const container = array ?? object;
  const items = array ? array.elements : object.properties;
  const last = items.at(-1);
  const previous = items.at(-2);
  const close = container.end - 1;
  const afterLast = last?.end ?? container.start + 1;

  // After the final item only comments, whitespace and a trailing comma are
  // possible. A comma *inside* a comment must not become an array separator.
  let end = afterLast;
  let trailingComma = false;
  for (const comment of comments) {
    if (comment.start >= afterLast && comment.end <= close) {
      trailingComma ||= source.slice(end, comment.start).includes(',');
      end = comment.end;
    }
  }
  const tail = source.slice(end, close);
  trailingComma ||= tail.includes(',');
  const insert = end + tail.trimEnd().length;
  const padding = source.slice(insert, close);

  const property = array
    ? object.properties.find((prop) => prop.type === 'Property' && prop.value === array)
    : object.properties.at(-1);
  const key = property?.type === 'Property' ? property.key : undefined;
  const value = property?.type === 'Property' ? property.value : undefined;
  const keyQuote = quoteAt(source, key);
  const quote =
    quoteAt(source, array?.elements.at(-1)) || quoteAt(source, value) || keyQuote || "'";
  const string = JSON.stringify(preset);
  let entry = quote === '"' ? string : "'" + string.slice(1, -1).replaceAll("'", "\\'") + "'";
  if (!array) {
    const colon = key && value && /^[ \t]*:[ \t]*$/.exec(source.slice(key.end, value.start));
    entry = `${keyQuote}extends${keyQuote}${colon?.[0] ?? ': '}[${entry}]`;
  }

  const lastLine = last && LINE_INDENT.exec(source.slice(0, last.start));
  const closingLine = LINE_INDENT.exec(padding);
  const newline = closingLine?.[1] ?? lastLine?.[1];
  let spacing: string;
  if (newline) {
    let indent = lastLine?.[2];
    if (indent == null) {
      const { insertSpaces, tabSize } = detectFormattingOptions(source);
      indent = (closingLine?.[2] ?? '') + (insertSpaces ? ' '.repeat(tabSize ?? 2) : '\t');
    }
    spacing = newline + indent;
  } else {
    const between = previous && last && source.slice(previous.end, last.start);
    spacing = (between && /,([ \t]*)$/.exec(between)?.[1]) ?? (end > afterLast ? ' ' : padding);
  }

  // Place a necessary comma before any comment attached to the old last item.
  return (
    source.slice(0, afterLast) +
    (last && !trailingComma ? ',' : '') +
    source.slice(afterLast, insert) +
    spacing +
    entry +
    (trailingComma ? ',' : '') +
    source.slice(insert)
  );
}
