import fs from 'node:fs';
import path from 'node:path';

import { applyEdits, modify, parse as parseJsonc } from 'jsonc-parser';

/**
 * Check if tsconfig.json has compilerOptions.baseUrl set.
 * oxlint's TypeScript checker (tsgolint) does not support baseUrl,
 * so typeAware/typeCheck must be disabled when it is present.
 */
export function hasBaseUrlInTsconfig(projectPath: string): boolean {
  try {
    const tsconfig = JSON.parse(
      fs.readFileSync(path.join(projectPath, 'tsconfig.json'), 'utf-8'),
    ) as { compilerOptions?: { baseUrl?: string } };
    return tsconfig?.compilerOptions?.baseUrl !== undefined;
  } catch {
    return false;
  }
}

const TSCONFIG_FILE_RE = /^tsconfig(\.[\w-]+)?\.json$/i;

export function findTsconfigFiles(projectPath: string): string[] {
  try {
    const entries = fs.readdirSync(projectPath);
    return entries
      .filter((name) => TSCONFIG_FILE_RE.test(name))
      .map((name) => path.join(projectPath, name));
  } catch {
    return [];
  }
}

// jsonc-parser is in dependencies (not devDependencies) so it's available at
// runtime for tsc-compiled code (init-config.ts imports this file).
// TODO: move back to devDependencies once the bundle refactoring lands
// https://github.com/voidzero-dev/vite-plus/issues/744
export function removeDeprecatedTsconfigFalseOption(filePath: string, optionName: string): boolean {
  let text: string;
  try {
    text = fs.readFileSync(filePath, 'utf-8');
  } catch {
    return false;
  }

  const parsed = parseJsonc(text) as {
    compilerOptions?: Record<string, unknown>;
  } | null;
  if (parsed?.compilerOptions?.[optionName] !== false) {
    return false;
  }

  const edits = modify(text, ['compilerOptions', optionName], undefined, {});
  if (edits.length === 0) {
    return false;
  }

  const newText = applyEdits(text, edits);
  fs.writeFileSync(filePath, newText);
  return true;
}

export function rewriteTypesInTsconfig(filePath: string): boolean {
  let text: string;
  try {
    text = fs.readFileSync(filePath, 'utf-8');
  } catch {
    return false;
  }

  const parsed = parseJsonc(text) as {
    compilerOptions?: { types?: unknown[] };
  } | null;

  const types = parsed?.compilerOptions?.types;
  if (!Array.isArray(types)) {
    return false;
  }

  const REPLACEMENTS: Record<string, string> = {
    'tsdown/client': 'vite-plus/pack/client',
    'vite/client': 'vite-plus/client',
  };

  const toReplace = types
    .map((t, i) =>
      typeof t === 'string' && t in REPLACEMENTS ? { i, newVal: REPLACEMENTS[t] } : null,
    )
    .filter((x): x is { i: number; newVal: string } => x !== null);

  if (toReplace.length === 0) {
    return false;
  }

  // Apply edits right-to-left so earlier element offsets stay valid after each replacement.
  let currentText = text;
  for (let j = toReplace.length - 1; j >= 0; j--) {
    const { i, newVal } = toReplace[j];
    const edits = modify(currentText, ['compilerOptions', 'types', i], newVal, {});
    if (edits.length > 0) {
      currentText = applyEdits(currentText, edits);
    }
  }

  fs.writeFileSync(filePath, currentText);
  return true;
}
