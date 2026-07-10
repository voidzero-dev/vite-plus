import fs from 'node:fs';
import path from 'node:path';

export default function myVitestPlugin() {
  return {
    name: 'my-vitest-plugin',
    configureVitest() {
      fs.writeFileSync(
        path.join(import.meta.dirname, '.vitest-plugin-loaded'),
        'configureVitest hook executed',
      );
    },
  };
}
