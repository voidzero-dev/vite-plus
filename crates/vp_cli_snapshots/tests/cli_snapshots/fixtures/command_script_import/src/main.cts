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
console.log(JSON.stringify({
  common, dependency, total: new Counter(3).total(), decorated,
}, null, 2));
