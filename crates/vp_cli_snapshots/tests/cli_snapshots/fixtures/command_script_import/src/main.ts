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
console.log(JSON.stringify({
  value, remapped, format, common, view, jsx, kind, dependency,
  total: new Counter(Options.count).total(), decorated,
}, null, 2));
