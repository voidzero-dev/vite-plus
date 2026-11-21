const { appendFileSync } = require('node:fs');

const command = process.argv[2];
if (command === '--version' || command === 'config' || command === 'install') {
  appendFileSync('pnpm-hook-events', `${command}\n`);
}

module.exports = { hooks: {} };
