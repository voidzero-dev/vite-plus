# Cloudflare Workers Builds

When you connect a Git repository to Cloudflare Workers Builds, you can use your project's copy of `vp` for both the build and deploy commands. Cloudflare does not list a global `vp` in its [build image](https://developers.cloudflare.com/workers/ci-cd/builds/build-image/).

## Configure the build and deploy commands

1. Add [`vite-plus` as a development dependency](/guide/local-cli#manual-installation) and commit your lockfile. Leave automatic dependency installation enabled so Workers Builds installs `vp` in `node_modules/.bin` before the build starts. This works with any package manager configured to use `node_modules`.
2. Use the `build` and `deploy` scripts in your `package.json`. Keep any framework-specific steps your project already uses. For example, a Worker that builds with Vite and deploys with Wrangler can use the following scripts.

   ```json [package.json]
   {
     "scripts": {
       "build": "vp build",
       "deploy": "wrangler deploy"
     }
   }
   ```

   This example uses `wrangler` from the project's development dependencies. For help configuring Vite for your Worker, see [Vite+ on Workers](https://developers.cloudflare.com/workers/vite-plugin/vite-plus/).

3. In the Cloudflare dashboard, open your Worker and enter the following commands under [**Settings > Build**](https://developers.cloudflare.com/workers/ci-cd/builds/configuration/).

   | Setting        | Value                               |
   | -------------- | ----------------------------------- |
   | Build command  | `./node_modules/.bin/vp run build`  |
   | Deploy command | `./node_modules/.bin/vp run deploy` |

Both paths are relative to the **Root directory** in your build settings. If a monorepo installs `vite-plus` at the workspace root, adjust the path to its `node_modules/.bin/vp` in both fields. The `run` subcommand executes the [named project script](/guide/run).

## Install dependencies with `vp install`

If you want `vp` to install dependencies, set `SKIP_DEPENDENCY_INSTALL` to `1` under **Settings > Build > Build Variables and Secrets**. This skips Cloudflare's automatic install. Set **Build command** to `bash scripts/workers-build.sh` and commit the following script.

```bash [scripts/workers-build.sh]
set -euo pipefail

export VP_HOME="$HOME/.vite-plus"
export PATH="$VP_HOME/bin:$PATH"
curl -fsSL https://vite.plus | bash

vp install --frozen-lockfile
vp run build
```

Keep **Deploy command** set to `./node_modules/.bin/vp run deploy`. The install step makes the project-local CLI available to the deploy command. To pin the CLI used for installation, set the [`VP_VERSION` build variable](/guide/global-cli#vp-version) to an exact release.
