#!/bin/sh
# Create mock rolldown-vite source with upstream-changed constants.ts.
# constants.ts uses a different variable name to simulate upstream changes.
set -e

dir="rolldown-vite/packages/vite/src/node"
mkdir -p "$dir"

cat > "$dir/constants.ts" << 'FIXTURE'
import { version } from '../../package.json'

// upstream changed: renamed VERSION to VITE_VERSION
export const VITE_VERSION = version as string
FIXTURE

cat > "$dir/cli.ts" << 'FIXTURE'
import colors from 'picocolors'
import { VERSION } from './constants'

export function createBanner() {
  return `${colors.bold('VITE')} v${VERSION}`
}
FIXTURE

cat > "$dir/build.ts" << 'FIXTURE'
import {
  ROLLUP_HOOKS,
  VERSION,
} from './constants'

export function buildBanner() {
  return `vite v${VERSION} building for production...`
}

export function buildError() {
  return `[vite]: Rolldown failed to resolve`
}
FIXTURE

cat > "$dir/logger.ts" << 'FIXTURE'
export function createLogger(level = 'info', prefix = '[vite]') {
  return { level, prefix }
}
FIXTURE
