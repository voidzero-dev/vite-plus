import fs from 'node:fs';
import path from 'node:path';

import { expect, onTestFinished, test } from '@voidzero-dev/vite-plus-test';

test('async plugin factory should load vitest plugin with configureVitest hook', () => {
  const markerPath = path.join(import.meta.dirname, '..', '.vitest-plugin-loaded');
  onTestFinished(() => {
    fs.rmSync(markerPath, { force: true });
  });
  expect(fs.existsSync(markerPath)).toBe(true);
  expect(fs.readFileSync(markerPath, 'utf-8')).toBe('configureVitest hook executed');
});
