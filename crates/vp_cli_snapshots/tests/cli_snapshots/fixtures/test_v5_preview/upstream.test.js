import { expect, test, vi } from 'vitest';
import { page } from 'vitest/browser';

test('Preview locator clicks with real timers', async () => {
  expect(vi.isFakeTimers()).toBe(false);
  document.body.innerHTML = '<button>Click</button>';
  await page.getByRole('button', { name: 'Click' }).click();
});
