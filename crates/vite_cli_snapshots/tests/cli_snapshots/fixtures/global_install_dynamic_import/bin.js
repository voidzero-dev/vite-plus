#!/usr/bin/env node

import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const loadedModule = join(dirname(fileURLToPath(import.meta.url)), 'loaded.js');
await import(loadedModule);
