import { mkdirSync, writeFileSync } from 'node:fs';
import path from 'node:path';
const dir = 'library';
mkdirSync(path.join(dir, 'src'), { recursive: true });
writeFileSync(path.join(dir, 'package.json'), JSON.stringify({
  name: 'library', type: 'module', private: true,
  scripts: { build: 'vp pack' },
  devDependencies: { 'vite-plus': 'latest', typescript: '7.0.2' },
}));
writeFileSync(path.join(dir, 'vite.config.ts'), `import { defineConfig } from 'vite-plus';
export default defineConfig({ pack: { dts: { tsgo: true }, exports: true } });
`);
writeFileSync(path.join(dir, 'src/index.ts'), 'export const value: number = 42;\n');
writeFileSync(path.join(dir, 'tsconfig.json'), JSON.stringify({
  compilerOptions: { module: 'NodeNext', target: 'ESNext', strict: true },
  include: ['src'],
}));
