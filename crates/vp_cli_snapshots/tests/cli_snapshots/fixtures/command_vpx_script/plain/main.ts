// No legacy decorators here: class features run natively, and an import whose
// bindings are unused at runtime stays because of verbatimModuleSyntax.
import { unused } from './side.ts';

class Counter {
  count = 0;
  #step = 1;
  bump(): number {
    this.count += this.#step;
    return this.count;
  }
}

const native = Counter.toString().includes('count = 0');
console.log('native class fields:', native, new Counter().bump());
void (0 as unknown as typeof unused);
