import { run } from '../binding/index.js';
import { build } from './build.js';
import { lint } from './lint.js';

run({
  lint,
  build,
});
