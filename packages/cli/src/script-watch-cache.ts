import { NODE_OPTIONS_WITH_VALUE, WATCH_OPTIONS_WITH_VALUE } from './vpx-script.ts';

/** Consume the watcher's cache flag, preserving options' values and script arguments. */
export function watchCacheArgs(argv: readonly string[]): { args: string[]; disabled: boolean } {
  const args: string[] = [];
  let disabled = false;
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === '--' || arg === '-' || !arg.startsWith('-')) {
      args.push(...argv.slice(i));
      break;
    }
    const name = arg.split('=', 1)[0].replaceAll('_', '-');
    if (name === '--no-cache') {
      disabled = true;
      continue;
    }
    args.push(arg);
    // Match vpx's treatment of a bare config flag across supported Node versions:
    // only a .json token is its value; a script extension remains the entry.
    const takesValue =
      name === '--experimental-config-file'
        ? argv[i + 1]?.endsWith('.json') && !argv[i + 1].startsWith('-')
        : NODE_OPTIONS_WITH_VALUE.has(name) || WATCH_OPTIONS_WITH_VALUE.has(name);
    if (!arg.includes('=') && takesValue && i + 1 < argv.length) {
      args.push(argv[++i]);
    }
  }
  return { args, disabled };
}
