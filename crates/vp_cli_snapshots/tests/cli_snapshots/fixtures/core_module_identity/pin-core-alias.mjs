import { readFileSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';

// Before installation, resolve the checkout exposed by the snapshot runner.
// CI can stamp its version, so do not hard-code the committed package version.
const require = createRequire(import.meta.url);
const { version } = require('vite-plus/package.json');
const project = JSON.parse(readFileSync('package.json', 'utf8'));
project.devDependencies.vite = `npm:@voidzero-dev/vite-plus-core@${version}`;
writeFileSync('package.json', `${JSON.stringify(project, null, 2)}\n`);
