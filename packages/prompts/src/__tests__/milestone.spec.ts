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

  it('encodes the name as a window-title milestone marker', async () => {
    const { milestone } = await loadMilestone('1');
    // The marker must match vite-task's pty_terminal_test_client title
    // protocol: OSC 2 ; pty-terminal-test:<32-hex-id>:<base64url(name)> ST.
    const marker = milestone('vp');
    const match = /^\x1b\]2;pty-terminal-test:([0-9a-f]{32}):([A-Za-z0-9_-]+)\x1b\\$/.exec(marker);
    expect(match).not.toBeNull();
    expect(Buffer.from(match![2], 'base64url').toString('utf8')).toBe('vp');
  });

  it('uses a fresh id per emission so repeated names stay observable', async () => {
    const { milestone } = await loadMilestone('1');
    expect(milestone('ready')).not.toBe(milestone('ready'));
  });

  it('formats prompt milestones as <kind>:<id>:<state>', async () => {
    const { promptMilestone } = await loadMilestone('1');
    const decodeName = (marker: string) => {
      const match = /:([A-Za-z0-9_-]+)\x1b\\$/.exec(marker);
      return Buffer.from(match![1], 'base64url').toString('utf8');
    };
    expect(decodeName(promptMilestone('select', 'template', '1'))).toBe('select:template:1');
    expect(decodeName(promptMilestone('confirm', undefined, 'yes'))).toBe('confirm:confirm:yes');
  });
});
