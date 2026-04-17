type TestFn = (typeof import('vitest'))['test'];

declare module '@vitest/browser-playwright' {}

import client = require('vite/client');

export type { TestFn };

void client;
