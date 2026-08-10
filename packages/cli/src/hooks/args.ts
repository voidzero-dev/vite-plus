const KNOWN_HOOKS_ARG_KEYS = new Set(['_', 'help', 'h', 'hooks-dir']);

/**
 * Reject leftover positionals and unknown flags before enable/disable mutate state.
 */
export function unexpectedHooksArgsError(args: {
  _: Array<string | number>;
  [key: string]: unknown;
}): string | null {
  const extra = args._.map(String).filter((value) => value !== '');
  if (extra.length > 0) {
    return `Unexpected argument "${extra[0]}". Use --hooks-dir <path> to set a custom hooks directory.`;
  }
  for (const key of Object.keys(args)) {
    if (!KNOWN_HOOKS_ARG_KEYS.has(key)) {
      return `Unknown option "--${key}".`;
    }
  }
  return null;
}
