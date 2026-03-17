import fs from 'node:fs';

import detectIndent from 'detect-indent';
import { detectNewline } from 'detect-newline';
import { parse as parseJsonc } from 'jsonc-parser';

export function readJsonFile<T = Record<string, unknown>>(
  file: string,
  allowComments?: boolean,
): T {
  const content = fs.readFileSync(file, 'utf-8');
  const parseFunction = allowComments ? parseJsonc : JSON.parse;
  return parseFunction(content) as T;
}

export function writeJsonFile<T = Record<string, unknown>>(file: string, data: T) {
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

export function editJsonFile<T = Record<string, unknown>>(
  file: string,
  callback: (content: T) => T | undefined,
) {
  const json = readJsonFile<T>(file);
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
