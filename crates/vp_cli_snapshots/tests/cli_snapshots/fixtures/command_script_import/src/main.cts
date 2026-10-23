const assert = require('node:assert/strict');
const common = require('./common.cts');
const dependency: string = require('fixture-ts-require');
const decorated: string[] = [];
function record(_target: object, key: string) { decorated.push(key); }
class Counter {
  field = 2;
  constructor(public initial: number) {}
  @record
  total() { return this.initial + this.field; }
}
assert.deepEqual(common, { format: 'cts' });
assert.equal(dependency, 'cjs dependency');
assert.equal(new Counter(3).total(), 5);
assert.deepEqual(decorated, ['total']);
console.log('cjs syntax, nested require, helpers, and dependencies');
