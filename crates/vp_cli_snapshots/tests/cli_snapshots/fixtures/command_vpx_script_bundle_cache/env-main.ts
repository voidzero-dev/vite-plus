// Order of execution: a bundled module sets the environment, a CommonJS dependency reads it
// while it loads, and another bundled module runs after both.
import './load-env.ts';
import { url } from 'reads-env';
import './after.ts';

const order = (globalThis as { order?: string[] }).order ?? [];
console.log(`url=${url} order=${order.join(',')}`);
