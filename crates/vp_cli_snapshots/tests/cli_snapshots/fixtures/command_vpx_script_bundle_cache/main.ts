import { fileURLToPath, pathToFileURL } from 'node:url';

import cjs from 'dep-cjs';
import { esm } from 'dep-esm';

import data from './data.json' with { type: 'json' };
import { Color, Store, where } from './lib.ts';
import { counter } from './shared.ts';

const store = new Store('main');
store.add(1);
counter.value++;
console.log(
  `color=${Color.Red} items=${store.items.length} esm=${esm()} cjs=${cjs.cjs()} json=${data.name}`,
);
const isMain = import.meta.url === pathToFileURL(process.argv[1]!).href;
const filename = fileURLToPath(import.meta.url) === import.meta.filename;
console.log(`main=${isMain} filename=${filename} dirname=${where()} class=${Store.name}`);

const mode = process.argv[2];
if (mode === 'lazy') {
  // A literal import() that the recorded run did not reach.
  const { lazy } = await import('./lazy.ts');
  console.log(`lazy=${lazy()} counter=${counter.value}`);
} else if (mode === 'computed') {
  // A computed import(), which the run resolves from this file.
  const name = 'computed';
  const { computed } = await import(`./${name}.ts`);
  console.log(`computed=${computed()} counter=${counter.value}`);
  console.log(`resolve=${import.meta.resolve('dep-esm').endsWith('/dep-esm/index.js')}`);
} else if (mode === 'throw') {
  store.fail();
}
