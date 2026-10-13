import assert from 'node:assert/strict';
import { appendFileSync, existsSync, readdirSync } from 'node:fs';
import { join } from 'node:path';

const [operation, ...directories] = process.argv.slice(2);
assert.ok(['check', 'mutate', 'absent'].includes(operation));
assert.ok(directories.length > 0);

function cacheFiles(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) return cacheFiles(path);
    // CACHEDIR.TAG marks the parent directory; it contains no cached test data.
    return entry.isFile() && entry.name !== 'CACHEDIR.TAG' ? [path] : [];
  });
}

for (const directory of directories) {
  if (operation === 'absent') {
    // Parent directories and their cache tags can be recreated on a cache hit.
    assert.deepEqual(existsSync(directory) ? cacheFiles(directory) : [], []);
    console.log(`No cache files restored in ${directory}`);
    continue;
  }
  const files = cacheFiles(directory);
  assert.ok(files.length > 0, `No Vitest cache files in ${directory}`);
  if (operation === 'mutate') {
    for (const file of files) appendFileSync(file, '\n');
    console.log(`Modified cache files in ${directory}`);
  } else {
    console.log(`Cache files exist in ${directory}`);
  }
}
