import { existsSync, mkdirSync, realpathSync, symlinkSync, writeFileSync } from 'node:fs';
import path from 'node:path';

const installDir = path.resolve('home/.vite-plus');
mkdirSync(installDir, { recursive: true });

// Reuse the runner's managed runtime in the installation under the parent workspace.
const runtime = path.join(process.env.VP_HOME, 'js_runtime');
if (existsSync(runtime)) {
  symlinkSync(realpathSync(runtime), path.join(installDir, 'js_runtime'), 'junction');
}

if (process.argv.includes('--valid-lockfile')) {
  writeFileSync('home/apps-ts/kami/package.json', '{"name":"kami","private":true}\n');
}
