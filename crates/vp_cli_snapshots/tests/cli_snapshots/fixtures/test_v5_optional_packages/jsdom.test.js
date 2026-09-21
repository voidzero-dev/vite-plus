import { expect, test } from 'vite-plus/test';
import { expect as rawExpect } from 'vitest';

test('DOM assignment reaches the jsdom window and shares assertion state', () => {
  expect(expect).toBe(rawExpect);
  globalThis.innerWidth = 412;
  expect(jsdom.window.innerWidth).toBe(412);
  document.body.innerHTML = '<p>jsdom</p>';
  expect(document.querySelector('p').textContent).toBe('jsdom');
});
