import { writeSync } from 'node:fs';

// A preload that APM agents and coverage tools resemble: it must load once and
// its exit hook must run once, in the process that runs Vite.
writeSync(2, 'preload loaded\n');
process.once('exit', () => writeSync(2, 'preload exit hook ran\n'));
