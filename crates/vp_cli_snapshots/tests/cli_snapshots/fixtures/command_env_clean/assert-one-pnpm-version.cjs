const { readdirSync } = require('node:fs')
const { join } = require('node:path')

const versions = readdirSync(join(process.env.VP_HOME, 'package_manager', 'pnpm'), { withFileTypes: true })
  .filter(entry => entry.isDirectory())
  .map(entry => entry.name)

if (versions.length !== 1 || versions[0] === '0.0.1') {
  process.stderr.write(`unexpected pnpm installs: ${versions.join(', ')}\n`)
  process.exit(1)
}

console.log('kept one concrete pnpm fallback')
