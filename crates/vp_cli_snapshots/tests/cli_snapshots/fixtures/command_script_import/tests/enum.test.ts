import assert from 'node:assert/strict';
import { test } from 'node:test';
enum Result { Value = 'node:test' }
test('preload reaches the test process', () => {
  assert.equal(Result.Value, 'node:test');
});
