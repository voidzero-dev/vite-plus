type TestFn = (typeof import('vitest'))['test'];
type BrowserContext = typeof import('@vitest/browser/context');
type BrowserClient = typeof import('@vitest/browser/client');
type PlaywrightProvider = typeof import('@vitest/browser-playwright/provider');

declare module '@vitest/browser-playwright' {}
declare module '@vitest/browser-playwright/context' {}

import client = require('vite/client');

export type { BrowserClient, BrowserContext, PlaywrightProvider, TestFn };

void client;
