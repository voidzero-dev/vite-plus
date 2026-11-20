import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readFileSync, realpathSync, writeFileSync } from 'node:fs';

export default {
  defaultPackage: { build: './nested' },
  plugins: [
    {
      name: 'assert-fixed-target',
      configResolved() {
        const expectedNode = `v${readFileSync('.node-version', 'utf8').trim()}`;
        assert.equal(process.version, expectedNode);
        assert.equal(execFileSync('node', ['--version'], { encoding: 'utf8' }).trim(), expectedNode);
        if (process.platform !== 'win32') {
          assert.equal(realpathSync(process.env.PWD), realpathSync(process.cwd()));
        }
        writeFileSync('build-args.json', JSON.stringify(process.argv.slice(2)) + '\n');
      },
    },
  ],
};
