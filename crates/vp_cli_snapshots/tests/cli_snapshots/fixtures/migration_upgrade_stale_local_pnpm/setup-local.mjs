import fs from 'node:fs';
import path from 'node:path';

fs.mkdirSync('node_modules', { recursive: true });
fs.cpSync('local-vite-plus', path.join('node_modules', 'vite-plus'), { recursive: true });
