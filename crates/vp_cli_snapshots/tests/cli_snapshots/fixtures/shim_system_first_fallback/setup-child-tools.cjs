// A system npm that reports which Node.js runs it. `npm nested` runs
// `npm --version` again from PATH, as a lifecycle script would: the system npm
// answers with a marker, while the npm bundled with the seeded managed runtime
// prints its version.
const { chmodSync, mkdirSync, writeFileSync } = require('node:fs');
const path = require('node:path');

const npm = path.join('system-bin', 'npm');
writeFileSync(
  npm,
  `#!/usr/bin/env node
const node = process.execPath.includes('system-bin')
  ? 'system'
  : process.execPath.includes('js_runtime')
    ? 'managed'
    : 'other';
if (process.argv[2] !== 'nested') {
  console.log(\`system npm child on \${node} Node.js\`);
  process.exit(0);
}
console.log(\`system npm on \${node} Node.js\`);
const child = require('node:child_process').spawnSync('npm', ['--version'], { stdio: 'inherit' });
process.exit(child.status ?? 1);
`,
);
chmodSync(npm, 0o755);

// A system pnpm, a project-local tool and a system path tool, all on `#!/usr/bin/env node`, that
// report which Node.js runs them.
const reporter = (name) => `#!/usr/bin/env node
const node = process.execPath.includes('system-bin')
  ? 'system'
  : process.execPath.includes('js_runtime')
    ? 'managed'
    : 'other';
console.log(\`${name} on \${node} Node.js\`);
`;
const pnpm = path.join('system-bin', 'pnpm');
writeFileSync(pnpm, reporter('system pnpm'));
chmodSync(pnpm, 0o755);
mkdirSync(path.join('node_modules', '.bin'), { recursive: true });
const localTool = path.join('node_modules', '.bin', 'local-tool');
writeFileSync(localTool, reporter('local tool'));
chmodSync(localTool, 0o755);
// A system tool that is not a core tool, which vpx finds through its own PATH search.
const pathTool = path.join('system-bin', 'path-tool');
writeFileSync(pathTool, reporter('system path tool'));
chmodSync(pathTool, 0o755);
