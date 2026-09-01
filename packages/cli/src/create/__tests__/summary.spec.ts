import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { getNextCommand, showCreateSummary } from '../summary.js';

// `terminal.ts` reaches the NAPI binding for the Vite+ header, so stub it
// outright rather than importing the real module. Styling is not asserted.
const terminal = vi.hoisted(() => ({
  log: vi.fn(),
  accent: (text: string) => text,
  formatDuration: (durationMs: number) => `${durationMs}ms`,
}));

vi.mock('../../utils/terminal.ts', () => terminal);

const installed = { durationMs: 1200, exitCode: 0, status: 'installed' } as const;
const formatted = { durationMs: 30, exitCode: 0, status: 'formatted' } as const;
const failed = { durationMs: 30, exitCode: 1, status: 'failed' } as const;

const baseOptions = {
  description: 'Vite application',
  packageManager: 'pnpm',
  packageManagerVersion: '11.21.0',
  projectDir: 'create-failure-demo',
};

// `showCreateSummary` writes through `log`, so assert on the joined output
// rather than the exact styling of any one line.
function output() {
  return terminal.log.mock.calls.map((call) => String(call[0])).join('\n');
}

describe('getNextCommand', () => {
  it('prefixes a cd when the project is in a subdirectory', () => {
    expect(getNextCommand('demo', 'vp run')).toBe('cd demo && vp run');
  });

  it('omits the cd for the current directory', () => {
    expect(getNextCommand('.', 'vp run')).toBe('vp run');
    expect(getNextCommand('', 'vp run')).toBe('vp run');
  });
});

describe('showCreateSummary', () => {
  let previousExitCode: typeof process.exitCode;

  beforeEach(() => {
    terminal.log.mockClear();
    previousExitCode = process.exitCode;
    process.exitCode = undefined;
  });

  afterEach(() => {
    process.exitCode = previousExitCode;
  });

  it('reports a ready project and leaves the exit code alone on success', () => {
    showCreateSummary({ ...baseOptions, installSummary: installed, fmtSummary: formatted });

    const text = output();
    expect(text).toContain('Scaffolded');
    expect(text).toContain('Dependencies installed');
    expect(text).toContain('Next: ');
    expect(text).toContain('cd create-failure-demo && vp run');
    expect(text).not.toContain('were not installed');
    expect(process.exitCode).toBeUndefined();
  });

  // Regression test for #2453: a failed install used to be summarized as a
  // finished project — "Scaffolded", "Next: vp run", exit code 0 — even
  // though node_modules was missing.
  it('does not report a failed install as a ready project', () => {
    showCreateSummary({ ...baseOptions, installSummary: failed, fmtSummary: failed });

    const text = output();
    expect(text).toContain('Dependencies were not installed');
    expect(text).toContain('Code was not formatted');
    expect(text).toContain('cd create-failure-demo && vp install');
    expect(text).not.toContain('vp run');
    expect(process.exitCode).toBe(1);
  });

  // Regression test for the PTY snapshot failures this PR first introduced:
  // `create_framework_shim_vue` and `new_create_vite` scaffold templates whose
  // plugin imports are not resolvable when `vp fmt` runs, so the format step
  // fails on a project that is otherwise complete. Naming it is right; exiting
  // non-zero for it is not.
  it('still suggests running the project and succeeds when only formatting failed', () => {
    showCreateSummary({ ...baseOptions, installSummary: installed, fmtSummary: failed });

    const text = output();
    expect(text).toContain('Code was not formatted');
    expect(text).toContain('cd create-failure-demo && vp run');
    expect(text).not.toContain('Dependencies were not installed');
    expect(process.exitCode).toBeUndefined();
  });

  it('never lowers an exit code another step already raised', () => {
    process.exitCode = 1;
    showCreateSummary({ ...baseOptions, installSummary: installed, fmtSummary: formatted });

    expect(process.exitCode).toBe(1);
  });
});
