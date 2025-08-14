import { run } from '../binding/index.js';
import { lint } from './lint.js';
import { test } from './test.js';
import { vite } from './vite.js';

run({
  lint,
  vite,
  test,
});
