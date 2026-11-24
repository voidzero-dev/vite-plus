import fs from 'node:fs';
import path from 'node:path';
import { styleText } from 'node:util';

import { confirm, isCancel, log } from '@voidzero-dev/vite-plus-prompts';
import { applyEdits, modify, parse as parseJsonc } from 'jsonc-parser';

import vitestV5EntryPoints from '../vitest-v5-entry-points.json' with { type: 'json' };
import { getVpCommand, runCommandSilently } from './command.ts';
import { BASEURL_TSCONFIG_FIX_PACKAGE, createBaseUrlTsconfigFixArgs } from './constants.ts';
import type { JsonObject } from './json-value.ts';
import { isJsonObject, isObjectValue } from './json-value.ts';
import { hasOwnKey } from './object.ts';
import { cancelAndExit } from './prompts.ts';

function parseCompilerOptions(text: string): JsonObject | undefined {
  const parsed: unknown = parseJsonc(text);
  return isJsonObject(parsed) && isObjectValue(parsed.compilerOptions)
    ? parsed.compilerOptions
    : undefined;
}

export type BaseUrlFixStatus = 'not-needed' | 'fixed' | 'declined' | 'failed';

/**
 * Check if tsconfig.json has compilerOptions.baseUrl set.
 * Oxlint's TypeScript 7 backend does not support baseUrl,
 * so typeAware/typeCheck must be disabled when it is present.
 */
export function hasBaseUrlInTsconfigFile(filePath: string): boolean {
  try {
    const options = parseCompilerOptions(fs.readFileSync(filePath, 'utf8'));
    return options?.baseUrl !== undefined && options.baseUrl !== null;
  } catch {
    return false;
  }
}

const TSCONFIG_FILE_RE = /^tsconfig(\.[\w-]+)?\.json$/i;

export function findTsconfigFiles(projectPath: string): Array<string> {
  try {
    const entries = fs.readdirSync(projectPath);
    return entries
      .filter((name) => TSCONFIG_FILE_RE.test(name))
      .map((name) => path.join(projectPath, name));
  } catch {
    return [];
  }
}

export function hasBaseUrlInTsconfig(projectPath: string): boolean {
  return findTsconfigFiles(projectPath).some((filePath) => hasBaseUrlInTsconfigFile(filePath));
}

export function findTsconfigFilesWithBaseUrl(projectPath: string): Array<string> {
  return findTsconfigFiles(projectPath).filter((filePath) => hasBaseUrlInTsconfigFile(filePath));
}

export async function confirmBaseUrlFix(interactive: boolean): Promise<boolean> {
  if (!interactive) {
    return true;
  }

  const command = [
    BASEURL_TSCONFIG_FIX_PACKAGE,
    ...createBaseUrlTsconfigFixArgs('<tsconfig path>'),
  ].join(' ');
  const confirmed = await confirm({
    message:
      'Your tsconfig contains `baseUrl`, which prevents enabling type-aware linting.\n  ' +
      styleText(
        'gray',
        '`baseUrl` is deprecated in TypeScript 6.0 and removed in TypeScript 7.0.',
      ) +
      `\n  Download and run the external \`${BASEURL_TSCONFIG_FIX_PACKAGE}\` fixer now?\n  ` +
      styleText('gray', `Equivalent command: \`vp dlx ${command}\``),
    initialValue: true,
  });
  if (isCancel(confirmed)) {
    cancelAndExit();
  }
  return confirmed;
}

