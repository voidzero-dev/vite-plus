const { mkdirSync, writeFileSync } = require('node:fs')
const { join } = require('node:path')

const binRoot = join(
  process.env.VP_HOME,
  'package_manager',
  'npm',
  '10.9.4',
  'npm',
  'bin',
)
const extension = process.platform === 'win32' ? '.cmd' : ''
const contents = process.platform === 'win32' ? '@echo off\r\n' : '#!/bin/sh\n'

mkdirSync(binRoot, { recursive: true })
for (const name of ['npm', 'npx'])
  writeFileSync(join(binRoot, `${name}${extension}`), contents, { mode: 0o755 })
