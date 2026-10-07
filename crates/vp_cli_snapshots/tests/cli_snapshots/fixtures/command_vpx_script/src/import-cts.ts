// A `.cts` that uses `export` runs as an ES module, so its named exports import.
import { describe } from './both.cts';

console.log(describe());
