// Drives a watch mode: change a file the script uses and wait for the restart.
//
//   node verify-watch.mjs <scenario> <vpx args before the entry...>
import { spawn } from 'node:child_process';
import { rm, writeFile } from 'node:fs/promises';

const SCENARIOS = {
  // An imported .ts file changes.
  dependency: {
    cwd: '.',
    first: 'value first',
    file: 'dep.ts',
    edit: "export const value: string = 'second';\n",
    restore: "export const value: string = 'first';\n",
    second: 'value second',
    done: 'restarted after an imported .ts file changed',
  },
  // The tsconfig maps `@lib/*` to another folder.
  tsconfig: {
    cwd: 'paths',
    first: 'name a',
    file: 'tsconfig.json',
    edit: '{ "compilerOptions": { "paths": { "@lib/*": ["./b/*"] } } }\n',
    restore: '{ "compilerOptions": { "paths": { "@lib/*": ["./a/*"] } } }\n',
    second: 'name b',
    done: 'restarted after a tsconfig paths edit',
  },
  // The file that `import('./later.js')` looked for is created.
  created: {
    cwd: 'later',
    first: 'later missing',
    file: 'later.ts',
    edit: "export const value: string = 'found';\n",
    restore: undefined,
    second: 'later found',
    done: 'restarted after the file an import looked for was created',
  },
};

const [name, ...vpxArgs] = process.argv.slice(2);
const scenario = SCENARIOS[name];
process.chdir(scenario.cwd);

const child = spawn('vpx', [...vpxArgs, './main.ts'], { stdio: ['ignore', 'pipe', 'pipe'] });
let logs = '';
const waiters = [];
const onData = (data) => {
  logs += data;
  for (const waiter of waiters) {
    waiter();
  }
};
child.stdout.on('data', onData);
child.stderr.on('data', onData);

function waitFor(text) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error(`timed out waiting for ${text}`)), 20000);
    const check = () => {
      if (logs.includes(text)) {
        clearTimeout(timer);
        resolve();
      }
    };
    waiters.push(check);
    check();
  });
}

try {
  await waitFor(scenario.first);
  // The watcher watches a file only once the child reports it, which can be after its
  // first output, and macOS drops a change made before the watcher is ready. Keep
  // rewriting the file until the restart shows up.
  const restarted = waitFor(scenario.second);
  const rewrite = () => writeFile(scenario.file, scenario.edit);
  await rewrite();
  const timer = setInterval(() => rewrite().catch(() => {}), 500);
  try {
    await restarted;
  } finally {
    clearInterval(timer);
  }
  console.log(`vpx ${vpxArgs.join(' ')} ${scenario.done}`);
} catch (error) {
  console.error(logs);
  throw error;
} finally {
  await (scenario.restore === undefined
    ? rm(scenario.file, { force: true })
    : writeFile(scenario.file, scenario.restore));
  child.kill();
}
