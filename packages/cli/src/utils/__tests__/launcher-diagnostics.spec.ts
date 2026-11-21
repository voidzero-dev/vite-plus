import { describe, expect, it } from 'vitest';

import { nodeOptionsStartProfiler } from '../launcher-diagnostics.ts';

describe('nodeOptionsStartProfiler', () => {
  it('finds a profiler option among other options', () => {
    expect(nodeOptionsStartProfiler('--cpu-prof')).toBe(true);
    expect(nodeOptionsStartProfiler('--require ./hook.cjs  --heap-prof')).toBe(true);
  });

  it('accepts underscores in option names', () => {
    expect(nodeOptionsStartProfiler('--cpu_prof')).toBe(true);
  });

  it('ignores options that do not start a profiler', () => {
    expect(nodeOptionsStartProfiler(undefined)).toBe(false);
    expect(nodeOptionsStartProfiler('')).toBe(false);
    expect(nodeOptionsStartProfiler('--max-old-space-size=4096 --enable-source-maps')).toBe(false);
    expect(nodeOptionsStartProfiler('--cpu-prof-dir=/tmp/profiles')).toBe(false);
  });
});
