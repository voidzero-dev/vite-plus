#!/usr/bin/env node
import { mkdirSync, writeFileSync } from 'node:fs';

mkdirSync('web', { recursive: true });
writeFileSync('web/package.json', JSON.stringify({
  name: 'web',
  private: true,
  devDependencies: {
    'vite-plus': 'catalog:tools',
    '@vitest/utils': '^5.0.0',
  },
}, null, 2) + '\n');
