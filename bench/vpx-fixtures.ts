import { mkdirSync, writeFileSync } from 'node:fs';
import path from 'node:path';

const modules = 300;

export const vpxFixtures = [
  { name: 'hello', entry: 'hello.ts', output: 'hello' },
  { name: 'esm-app', entry: 'index.ts', output: 'total 44850' },
  { name: 'cjs-app', entry: 'index.ts', output: 'total 44850' },
  { name: 'large-cjs', entry: 'main.ts', output: 'total 4950' },
  { name: 'large-esm', entry: 'main.ts', output: 'total 4950' },
] as const;

function moduleSource(i: number): string {
  return `export interface Item${i} {
  id: number;
  name: string;
}

export enum Kind${i} {
  A = 'a${i}',
  B = 'b${i}',
}

export class Store${i} {
  items: Item${i}[] = [];

  constructor(private readonly prefix: string) {}

  add(id: number): Item${i} {
    const item = { id, name: \`\${this.prefix}-\${id}\` };
    this.items.push(item);
    return item;
  }
}

export function run${i}(): number {
  return new Store${i}(Kind${i}.A).add(${i}).id;
}
`;
}

/** Keep the historical startup fixtures identical across benchmark runners. */
export function generateVpxFixtures(root: string, afterMain = ''): void {
  function write(file: string, content: string): void {
    const target = path.join(root, file);
    mkdirSync(path.dirname(target), { recursive: true });
    writeFileSync(target, content);
  }

  write('hello/package.json', '{ "type": "module" }\n');
  write('hello/hello.ts', `const message: string = 'hello';\nconsole.log(message);\n${afterMain}`);

  const index = [
    ...Array.from({ length: modules }, (_, i) => `import { run${i} } from './mod${i}.ts';`),
    'let total = 0;',
    ...Array.from({ length: modules }, (_, i) => `total += run${i}();`),
    "console.log('total', total);",
  ].join('\n');
  for (const [name, packageJson] of [
    ['esm-app', '{ "type": "module" }\n'],
    ['cjs-app', '{ "name": "cjs-app" }\n'],
  ]) {
    write(`${name}/package.json`, packageJson);
    write(`${name}/index.ts`, `${index}\n${afterMain}`);
    for (let i = 0; i < modules; i++) {
      write(`${name}/mod${i}.ts`, moduleSource(i));
    }
  }

  // One 1.8 MB file without import/export, in each module format.
  let large = '';
  for (let i = 0; i < 12000; i++) {
    large +=
      `interface Shape${i} { id: number; label: string }\n` +
      `function make${i}(id: number): Shape${i} { return { id, label: 'shape-${i}-' + id.toString(16) }; }\n`;
  }
  large +=
    "let total: number = 0;\nfor (let i = 0; i < 100; i++) total += make0(i).id;\nconsole.log('total', total);\n";
  write('large-cjs/package.json', '{ "name": "large-cjs" }\n');
  write('large-cjs/main.ts', large + afterMain);
  write('large-esm/package.json', '{ "type": "module" }\n');
  write('large-esm/main.ts', large + afterMain);
}
