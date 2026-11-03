import { readFile, writeFile } from 'node:fs/promises';

import { resolveDocsSiteOrigin } from '../site-origin.ts';

// Piped installers must fetch legacy from the same production or preview deploy.
const origin = resolveDocsSiteOrigin() || 'https://viteplus.dev';
for (const name of ['install.sh', 'install.ps1', 'install-legacy.sh', 'install-legacy.ps1']) {
  const source = new URL(`../../../packages/cli/${name}`, import.meta.url);
  const destination = new URL(`../../public/${name}`, import.meta.url);
  const content = await readFile(source, 'utf8');
  await writeFile(
    destination,
    content.replace('https://viteplus.dev/install-legacy.', `${origin}/install-legacy.`),
  );
}
