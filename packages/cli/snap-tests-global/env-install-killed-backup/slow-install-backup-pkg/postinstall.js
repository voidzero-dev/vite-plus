const fs = require('node:fs');

if (process.env.SLOW_INSTALL_MARKER) {
  fs.writeFileSync(process.env.SLOW_INSTALL_MARKER, 'started');
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 10000);
}

fs.writeFileSync('cli.js', "#!/usr/bin/env node\nconsole.log('slow install backup cli')\n", {
  mode: 0o755,
});
