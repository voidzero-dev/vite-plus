import { createRequire } from 'node:module'

const require = createRequire(import.meta.url)

export async function lint(): Promise<{
  binPath: string
  envs: Record<string, string>
}> {
  const binPath = require.resolve('oxlint/bin/oxlint')
  const bin = require.resolve(binPath, {
    paths: [require.resolve('oxlint/package.json')],
  })
  return {
    binPath: bin,
    // TODO: provide envs inference API
    envs: {
      JS_RUNTIME_VERSION: process.versions.node,
      JS_RUNTIME_NAME: process.release.name,
      NODE_PACKAGE_MANAGER: 'vite-plus',
    },
  }
}
