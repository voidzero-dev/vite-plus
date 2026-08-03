#!/usr/bin/env node

import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const loadedModule = join(dirname(fileURLToPath(import.meta.url)), 'loaded.js');
// Keep `#` unescaped to reproduce the old install layout's URL-fragment failure.
const loadedModuleUrl = pathToFileURL(loadedModule).href.replaceAll('%23', '#');
await import(loadedModuleUrl);
