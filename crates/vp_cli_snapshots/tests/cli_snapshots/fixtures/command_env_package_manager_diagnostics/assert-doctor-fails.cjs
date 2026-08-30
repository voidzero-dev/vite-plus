const { spawnSync } = require('node:child_process')

const result = spawnSync('vp', ['env', 'doctor', 'pm'], {
  encoding: 'utf8',
  env: { ...process.env, VP_PACKAGE_MANAGER: 'unknown@1.0.0' },
})
const output = result.stdout.replace(/\u001B\[[0-9;]*m/g, '')
if (result.status !== 1 || !output.includes('Package manager') || !output.includes('Some issues found')) {
  process.stderr.write(`${result.stdout}${result.stderr}`)
  process.exit(1)
}

console.log('doctor returns a failing status for package-manager resolution errors')
