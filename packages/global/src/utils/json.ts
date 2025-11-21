import fs from 'node:fs';

import detectIndent from 'detect-indent';
import { detectNewline } from 'detect-newline';

export function readJsonFile<T = Record<string, any>>(file: string): T {
  const content = fs.readFileSync(file, 'utf-8');
  return JSON.parse(content) as T;
}

export function writeJsonFile<T = Record<string, any>>(file: string, data: T) {
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

export function editJsonFile<T = Record<string, any>>(
  file: string,
  callback: (content: T) => T | undefined,
) {
  const json = readJsonFile<T>(file);
  const newJson = callback(json);
  if (newJson) {
    writeJsonFile(file, newJson);
  }
}
