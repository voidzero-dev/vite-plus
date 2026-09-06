import { expect, test } from 'vite-plus/test';
import { expect as rawExpect } from 'vitest';
import { commands, page } from 'vite-plus/test/browser';
import { page as providerPage } from 'vite-plus/test/browser-webdriverio/context';
import { page as legacyPage } from 'vite-plus/test/browser/providers/webdriverio/context';

test('community provider shares runner state and serialized locators', async () => {
  expect(expect).toBe(rawExpect);
  expect(page).toBe(providerPage);
  expect(page).toBe(legacyPage);
  document.body.innerHTML = '<button>Increment</button><output>0</output>';
  document.querySelector('button').onclick = () => { document.querySelector('output').textContent = '1'; };
  const button = page.getByRole('button', { name: 'Increment' });
  const serialized = await commands.inspectLocator(button);
  expect(serialized.selector).toBeTypeOf('string');
  expect(serialized.locator).toBeTypeOf('string');
  expect(serialized.session).toBe(true);
  await button.click();
  await expect.element(page.getByRole('status')).toHaveTextContent('1');
});
