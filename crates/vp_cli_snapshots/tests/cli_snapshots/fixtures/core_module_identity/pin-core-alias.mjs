import { readFileSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';

// Before installation, resolve the checkout exposed by the snapshot runner.
// CI can stamp its version, so do not hard-code the committed package version.
// The core version encodes the bundled Vite version, so it differs from the
// vite-plus version (rfcs/core-version-alignment.md).
const require = createRequire(import.meta.url);
const { version } = require('@voidzero-dev/vite-plus-core/package.json');
const project = JSON.parse(readFileSync('package.json', 'utf8'));
project.devDependencies.vite = `npm:@voidzero-dev/vite-plus-core@${version}`;
writeFileSync('package.json', `${JSON.stringify(project, null, 2)}\n`);
