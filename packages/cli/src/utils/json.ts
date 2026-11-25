import fs from 'node:fs';

import detectIndent from 'detect-indent';
import { detectNewline } from 'detect-newline';
import type { FormattingOptions, JSONPath, ModificationOptions } from 'jsonc-parser';
import { applyEdits, modify, parse as parseJsonc } from 'jsonc-parser';

import type { JsonObject, JsonValue } from './json-value.ts';
import { isJsonObject, isObjectValue, parseJsonValue } from './json-value.ts';

export { isJsonObject, parseJsonValue };
export type { JsonObject, JsonValue };

/**
 * Derive `jsonc-parser` formatting options from existing file text so inserted
 * fragments match the file's indentation and newline style.
 */
export function detectFormattingOptions(text: string): FormattingOptions {
  const detected = detectIndent(text);
  return {
    insertSpaces: detected.type !== 'tab',
    tabSize: detected.amount || 2,
    eol: detectNewline(text) ?? '\n',
  };
}

export function applyJsoncEdit(
  text: string,
  path: JSONPath,
  value: unknown,
  options: ModificationOptions,
): string {
  return applyEdits(text, modify(text, path, value, options));
}

/** Append missing entries in place so comments inside the array survive. */
export function appendMissingJsoncArrayItems(
  text: string,
  path: JSONPath,
  existing: unknown,
  incoming: ReadonlyArray<unknown>,
  formattingOptions?: FormattingOptions,
): string {
  if (existing === undefined) {
    return applyJsoncEdit(text, path, incoming, { formattingOptions });
  }
  if (!Array.isArray(existing)) {
    return text;
  }

  const existingItems = new Set<unknown>(existing);
  for (const item of incoming) {
    if (!existingItems.has(item)) {
      text = applyJsoncEdit(text, [...path, -1], item, {
        formattingOptions,
        isArrayInsertion: true,
      });
    }
  }
  return text;
}

export function readJsonValue(file: string, allowComments?: boolean): JsonValue {
  const content = fs.readFileSync(file, 'utf8');
  if (!allowComments) {
    return parseJsonValue(content);
  }
  const value: unknown = parseJsonc(content);
  if (value === undefined) {
    throw new SyntaxError(`Invalid JSON in ${file}`);
  }
  // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- jsonc-parser returns a JSON value or undefined.
  return value as JsonValue;
}

export function readJsonFile(file: string, allowComments?: boolean): JsonObject {
  const value = readJsonValue(file, allowComments);
  if (!isObjectValue(value)) {
    throw new TypeError(`Expected a JSON object in ${file}`);
  }
  return value;
}

export function writeJsonFile(file: string, data: unknown) {
  let newline = '\n';
  let indent = '  ';
  if (fs.existsSync(file)) {
    const content = fs.readFileSync(file, 'utf8');
    // keep the original newline and indent
    indent = detectIndent(content).indent;
    newline = detectNewline(content) ?? '';
  }
  fs.writeFileSync(file, JSON.stringify(data, null, indent) + newline, 'utf8');
}

export function editJsonFile<T extends object = JsonObject>(
  file: string,
  callback: (content: T) => T | undefined,
) {
  // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- Callers supply the schema of the file they own and edit.
  const json = readJsonFile(file) as T;
  const newJson = callback(json);
  if (newJson) {
    writeJsonFile(file, newJson);
  }
}

export function isJsonFile(file: string): boolean {
  try {
    readJsonValue(file);
    return true;
  } catch {
    return false;
  }
}
