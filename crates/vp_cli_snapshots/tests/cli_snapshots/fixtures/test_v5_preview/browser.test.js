import { expect, test, vi } from 'vite-plus/test';
import { expect as upstreamExpect } from 'vitest';
import { page } from 'vite-plus/test/browser';
import { page as compatibilityPage } from 'vite-plus/test/browser/context';
import { page as providerPage } from 'vite-plus/test/browser-preview/context';
import { page as bareContextPage } from 'vite-plus/test/context';
import { page as pluginPage } from 'vite-plus/test/plugins/browser-context';
import { page as providerAliasPage } from 'vite-plus/test/browser/providers/preview/context';

test('Preview shares runtime and browser context across public entry points', async () => {
  expect(expect).toBe(upstreamExpect);
  expect(page).toBe(compatibilityPage);
  expect(page).toBe(providerPage);
  expect(page).toBe(bareContextPage);
  expect(page).toBe(pluginPage);
  expect(page).toBe(providerAliasPage);
  document.body.innerHTML = '<button type="button">Increment</button><output>0</output>';
  document.querySelector('button').addEventListener('click', () => { document.querySelector('output').textContent = '1'; });
  // Upstream #9891 regressed Preview real timers in 4.1.1, also affecting 5.0.1.
  // The separate upstream_real_timers case records this pre-existing failure.
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
  try {
    await page.getByRole('button', { name: 'Increment' }).click();
  } finally {
    vi.useRealTimers();
  }
  await expect.element(page.getByRole('status')).toHaveTextContent('1');
});