export async function fixBaseUrlInTsconfig(
  projectPath: string,
  options?: {
    interactive?: boolean;
    confirmed?: boolean;
    silent?: boolean;
    onStatus?: (status: BaseUrlFixStatus, projectPath: string) => void;
  },
): Promise<BaseUrlFixStatus> {
  const files = findTsconfigFilesWithBaseUrl(projectPath);
  if (files.length === 0) {
    return 'not-needed';
  }

  const confirmed = options?.confirmed ?? (await confirmBaseUrlFix(options?.interactive ?? false));
  if (!confirmed) {
    options?.onStatus?.('declined', projectPath);
    return 'declined';
  }

  try {
    for (const filePath of files) {
      const target = path.relative(projectPath, filePath) || filePath;
      const fixArgs = createBaseUrlTsconfigFixArgs(target);
      if (!options?.silent) {
        log.info(`Running vp dlx ${BASEURL_TSCONFIG_FIX_PACKAGE} ${fixArgs.join(' ')}`);
      }
      const result = await runCommandSilently({
        ...getVpCommand(['dlx', BASEURL_TSCONFIG_FIX_PACKAGE, ...fixArgs]),
        cwd: projectPath,
        envs: process.env,
      });

      if (result.exitCode !== 0) {
        if (!options?.silent) {
          const output = `${result.stdout.toString()}${result.stderr.toString()}`.trim();
          if (output) {
            log.warn(output);
          }
        }
        options?.onStatus?.('failed', projectPath);
        return 'failed';
      }
    }

    if (hasBaseUrlInTsconfig(projectPath)) {
      if (!options?.silent) {
        log.warn('tsconfig still contains baseUrl after running the fixer.');
      }
      options?.onStatus?.('failed', projectPath);
      return 'failed';
    }
  } catch (error) {
    if (!options?.silent && error instanceof Error) {
      log.warn(error.message);
    }
    options?.onStatus?.('failed', projectPath);
    return 'failed';
  }

  options?.onStatus?.('fixed', projectPath);
  return 'fixed';
}

export function removeDeprecatedTsconfigFalseOption(filePath: string, optionName: string): boolean {
  let text: string;
  try {
    text = fs.readFileSync(filePath, 'utf8');
  } catch {
    return false;
  }

  const options = parseCompilerOptions(text);
  if (options?.[optionName] !== false) {
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

// `vite/client` is intentionally NOT rewritten to `vite-plus/client`: per issue
// #2004 the migration preserves `vite`/`vite/*` references outside config entry
// files (a tsconfig is not a `vite.config.*`), and `vite/client` still resolves
// through the `@voidzero-dev/vite-plus-core` alias. Only `tsdown/client` is
// rewritten here (tsdown is not part of the vite scoping).
const TSCONFIG_TYPE_REPLACEMENTS = {
  'tsdown/client': 'vite-plus/pack/client',
  // Removed Vite+ exports must also migrate in type-only entry lists.
  ...Object.fromEntries(
    Object.entries(vitestV5EntryPoints).filter(([source]) => source.startsWith('vite-plus/')),
  ),
};

export function hasTypesToRewriteInTsconfig(filePath: string): boolean {
  let text: string;
  try {
    text = fs.readFileSync(filePath, 'utf8');
  } catch {
    return false;
  }

  const options = parseCompilerOptions(text);
  const types = options?.types;
  return (
    Array.isArray(types) &&
    types.some((t) => typeof t === 'string' && hasOwnKey(TSCONFIG_TYPE_REPLACEMENTS, t))
  );
}

export function hasVitestTypesInTsconfig(filePath: string): boolean {
  let text: string;
  try {
    text = fs.readFileSync(filePath, 'utf8');
  } catch {
    return false;
  }

  const options = parseCompilerOptions(text);
  const types = options?.types;
  return (
    Array.isArray(types) &&
    types.some((type) =>
      typeof type === 'string' ? type === 'vitest' || type.startsWith('vitest/') : false,
    )
  );
}

export function rewriteTypesInTsconfig(filePath: string): boolean {
  let text: string;
  try {
    text = fs.readFileSync(filePath, 'utf8');
  } catch {
    return false;
  }

  const options = parseCompilerOptions(text);
  const types = options?.types;
  if (!Array.isArray(types)) {
    return false;
  }

  const toReplace = types
    .map((t, i) =>
      typeof t === 'string' && hasOwnKey(TSCONFIG_TYPE_REPLACEMENTS, t)
        ? { i, newVal: TSCONFIG_TYPE_REPLACEMENTS[t] }
        : null,
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
