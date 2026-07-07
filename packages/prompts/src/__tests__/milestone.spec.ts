import { afterEach, describe, expect, it, vi } from 'vitest';

const originalEnv = process.env.VP_EMIT_MILESTONES;

// The enabled flag is cached at module load (the runner sets the env before
// spawning the CLI), so each test imports a fresh module copy under its env.
async function loadMilestone(value: string | undefined) {
  vi.resetModules();
  if (value === undefined) {
    delete process.env.VP_EMIT_MILESTONES;
  } else {
    process.env.VP_EMIT_MILESTONES = value;
  }
  return import('../milestone.js');
}

afterEach(() => {
  if (originalEnv === undefined) {
    delete process.env.VP_EMIT_MILESTONES;
  } else {
    process.env.VP_EMIT_MILESTONES = originalEnv;
  }
});

describe('milestone', () => {
  it('emits nothing unless VP_EMIT_MILESTONES=1', async () => {
    const unset = await loadMilestone(undefined);
    expect(unset.milestone('vp')).toBe('');
    const disabled = await loadMilestone('0');
    expect(disabled.milestone('vp')).toBe('');
  });

  it('encodes the name as a hex OSC 8 hyperlink with a zero-width anchor', async () => {
    const { milestone } = await loadMilestone('1');
    // "vp" is 0x76 0x70; the sequence must match vite-task's
    // pty_terminal_test_client protocol byte for byte.
    expect(milestone('vp')).toBe('\x1b]8;;https://milestone.invalid/7670\x1b\\​\x1b]8;;\x1b\\');
  });

  it('formats prompt milestones as <kind>:<id>:<state>', async () => {
    const { milestone, promptMilestone } = await loadMilestone('1');
    expect(promptMilestone('select', 'template', '1')).toBe(milestone('select:template:1'));
    expect(promptMilestone('confirm', undefined, 'yes')).toBe(milestone('confirm:confirm:yes'));
  });
});
