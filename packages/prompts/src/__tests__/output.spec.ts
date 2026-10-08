import { Writable } from 'node:stream';
import { stripVTControlCharacters } from 'node:util';

import stringWidth from 'fast-string-width';
import color from 'picocolors';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { box } from '../box.js';
import { settings, updateSettings } from '../index.js';
import { log } from '../log.js';
import { note } from '../note.js';
import { spinner } from '../spinner.js';
import { stream } from '../stream.js';
import { taskLog } from '../task-log.js';

function capture(columns = 52, isTTY = false) {
  let content = '';
  const output = Object.assign(
    new Writable({
      write(chunk, _, done) {
        content += chunk.toString();
        done();
      },
    }),
    { columns, isTTY },
  );
  return { output, raw: () => content, text: () => stripVTControlCharacters(content) };
}

function expectNoTerminalControls(output: string) {
  // Forced colors are valid in CI and pipes; cursor movement and erasure are not.
  // oxlint-disable-next-line no-control-regex
  const withoutColors = output.replace(/\x1b\[[\d;]*m/g, '');
  expect(withoutColors).not.toContain('\x1b');
  expect(withoutColors).not.toContain('\r');
}

afterEach(() => {
  updateSettings({ withGuide: false });
  vi.useRealTimers();
  vi.unstubAllEnvs();
});

describe('output identity', () => {
  it('preserves severity without guides or blank lines and aligns wrapped continuations', () => {
    const c = capture(24);
    log.info('Checking assets', { output: c.output });
    log.success('Deployed', { output: c.output });
    log.warn('Long warning that wraps onto several lines', { output: c.output });
    log.error('Build failed\nMissing variable', { output: c.output });
    expect(
      c
        .text()
        .split('\n')
        .map((line) => line.trimEnd())
        .join('\n'),
    ).toBe(
      '● Checking assets\n◆ Deployed\n▲ Long warning that\n  wraps onto several\n  lines\n■ Build failed\n  Missing variable\n',
    );
    expect(
      c
        .text()
        .split('\n')
        .every((line) => stringWidth(line) <= 24),
    ).toBe(true);
  });

  it('keeps plain messages plain, honors explicit spacing, symbols, and global guides', () => {
    const c = capture();
    log.message('plain', { output: c.output });
    log.message('custom\ncontinued', { output: c.output, symbol: '!', spacing: 1 });
    updateSettings({ withGuide: true });
    log.info('guided\ncontinued', { output: c.output });
    log.message('override', { output: c.output, withGuide: false });
    expect(settings.withGuide).toBe(true);
    expect(c.text()).toBe('plain\n\n! custom\n  continued\n● guided\n│ continued\noverride\n');
  });

  it('uses one space in streamed messages and aligns continuation lines', async () => {
    const c = capture();
    const columns = Object.getOwnPropertyDescriptor(process.stdout, 'columns');
    Object.defineProperty(process.stdout, 'columns', { configurable: true, value: 52 });
    const write = vi
      .spyOn(process.stdout, 'write')
      .mockImplementation((chunk) => c.output.write(chunk));
    try {
      await stream.info(['First line\nSecond line']);
      expect(c.text()).toBe('● First line\n  Second line\n');
    } finally {
      write.mockRestore();
      if (columns) {
        Object.defineProperty(process.stdout, 'columns', columns);
      } else {
        Reflect.deleteProperty(process.stdout, 'columns');
      }
    }
  });

  it('uses one space in animated spinner frames', () => {
    vi.stubEnv('CI', '');
    vi.useFakeTimers();
    const c = capture(52, true);
    const s = spinner({ output: c.output });
    s.start('Building');
    vi.advanceTimersByTime(80);
    s.stop('Built');
    expect(c.text()).toContain('◒ Building');
    expect(c.text()).toContain('◐ Building');
    expect(c.text()).not.toMatch(/[◒◐] {2}/);
  });

  it.each([false, true])('encloses notes with both walls when withGuide=%s', (withGuide) => {
    const c = capture(24);
    note('First line\nSecond line', 'Next steps', { output: c.output, withGuide });
    const prefix = withGuide ? '│ ' : '';
    const lines = c.text().trimEnd().split('\n');
    expect(lines[0].startsWith(`${prefix}╭`)).toBe(true);
    expect(lines.at(-1)?.endsWith('╯')).toBe(true);
    expect(
      lines.slice(1, -1).every((line) => line.startsWith(`${prefix}│`) && line.endsWith('│')),
    ).toBe(true);
    expect(new Set(lines.map((line) => stringWidth(line))).size).toBe(1);
    expect(lines.every((line) => stringWidth(line) <= 24)).toBe(true);
    expect(c.raw()).not.toContain('\x1b[2m');
  });

  it.each([8, 12, 24, 52])(
    'fits long Unicode and ANSI titles/content within %s columns',
    (columns) => {
      const c = capture(columns);
      box('日本語 👩‍💻 text with a very long line', color.blue('Extremely long 日本語 👩‍💻 title'), {
        output: c.output,
        width: 0.2,
        withGuide: true,
      });
      const lines = c.text().trimEnd().split('\n');
      expect(lines.every((line) => stringWidth(line) <= columns)).toBe(true);
      expect(new Set(lines.map((line) => stringWidth(line))).size).toBe(1);
    },
  );

  it('renders task logs without destructive terminal controls in pipes', () => {
    const c = capture();
    const task = taskLog({ title: 'Build', output: c.output });
    task.message('Building modules');
    task.error('Build failed');
    expect(c.text()).toContain('◇ Build\n');
    expect(c.text()).toContain('■ Build failed');
    expect(c.text()).toContain('Building modules');
    expectNoTerminalControls(c.raw());
  });
});

describe('spinner', () => {
  it.each(['pipe', 'CI', 'accessible'])('uses static output in %s mode', (mode) => {
    const c = capture(52, mode !== 'pipe');
    if (mode === 'CI') {
      vi.stubEnv('CI', 'true');
    }
    const s = spinner({ output: c.output, accessible: mode === 'accessible' });
    s.start('Building...');
    s.message('Uploading');
    s.stop('Deployed');
    expect(c.text()).toBe('◒ Building...\n◇ Deployed\n');
    expectNoTerminalControls(c.raw());
    expect(c.text()).not.toContain('\n\n');
  });

  it('includes running time while excluding paused time', () => {
    vi.stubEnv('CI', '');
    vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval', 'performance'] });
    const c = capture(52, true);
    const s = spinner({ output: c.output, indicator: 'timer' });
    s.start('Building');
    vi.advanceTimersByTime(2000);
    s.pause();
    vi.advanceTimersByTime(10000);
    s.resume();
    vi.advanceTimersByTime(1000);
    s.stop('Built');
    expect(c.text()).toContain('◒ Building (0s)');
    expect(c.text()).toContain('◇ Built (3s)\n');
    expect(c.text()).not.toContain('(13s)');
  });

  it('cancels an already aborted signal without leaking hooks or a timer', () => {
    const c = capture();
    const signal = AbortSignal.abort();
    const before = process.listenerCount('SIGINT');
    const s = spinner({ output: c.output, signal });
    s.start('Building');
    expect(s.isCancelled).toBe(true);
    expect(c.text()).toContain('■ Canceled');
    expect(process.listenerCount('SIGINT')).toBe(before);
  });
});
