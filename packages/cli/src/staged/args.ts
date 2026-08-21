import type { Options } from 'lint-staged';
import mri from 'mri';

export interface StagedArgs {
  help?: boolean;
  concurrent?: unknown;
  cwd?: unknown;
  diff?: unknown;
  'diff-filter'?: unknown;
  'allow-empty'?: boolean;
  debug?: boolean;
  'continue-on-error'?: boolean;
  'fail-on-changes'?: boolean;
  'hide-partially-staged'?: boolean;
  'hide-unstaged'?: boolean;
  quiet?: boolean;
  relative?: boolean;
  revert?: boolean;
  stash?: boolean;
  verbose?: boolean;
}

export interface NormalizedStagedArgs {
  concurrent?: Options['concurrent'];
  cwd?: string;
  diff?: string;
  diffFilter?: string;
}

export function parseStagedArgs(argv: string[]): StagedArgs {
  return mri<StagedArgs>(argv, {
    alias: {
      h: 'help',
      p: 'concurrent',
      d: 'debug',
      q: 'quiet',
      r: 'relative',
      v: 'verbose',
    },
    boolean: [
      'help',
      'allow-empty',
      'debug',
      'continue-on-error',
      'fail-on-changes',
      'hide-partially-staged',
      'hide-unstaged',
      'quiet',
      'relative',
      'revert',
      'stash',
      'verbose',
    ],
    string: ['concurrent', 'cwd', 'diff', 'diff-filter'],
  });
}

export function normalizeStagedArgs(args: StagedArgs): NormalizedStagedArgs {
  return {
    concurrent: normalizeConcurrent(args.concurrent),
    cwd: normalizeStringOption(args.cwd, 'cwd', 'path'),
    diff: normalizeStringOption(args.diff, 'diff', 'string'),
    diffFilter: normalizeStringOption(args['diff-filter'], 'diff-filter', 'string'),
  };
}

function normalizeConcurrent(value: unknown): Options['concurrent'] | undefined {
  if (value == null) {
    return undefined;
  }
  if (value === true || value === false) {
    return value;
  }
  if (value === '') {
    return true;
  }
  if (value === 'true') {
    return true;
  }
  if (value === 'false') {
    return false;
  }

  const number = Number(value);
  if (Number.isFinite(number) && number > 0) {
    return number;
  }

  throw new Error('Option "--concurrent" must be true, false, or a number greater than 0.');
}

function normalizeStringOption(
  value: unknown,
  option: string,
  valueName: string,
): string | undefined {
  if (value == null) {
    return undefined;
  }
  if (value === false) {
    throw new Error(`Option "--no-${option}" is not supported. Use "--${option} <${valueName}>".`);
  }
  if (typeof value !== 'string' || value === '') {
    throw new Error(`Option "--${option}" requires a value.`);
  }
  return value;
}
