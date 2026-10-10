// A `.cts` with ES module syntax fails with the same error when imported.
import { describe } from './both.cts';

console.log(describe());
