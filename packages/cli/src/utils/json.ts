import fs from 'node:fs';

import detectIndent from 'detect-indent';
import { detectNewline } from 'detect-newline';
import {
  applyEdits,
  type FormattingOptions,
  type JSONPath,
  modify,
  type ModificationOptions,
  parse as parseJsonc,
} from 'jsonc-parser';

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
  incoming: readonly unknown[],
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

export function readJsonFile(file: string, allowComments?: boolean): Record<string, unknown> {
  const content = fs.readFileSync(file, 'utf-8');
  const parseFunction = allowComments ? parseJsonc : JSON.parse;
  return parseFunction(content);
}

export function writeJsonFile(file: string, data: Record<string, unknown>) {
  let newline = '\n';
  let indent = '  ';
  if (fs.existsSync(file)) {
    const content = fs.readFileSync(file, 'utf-8');
    // keep the original newline and indent
    indent = detectIndent(content).indent;
    newline = detectNewline(content) ?? '';
  }
  fs.writeFileSync(file, JSON.stringify(data, null, indent) + newline, 'utf-8');
}

export function editJsonFile<T extends Record<string, unknown> = Record<string, unknown>>(
  file: string,
  callback: (content: T) => T | undefined,
) {
  const json = readJsonFile(file) as T;
  const newJson = callback(json);
  if (newJson) {
    writeJsonFile(file, newJson);
  }
}

export function isJsonFile(file: string): boolean {
  try {
    readJsonFile(file);
    return true;
  } catch {
    return false;
  }
}
