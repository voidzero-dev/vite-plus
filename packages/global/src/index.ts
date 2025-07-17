import { join } from 'node:path';
import { parseArgs } from 'node:util';
import { questionnaire } from './command/new';

try {
  const { positionals, values } = parseArgs({
    allowPositionals: true,
    options: { help: { type: 'boolean', short: 'h' } },
  });

  const [command] = positionals;

  if (values.help) {
    console.log(`Usage: vite [command] [options] -- [arguments for command]

vite new               Scaffold new project
vite build [dir]       Run vite build (default in: ".")
vite optimize [dir]    Run vite optimize
vite preview [dir]     Run vite preview
vite dev [dir]         Run vite dev
vite lint [dir]        Run oxlint
vite lib [dir]         Run tsdown
vite test [dir]        Run vitest
vite bench [dir]       Run vitest bench
vite docs [dir]        Run vitepress
vite task [name]       Run package.json#scripts[name] in each workspace`);
  } else if (command === 'new') {
    await questionnaire();
  } else {
    await import(join(process.cwd(), 'node_modules/vite-plus/dist/cli.js'));
  }
} catch (e: any) {
  if (e && e.status) process.exit(e.status);
  throw e;
}
