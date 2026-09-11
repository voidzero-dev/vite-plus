import fs from 'node:fs';

import detectIndent from 'detect-indent';
import { detectNewline } from 'detect-newline';
import { type FormattingOptions, parse as parseJsonc } from 'jsonc-parser';

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
