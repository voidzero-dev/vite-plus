import { test } from 'vitest';
import { page } from 'vitest/browser';

test('Preview locator clicks with real timers', async () => {
  document.body.innerHTML = '<button>Click</button>';
  await page.getByRole('button', { name: 'Click' }).click();
});
