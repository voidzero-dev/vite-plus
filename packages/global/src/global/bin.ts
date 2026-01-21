import { run } from '../../binding/index.js';

run({})
  .then((exitCode) => {
    process.exit(exitCode);
  })
  .catch((err) => {
    console.error('[Vite+] run error:', err);
    process.exit(1);
  });
