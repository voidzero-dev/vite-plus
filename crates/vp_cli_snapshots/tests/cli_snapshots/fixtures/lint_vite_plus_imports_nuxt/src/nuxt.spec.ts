import { mockNuxtImport } from '@nuxt/test-utils/runtime';
import { expect, vi } from 'vitest';
import { startVitest } from 'vitest/node';

mockNuxtImport('useExample', () => vi.fn());
void expect;
void startVitest;
