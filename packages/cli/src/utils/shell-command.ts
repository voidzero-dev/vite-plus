import { parseLiteralCommands, type LiteralCommandToken } from '../../binding/index.js';

export type CommandToken = LiteralCommandToken;

/** Locate the executable after a supported package-manager wrapper. */
export function getCommandStartIndex(argv: readonly string[]): number {
  if (!['pnpm', 'npm', 'yarn', 'bun', 'npx', 'bunx'].includes(argv[0])) {
    return 0;
  }
  let start = 1;
  if (argv[start] === 'exec' || argv[start] === 'x') {
    start++;
  }
  if (argv[start] === '--') {
    start++;
  }
  return start;
}

/** Parse only a single literal argv command. */
export function literalArgv(command: string): CommandToken[] | undefined {
  return parseLiteralCommands(command, false)?.[0];
}

/** Read literal command lists with offsets in the original JavaScript string. */
export function literalCommands(command: string): CommandToken[][] | undefined {
  return parseLiteralCommands(command, true) ?? undefined;
}
