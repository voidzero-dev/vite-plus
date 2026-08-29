# vite_task_lifecycle_env

Regression test for #2317: `vp run` stamps the package-manager lifecycle env
(npm_execpath, npm_config_user_agent) for package.json scripts, so
child tooling like npm-run-all detects pnpm instead of falling back to npm.
Pre-fix every variable printed `(undefined)`. The fake managed pnpm install
under VP_HOME keeps the case offline; the script normalizes the user-agent
platform/arch tail that suite redaction does not mask.

## `node scripts/setup-fake-pnpm.cjs`


## `vp run check-env`

```
$ node check-env.js ⊘ cache disabled
npm_execpath=<home>/.vite-plus/package_manager/pnpm/<version>/pnpm/bin/pnpm.cjs
npm_config_user_agent=pnpm/<version> npm/? node/<version> <platform> <arch>
```
