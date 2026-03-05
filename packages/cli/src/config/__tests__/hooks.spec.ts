import { describe, expect, it } from 'vitest';

import { hookScript } from '../hooks.js';

function countDirnameCalls(script: string): number {
  // Count nested dirname calls in the `d=...` line
  const match = script.match(/^d=(.+)$/m);
  if (!match) {
    return 0;
  }
  return (match[1].match(/dirname/g) ?? []).length;
}

describe('hookScript', () => {
  it('should compute correct depth for simple dir', () => {
    // ".vite-hooks" → 1 segment → depth 3
    const script = hookScript('.vite-hooks');
    expect(countDirnameCalls(script)).toBe(3);
  });

  it('should compute correct depth for nested dir', () => {
    // ".config/husky" → 2 segments → depth 4
    const script = hookScript('.config/husky');
    expect(countDirnameCalls(script)).toBe(4);
  });

  it('should handle ./ prefix correctly (bug case)', () => {
    // "./.config/husky" should produce same depth as ".config/husky"
    // Before fix: filter(Boolean) kept "." → 3 segments → depth 5 (wrong)
    // After fix: filter out "." → 2 segments → depth 4 (correct)
    const withDot = hookScript('./.config/husky');
    const withoutDot = hookScript('.config/husky');
    expect(countDirnameCalls(withDot)).toBe(countDirnameCalls(withoutDot));
    expect(countDirnameCalls(withDot)).toBe(4);
  });

  it('should handle ./ prefix for simple dir', () => {
    // "./custom-hooks" should produce same depth as "custom-hooks"
    const withDot = hookScript('./custom-hooks');
    const withoutDot = hookScript('custom-hooks');
    expect(countDirnameCalls(withDot)).toBe(countDirnameCalls(withoutDot));
    expect(countDirnameCalls(withDot)).toBe(3);
  });
});
