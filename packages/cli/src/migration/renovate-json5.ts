import type * as t from '@oxc-project/types';

import { detectFormattingOptions } from '../utils/json.ts';
import { isString, objectProperty, staticObject } from './ast-utils.ts';

// Oxc parses JavaScript. JSON5 has a smaller set of numeric literals and
// uses four hex digits for Unicode escapes, including in unquoted keys.
const JSON5_NUMBER =
  /^[+-]?(?:Infinity|NaN|0[xX][\da-fA-F]+|(?:0|[1-9]\d*)(?:\.\d*)?(?:[eE][+-]?\d+)?|\.\d+(?:[eE][+-]?\d+)?)$/;
const CODE_POINT_ESCAPE = /(?:^|[^\\])(?:\\\\)*\\u\{/;
const LINE_INDENT = /(\r\n|[\r\n\u2028\u2029])([^\S\r\n\u2028\u2029]*)$/;

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
  const extendsProperty = objectProperty(object, 'extends');
  const array = extendsProperty?.value;
  if (
    array !== undefined &&
    (array.type !== 'ArrayExpression' ||
      !array.elements.every((item) => isString(item) && item.value !== preset))
  ) {
    return undefined;
  }
  const styleProperty = extendsProperty ?? object.properties.at(-1);
  const entry = formatPresetEntry(wrapped, preset, array, styleProperty);
  return appendEntry(wrapped, array ?? object, parsed.comments, entry).slice(1, -2);
}

function quoteAt(source: string, node?: t.Span | null): string {
  const char = node && source[node.start];
  return char === "'" || char === '"' ? char : '';
}

/** Match the property's quotes and colon spacing when creating an `extends` list. */
function formatPresetEntry(
  source: string,
  preset: string,
  array: t.ArrayExpression | undefined,
  styleProperty: t.ObjectPropertyKind | undefined,
): string {
  const property = styleProperty?.type === 'Property' ? styleProperty : undefined;
  const keyQuote = quoteAt(source, property?.key);
  const quote = quoteAt(source, array?.elements.at(-1) ?? property?.value) || keyQuote || "'";
  const jsonString = JSON.stringify(preset);
  const item =
    quote === '"' ? jsonString : "'" + jsonString.slice(1, -1).replaceAll("'", "\\'") + "'";
  if (array) {
    return item;
  }
  const colon =
    property && /^[ \t]*:[ \t]*$/.exec(source.slice(property.key.end, property.value.start));
  return `${keyQuote}extends${keyQuote}${colon?.[0] ?? ': '}[${item}]`;
}

function appendEntry(
  source: string,
  container: t.ArrayExpression | t.ObjectExpression,
  comments: readonly t.Span[],
  entry: string,
): string {
  const items = container.type === 'ArrayExpression' ? container.elements : container.properties;
  const last = items.at(-1);
  const previous = items.at(-2);
  const close = container.end - 1;
  const afterLast = last?.end ?? container.start + 1;

  // Search between comments so a comma in a comment cannot act as a separator.
  let afterComments = afterLast;
  let trailingComma = false;
  for (const comment of comments) {
    if (comment.start >= afterLast && comment.end <= close) {
      trailingComma ||= source.slice(afterComments, comment.start).includes(',');
      afterComments = comment.end;
    }
  }
  const tail = source.slice(afterComments, close);
  trailingComma ||= tail.includes(',');
  const insert = afterComments + tail.trimEnd().length;
  const padding = source.slice(insert, close);

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
    const separatorSpace = between && /,([ \t]*)$/.exec(between)?.[1];
    spacing = separatorSpace ?? (afterComments > afterLast ? ' ' : padding);
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
