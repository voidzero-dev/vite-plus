import fs from 'node:fs';
import path from 'node:path';

import detectIndent from 'detect-indent';
import { detectNewline } from 'detect-newline';

export function readJsonFile<T = Record<string, unknown>>(file: string): T {
  const content = fs.readFileSync(file, 'utf-8');
  return JSON.parse(content) as T;
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

/**
 * Check if tsconfig.json has compilerOptions.baseUrl set.
 * oxlint's TypeScript checker (tsgolint) does not support baseUrl,
 * so typeAware/typeCheck must be disabled when it is present.
 */
export function hasBaseUrlInTsconfig(projectPath: string): boolean {
  try {
    const tsconfig = readJsonFile<{ compilerOptions?: { baseUrl?: string } }>(
      path.join(projectPath, 'tsconfig.json'),
    );
    return tsconfig?.compilerOptions?.baseUrl !== undefined;
  } catch {
    return false;
  }
}
