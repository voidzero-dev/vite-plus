import { symlinkSync } from 'node:fs';

symlinkSync('./configs/nested', 'aliases', process.platform === 'win32' ? 'junction' : 'dir');
