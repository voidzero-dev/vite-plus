/** `NODE_OPTIONS` entries that start a profiler, which writes its profile when the process exits. */
const PROFILER_OPTIONS = new Set(['--cpu-prof', '--heap-prof']);

/**
 * Returns whether `NODE_OPTIONS` starts a profiler. Node.js accepts `_` in
 * place of `-` in option names.
 */
export function nodeOptionsStartProfiler(nodeOptions: string | undefined): boolean {
  return (nodeOptions ?? '')
    .split(/\s+/)
    .some((option) => PROFILER_OPTIONS.has(option.replaceAll('_', '-')));
}

/**
 * Returns whether Node.js ties a debugger session or exit-time output to this
 * process. An exec replaces the process before Node.js writes that output, and
 * the new process opens a second debugger session.
 */
export async function launcherHasDiagnostics(): Promise<boolean> {
  // Command-line flags apply only to this process, so keep any of them.
  if (process.execArgv.length > 0 || process.env.NODE_V8_COVERAGE) {
    return true;
  }
  // `NODE_OPTIONS` and `inspector.open()` start the inspector without `execArgv`.
  if (process.features.inspector) {
    const inspector = await import('node:inspector');
    if (inspector.url() !== undefined) {
      return true;
    }
  }
  return nodeOptionsStartProfiler(process.env.NODE_OPTIONS);
}
