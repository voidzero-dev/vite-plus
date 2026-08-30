const { spawnSync } = require('node:child_process')
const { writeFileSync } = require('node:fs')

writeFileSync('package.json', JSON.stringify({
  name: 'command-env-install-no-node-with-package-manager',
  private: true,
  packageManager: 'pnpm@10.18.0',
}))

const result = spawnSync('vp', ['env', 'install'], { encoding: 'utf8' })
const output = `${result.stdout}${result.stderr}`
if (result.status !== 1 || !output.includes('Installed pnpm v10.18.0')) {
  process.stderr.write(output)
  process.exit(1)
}

console.log('installed the declared package manager after reporting the missing Node.js pin')
