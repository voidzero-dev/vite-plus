import { writeFileSync } from 'node:fs';
import { installedChromium } from './chromium.mjs';

// Reuse the browser provisioned for the checkout before installing project peers.
writeFileSync('chromium.json', JSON.stringify(await installedChromium()));
