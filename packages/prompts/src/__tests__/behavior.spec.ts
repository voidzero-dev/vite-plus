import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, sep } from 'node:path';
import { PassThrough, Writable } from 'node:stream';
import { stripVTControlCharacters } from 'node:util';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { autocomplete, CANCEL_SYMBOL, isCancel, password, path, text } from '../index.js';

const cleanup: (() => void)[] = [];
function terminal() {
  const input = Object.assign(new PassThrough(), {
    isTTY: true,
    isRaw: true,
    setRawMode(value: boolean) {
      this.isRaw = value;
      return this;
    },
  });
  let content = '';
  const output = Object.assign(
    new Writable({
      write(chunk, _, done) {
        content += chunk.toString();
        done();
      },
    }),
    { isTTY: true, columns: 52, rows: 24 },
  );
  cleanup.push(() => {
    input.write('\x03');
    input.destroy();
    output.destroy();
  });
  return { input, output, text: () => stripVTControlCharacters(content) };
}
function files() {
  const temporaryDirectory = mkdtempSync(join(tmpdir(), 'vite-prompts-'));
  // Directory mode scans the parent, so keep it separate from other temporary files.
  const root = join(temporaryDirectory, 'project');
  mkdirSync(root);
  mkdirSync(join(root, 'child'));
  writeFileSync(join(root, 'file.txt'), '');
  cleanup.push(() => rmSync(temporaryDirectory, { recursive: true, force: true }));
  return root;
}
beforeEach(() => {
  vi.stubEnv('TERM', 'xterm-256color');
});
afterEach(() => {
  vi.unstubAllEnvs();
  cleanup
    .splice(0)
    .toReversed()
    .forEach((fn) => fn());
});

describe('real prompt behavior', () => {
  it('renders async validation, reports errors, and accepts corrected input', async () => {
    const t = terminal();
    let resolve!: (value: string | undefined) => void;
    const result = text({
      ...t,
      message: 'Project name',
      validate: (value) =>
        value === 'bad'
          ? new Promise<string | undefined>((r) => {
              resolve = r;
            })
          : undefined,
    });
    t.input.write('bad\r');
    await vi.waitFor(() => expect(t.text()).toContain('Validating…'));
    resolve('Try another name');
    await vi.waitFor(() => expect(t.text()).toContain('Try another name'));
    t.input.write('\x15good\r');
    await expect(result).resolves.toBe('good');
  });

  it('accepts Standard Schema validation and exports the precise cancel sentinel', async () => {
    const t = terminal();
    const result = text({
      ...t,
      message: 'Name',
      validate: {
        '~standard': {
          version: 1,
          vendor: 'test',
          validate: (value) => (value === 'ok' ? { value } : { issues: [{ message: 'Use ok' }] }),
        },
      },
    });
    t.input.write('wrong\r');
    await vi.waitFor(() => expect(t.text()).toContain('Use ok'));
    t.input.write('\x03');
    expect(await result).toBe(CANCEL_SYMBOL);
    expect(isCancel(CANCEL_SYMBOL)).toBe(true);
  });

  it('shows a password placeholder and never renders the secret during async validation', async () => {
    const t = terminal();
    let resolve!: (value: undefined) => void;
    const result = password({
      ...t,
      message: 'Token',
      placeholder: 'Paste API token',
      validate: () =>
        new Promise<undefined>((r) => {
          resolve = r;
        }),
    });
    expect(t.text()).toContain('Paste API token');
    t.input.write('super-secret\r');
    await vi.waitFor(() => expect(t.text()).toContain('Validating…'));
    expect(t.text()).not.toContain('super-secret');
    resolve(undefined);
    await expect(result).resolves.toBe('super-secret');
    expect(t.text()).not.toContain('super-secret');
  });

  it('preserves falsy initial selections', async () => {
    const t = terminal();
    const result = autocomplete({
      ...t,
      message: 'Choose',
      initialValue: 0,
      options: [{ value: 1 }, { value: 0 }],
    });
    t.input.write('\r');
    await expect(result).resolves.toBe(0);
  });

  it('submits the current directory and excludes file suggestions in directory mode', async () => {
    const root = files();
    const t = terminal();
    const result = path({ ...t, message: 'Directory', initialValue: root, directory: true });
    await vi.waitFor(() => expect(t.text()).toContain('Directory'));
    expect(t.text()).not.toContain('file.txt');
    t.input.write('\r');
    await expect(result).resolves.toBe(root);
  });

  it('allows navigation and Tab completion in file mode', async () => {
    const root = files();
    const t = terminal();
    const result = path({ ...t, message: 'File', root: root + sep });
    expect(t.text()).toContain('child');
    expect(t.text()).toContain('file.txt');
    expect(t.text()).toContain('Tab complete');
    t.input.write('\t');
    await vi.waitFor(() => expect(t.text()).toContain('child█'));
    t.input.write('\x03');
    await expect(result).resolves.toBe(CANCEL_SYMBOL);
  });
});
