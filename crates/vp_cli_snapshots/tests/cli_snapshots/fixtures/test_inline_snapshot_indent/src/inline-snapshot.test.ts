import { describe, expect, it } from 'vite-plus/test';

describe('inline snapshot indentation', () => {
  it('writes multiline snapshots using the surrounding file indentation style', () => {
    expect('alpha\nbeta').toMatchInlineSnapshot();
  });
});
