const fs = require('node:fs');

if (process.env.SLOW_INSTALL_MARKER) {
  fs.writeFileSync(process.env.SLOW_INSTALL_MARKER, 'started');
}

setTimeout(() => {
  fs.writeFileSync('cli.js', "#!/usr/bin/env node\nconsole.log('slow install backup cli')\n", {
    mode: 0o755,
  });
}, 1500);
