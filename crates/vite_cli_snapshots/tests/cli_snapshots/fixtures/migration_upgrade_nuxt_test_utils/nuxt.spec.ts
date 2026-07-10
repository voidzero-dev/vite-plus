import { mockNuxtImport } from '@nuxt/test-utils/runtime';
import { page } from '@vitest/browser/context';
import { vi } from 'vitest';
import { defineConfig } from 'vitest/config';

mockNuxtImport('useExample', () => vi.fn());
void page;
void defineConfig;
