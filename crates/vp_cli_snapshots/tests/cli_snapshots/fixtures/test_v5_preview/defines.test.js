import { expect, test } from 'vite-plus/test';

test('browser global string define', () => {
  expect(globalThis['__VP_STRING_DEFINE__']).toBe('/messages');
});

test('browser global boolean define', () => {
  expect(globalThis['__VP_BOOL_DEFINE__']).toBe(false);
});
