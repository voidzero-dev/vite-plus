import { writeFileSync } from 'node:fs';

// The task runner also loads this config, so only the build records its runtime.
if (process.argv.includes('build')) {
  writeFileSync('node-major.txt', `node ${process.versions.node.split('.')[0]}`);
}

export default {};
