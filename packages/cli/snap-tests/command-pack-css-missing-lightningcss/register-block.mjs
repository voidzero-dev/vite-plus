import { register } from 'node:module';

// Loaded via `NODE_OPTIONS=--import=./register-block.mjs` so the resolve hook
// applies to the `vp pack` process (and its managed-Node child).
register('./block-lightningcss.mjs', import.meta.url);
