import assert from 'node:assert/strict';
import { value } from '@value';
import { value as remapped } from './remapped.js';
import { format } from './module.mts';
import common from './common.cts';
import { view } from './view.tsx';
import { view as jsx } from './javascript-view.jsx';
import { kind } from 'fixture-ts-import';
import dependency from 'fixture-ts-require';

namespace Options { export const count = 3; }
const decorated: string[] = [];
function record(_target: object, key: string) { decorated.push(key); }
class Counter {
  field = 2;
  constructor(public initial: number) {}
  @record
  total() { return this.initial + this.field; }
}
assert.equal(value, 'default');
assert.equal(remapped, 'remapped');
assert.equal(format, 'mts');
assert.deepEqual(common, { format: 'cts' });
assert.deepEqual(view, { tag: 'section', children: ['tsx'] });
assert.deepEqual(jsx, { tag: 'section', children: ['jsx'] });
assert.equal(kind, 'esm dependency');
assert.equal(dependency, 'cjs dependency');
assert.equal(new Counter(Options.count).total(), 5);
assert.deepEqual(decorated, ['total']);
console.log('esm syntax, resolution, JSX, helpers, and dependencies');
