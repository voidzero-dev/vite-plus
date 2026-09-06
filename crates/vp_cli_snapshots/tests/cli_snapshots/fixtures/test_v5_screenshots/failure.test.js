import { expect, test } from 'vite-plus/test';

test('intentional failure screenshot', () => {
  document.body.innerHTML = '<div style="height:40px;background:red"></div>';
  expect(true, 'fixture deliberately fails to capture its attachment').toBe(false);
});
