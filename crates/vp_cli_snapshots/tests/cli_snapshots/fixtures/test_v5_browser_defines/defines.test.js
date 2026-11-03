import { expect, test } from 'vite-plus/test';

test('string defines preserve quotes inside the value', () => {
  expect(globalThis['__VP_STRING_DEFINE__']).toBe('/messages?locale="en"');
  expect(__VP_STRING_DEFINE__).toBe('/messages?locale="en"');
});

test('boolean defines retain their values and truthiness', () => {
  expect(globalThis['__VP_FALSE_DEFINE__']).toBe(false);
  expect(Boolean(globalThis['__VP_FALSE_DEFINE__'])).toBe(false);
  expect(__VP_FALSE_DEFINE__).toBe(false);
  expect(globalThis['__VP_TRUE_DEFINE__']).toBe(true);
  expect(__VP_TRUE_DEFINE__).toBe(true);
});

test('number, object, expression, and dotted defines retain Vite semantics', () => {
  expect(globalThis['__VP_NUMBER_DEFINE__']).toBe(0);
  expect(__VP_NUMBER_DEFINE__).toBe(0);
  expect(globalThis['__VP_OBJECT_DEFINE__']).toEqual({ url: '/messages', enabled: false });
  expect(__VP_OBJECT_DEFINE__).toEqual({ url: '/messages', enabled: false });
  expect(globalThis['__VP_EXPRESSION_DEFINE__']).toBe(3);
  expect(__VP_EXPRESSION_DEFINE__).toBe(3);
  expect(globalThis['__VP_DOTTED_DEFINE__']).toBe('dotted');
});
