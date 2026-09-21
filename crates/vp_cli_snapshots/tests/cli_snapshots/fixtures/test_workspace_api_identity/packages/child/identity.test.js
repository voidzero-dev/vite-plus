import { beforeEach, describe, expect, it, vi } from 'vite-plus/test';
import { expect as runnerExpect, vi as runnerVi } from 'vitest';
import { value } from './value.js';

vi.mock('./value.js', () => ({ value: 'mocked' }));
const hook = vi.fn();
beforeEach(hook);

describe('workspace test API', () => {
  it('shares collection, assertions, and mocks with the active runner', () => {
    expect(expect).toBe(runnerExpect);
    expect(vi).toBe(runnerVi);
    expect(value).toBe('mocked');
    expect(hook).toHaveBeenCalledOnce();
    expect(vi.fn()()).toBeUndefined();
  });
});
