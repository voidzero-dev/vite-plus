import assert from 'node:assert/strict';
import { createRequire } from 'node:module';

export async function installedChromium() {
  const require = createRequire(import.meta.resolve('vite-plus/package.json'));
  const { chromium } = require('playwright');
  const executablePath = chromium.executablePath();
  // WebDriverIO's Windows detection expects version-named directories beside
  // chrome.exe, which Playwright does not provide. Read the actual browser
  // version so driver selection cannot fall back to an unrelated system Chrome.
  const browser = await chromium.launch({ executablePath, args: ['--no-sandbox'] });
  try {
    const browserVersion = browser.version();
    assert.match(browserVersion, /^\d+\.\d+\.\d+\.\d+$/);
    return { executablePath, browserVersion };
  } finally {
    await browser.close();
  }
}
