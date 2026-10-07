import assert from 'node:assert/strict';
import { test } from 'node:test';

enum Sign {
  Plus = '+',
}

test('enum in a node:test file', () => {
  assert.equal(Sign.Plus, '+');
});
