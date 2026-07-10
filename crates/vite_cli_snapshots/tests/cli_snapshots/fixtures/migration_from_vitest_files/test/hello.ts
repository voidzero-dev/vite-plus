import { server } from '@vitest/browser-playwright/context';
import { test, describe, expect, it } from 'vitest';

const { readFile } = server.commands;

describe('Hello', () => {
  it('should return the correct result', () => {
    expect(true).toBe(true);
  });
});
