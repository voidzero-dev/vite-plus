import path from 'node:path';

console.log(JSON.stringify({
  sameExecutable: process.execPath === process.env.EXPECTED_EXEC_PATH,
  directChild: process.ppid === Number(process.env.EXPECTED_PARENT_PID),
  absoluteScript: path.isAbsolute(process.argv[1]),
  script: path.relative(process.cwd(), process.argv[1]).split(path.sep).join('/'),
  setup: (globalThis as any).setup,
  args: process.argv.slice(2),
  execArgv: process.execArgv,
}, null, 2));
process.exitCode = 7;
