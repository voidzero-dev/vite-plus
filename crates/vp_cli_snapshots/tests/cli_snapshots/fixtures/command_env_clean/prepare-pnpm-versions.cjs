const { execFileSync } = require('node:child_process')
const { mkdirSync } = require('node:fs')
const { join } = require('node:path')

const output = execFileSync('vp', ['env', 'current', 'pnpm', '--json'], { encoding: 'utf8' })
const info = JSON.parse(output)
const pnpmRoot = join(process.env.VP_HOME, 'package_manager', 'pnpm')

mkdirSync(join(pnpmRoot, info.package_manager.version), { recursive: true })
mkdirSync(join(pnpmRoot, '0.0.1'), { recursive: true })
