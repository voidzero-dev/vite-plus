import puppeteer from 'puppeteer';
import { expect, test } from 'vitest';

test('should navigate to the home page and find a heading', async () => {
  const browser = await puppeteer.launch();
  const page = await browser.newPage();
  await page.goto('https://voidzero.dev/'); // Replace with your app's URL

  const heading = await page.$eval('h1', el => el.textContent);
  expect(heading).toMatch('Next Generation Tooling');

  await browser.close();
});
