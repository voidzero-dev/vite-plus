import { afterEach, describe, expect, it, vi } from 'vitest';

const originalEnv = process.env.VP_EMIT_MILESTONES;

// Parsed with string ops rather than a regex: the marker embeds ESC (U+001B),
// which `no-control-regex` forbids inside a regex literal.
const ESC = '\x1b';
const TITLE_PREFIX = `${ESC}]2;pty-terminal-test:`;
const TITLE_SUFFIX = `${ESC}\\`;

// Decodes the name from a window-title milestone marker, mirroring vite-task's
// pty_terminal_test_client::decode_milestone_title. Asserts the marker shape
// (OSC 2 ; pty-terminal-test:<32-hex-id>:<base64url(name)> ST) and returns name.
function decodeName(marker: string): string {
  expect(marker.startsWith(TITLE_PREFIX)).toBe(true);
  expect(marker.endsWith(TITLE_SUFFIX)).toBe(true);
  const body = marker.slice(TITLE_PREFIX.length, marker.length - TITLE_SUFFIX.length);
  const id = body.slice(0, 32);
  expect(id).toMatch(/^[0-9a-f]{32}$/);
  expect(body[32]).toBe(':');
  return Buffer.from(body.slice(33), 'base64url').toString('utf8');
}

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
    expect(decodeName(milestone('vp'))).toBe('vp');
  });

  it('uses a fresh id per emission so repeated names stay observable', async () => {
    const { milestone } = await loadMilestone('1');
    expect(milestone('ready')).not.toBe(milestone('ready'));
  });

  it('formats prompt milestones as <kind>:<id>:<state>', async () => {
    const { promptMilestone } = await loadMilestone('1');
    expect(decodeName(promptMilestone('select', 'template', '1'))).toBe('select:template:1');
    expect(decodeName(promptMilestone('confirm', undefined, 'yes'))).toBe('confirm:confirm:yes');
  });
});
