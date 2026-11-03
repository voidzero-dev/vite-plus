import { Writable } from 'node:stream';
import { stripVTControlCharacters } from 'node:util';

import { describe, expect, it, vi } from 'vite-plus/test';

import { promptTitle } from '../common.js';
import { log } from '../log.js';
import { spinner } from '../spinner.js';

// Select fallback symbols before common.ts and its consumers initialize.
vi.mock('is-unicode-supported', () => ({ default: () => false }));

function capture() {
  let content = '';
  const output = Object.assign(
    new Writable({
      write(chunk, _, done) {
        content += chunk.toString();
        done();
      },
    }),
    { columns: 52, isTTY: false },
  );
  return { output, text: () => stripVTControlCharacters(content) };
}

describe('Unicode fallback output', () => {
  it('preserves log severity and guide alignment with fallback symbols', () => {
    const c = capture();
    log.info('Checking assets\nChecking config', { output: c.output, withGuide: true });
    log.success('Deployed', { output: c.output });
    log.warn('Missing optional setting', { output: c.output });
    log.error('Build failed', { output: c.output });

    expect(c.text()).toBe(
      '• Checking assets\n| Checking config\n* Deployed\n! Missing optional setting\nx Build failed\n',
    );
  });

  it('uses fallback frames and completion symbols in a static spinner', () => {
    const c = capture();
    const s = spinner({ output: c.output });
    s.start('Building');
    s.stop('Built');

    expect(c.text()).toBe('• Building...\no Built\n');
  });

  it.each([
    ['active', '>'],
    ['submit', 'o'],
    ['cancel', 'x'],
    ['error', 'x'],
  ] as const)('renders a guided %s prompt title with fallback symbols', (state, marker) => {
    const c = capture();
    const title = promptTitle('Choose an option', state, {
      output: c.output,
      withGuide: true,
    });

    expect(stripVTControlCharacters(title)).toBe(`|\n${marker} Choose an option\n`);
  });
});
