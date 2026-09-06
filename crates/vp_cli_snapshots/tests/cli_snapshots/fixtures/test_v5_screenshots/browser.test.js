import { expect, test } from 'vite-plus/test';
import { page } from 'vite-plus/test/browser-playwright/context';

test('fixed viewport and reference image', async () => {
  document.body.style.margin = '0';
  document.body.innerHTML = '<div data-testid="box" style="width:100px;height:40px;background:blue"></div>';
  expect([window.innerWidth, window.innerHeight]).toEqual([800, 600]);
  await expect(page.getByTestId('box')).toMatchScreenshot('box');
  await page.screenshot({ name: 'viewport' });
});
