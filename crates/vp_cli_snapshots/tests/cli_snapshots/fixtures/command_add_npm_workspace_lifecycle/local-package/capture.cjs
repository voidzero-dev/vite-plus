const { execFileSync } = require('node:child_process');
const { writeFileSync } = require('node:fs');

const globalPrefix = execFileSync(process.execPath, [process.env.npm_execpath, 'prefix', '-g'], {
  encoding: 'utf8',
}).trim();
writeFileSync('install-context.json', JSON.stringify({ caller: process.env.INIT_CWD, globalPrefix }));
