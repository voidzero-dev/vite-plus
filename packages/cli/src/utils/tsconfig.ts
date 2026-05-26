import fs from 'node:fs';
import path from 'node:path';
import { styleText } from 'node:util';

import * as prompts from '@voidzero-dev/vite-plus-prompts';
import { applyEdits, modify, parse as parseJsonc } from 'jsonc-parser';

import { runCommandSilently } from './command.ts';
import {
  BASEURL_TSCONFIG_FIX_PACKAGE,
  createBaseUrlTsconfigFixArgs,
} from './constants.ts';
import { cancelAndExit } from './prompts.ts';

export type BaseUrlFixStatus = 'not-needed' | 'fixed' | 'declined' | 'failed';

/**
 * Check if tsconfig.json has compilerOptions.baseUrl set.
 * oxlint's TypeScript checker (tsgolint) does not support baseUrl,
 * so typeAware/typeCheck must be disabled when it is present.
 */
export function hasBaseUrlInTsconfigFile(filePath: string): boolean {
  try {
    const tsconfig = parseJsonc(fs.readFileSync(filePath, 'utf-8')) as {
      compilerOptions?: { baseUrl?: string | null };
    };
    return tsconfig?.compilerOptions?.baseUrl != null;
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

export function hasBaseUrlInTsconfig(projectPath: string): boolean {
  return findTsconfigFiles(projectPath).some((filePath) =>
    hasBaseUrlInTsconfigFile(filePath),
  );
}

export function findTsconfigFilesWithBaseUrl(projectPath: string): string[] {
  return findTsconfigFiles(projectPath).filter((filePath) =>
    hasBaseUrlInTsconfigFile(filePath),
  );
}

export async function confirmBaseUrlFix(interactive: boolean): Promise<boolean> {
  if (!interactive) {
    return true;
  }

  const command = [
    BASEURL_TSCONFIG_FIX_PACKAGE,
    ...createBaseUrlTsconfigFixArgs('<tsconfig path>'),
  ].join(' ');
  const confirmed = await prompts.confirm({
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
  if (prompts.isCancel(confirmed)) {
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

  const confirmed =
    options?.confirmed ?? (await confirmBaseUrlFix(options?.interactive ?? false));
  if (!confirmed) {
    options?.onStatus?.('declined', projectPath);
    return 'declined';
  }

  try {
    for (const filePath of files) {
      const target = path.relative(projectPath, filePath) || filePath;
      const fixArgs = createBaseUrlTsconfigFixArgs(target);
      if (!options?.silent) {
        prompts.log.info(`Running vp dlx ${BASEURL_TSCONFIG_FIX_PACKAGE} ${fixArgs.join(' ')}`);
      }
      const result = await runCommandSilently({
        command: process.env.VP_CLI_BIN ?? 'vp',
        args: ['dlx', BASEURL_TSCONFIG_FIX_PACKAGE, ...fixArgs],
        cwd: projectPath,
        envs: process.env,
      });

      if (result.exitCode !== 0) {
        if (!options?.silent) {
          const output = `${result.stdout.toString()}${result.stderr.toString()}`.trim();
          if (output) {
            prompts.log.warn(output);
          }
        }
        options?.onStatus?.('failed', projectPath);
        return 'failed';
      }
    }

    if (hasBaseUrlInTsconfig(projectPath)) {
      if (!options?.silent) {
        prompts.log.warn('tsconfig still contains baseUrl after running the fixer.');
      }
      options?.onStatus?.('failed', projectPath);
      return 'failed';
    }
  } catch (error) {
    if (!options?.silent && error instanceof Error) {
      prompts.log.warn(error.message);
    }
    options?.onStatus?.('failed', projectPath);
    return 'failed';
  }

  options?.onStatus?.('fixed', projectPath);
  return 'fixed';
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
