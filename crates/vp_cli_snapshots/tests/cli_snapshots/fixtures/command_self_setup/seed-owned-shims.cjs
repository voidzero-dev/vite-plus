const { symlinkSync } = require('node:fs');

// Model the old layout with owned links, rather than unrelated executable files.
for (const tool of ['node', 'npm', 'pnpm', 'pnpx']) {
  symlinkSync('../current/bin/vp', `home/bin/${tool}`);
}
