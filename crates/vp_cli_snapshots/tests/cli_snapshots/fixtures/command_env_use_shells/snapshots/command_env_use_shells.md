# command_env_use_shells

## `VP_SHELL=bash vp env use 20.18.0 pnpm@10.18.0 --no-install`

should detect bash and output both posix exports

```
export VP_NODE_VERSION=20.18.0
export VP_PACKAGE_MANAGER=pnpm@10.18.0
Using Node.js <version> (resolved from 20.18.0)
Using pnpm <version> (resolved from 10.18.0)
```

## `VP_SHELL=zsh vp env use 20.18.0 pnpm@10.18.0 --no-install`

should detect zsh and output both posix exports

```
export VP_NODE_VERSION=20.18.0
export VP_PACKAGE_MANAGER=pnpm@10.18.0
Using Node.js <version> (resolved from 20.18.0)
Using pnpm <version> (resolved from 10.18.0)
```

## `VP_SHELL=fish vp env use 20.18.0 pnpm@10.18.0 --no-install`

should detect fish and output both fish exports

```
set -gx VP_NODE_VERSION 20.18.0
set -gx VP_PACKAGE_MANAGER pnpm@10.18.0
Using Node.js <version> (resolved from 20.18.0)
Using pnpm <version> (resolved from 10.18.0)
```

## `VP_SHELL=nu vp env use 20.18.0 pnpm@10.18.0 --no-install`

should detect nushell and output both nushell exports

```
$env.VP_NODE_VERSION = "20.18.0"
$env.VP_PACKAGE_MANAGER = "pnpm@10.18.0"
Using Node.js <version> (resolved from 20.18.0)
Using pnpm <version> (resolved from 10.18.0)
```

## `VP_SHELL=pwsh vp env use 20.18.0 pnpm@10.18.0 --no-install`

should detect powershell and output both powershell exports

```
$env:VP_NODE_VERSION = "20.18.0"
$env:VP_PACKAGE_MANAGER = "pnpm@10.18.0"
Using Node.js <version> (resolved from 20.18.0)
Using pnpm <version> (resolved from 10.18.0)
```

## `VP_SHELL=cmd vp env use 20.18.0 pnpm@10.18.0 --no-install`

should detect cmd and output both cmd exports

```
set VP_NODE_VERSION=20.18.0
set VP_PACKAGE_MANAGER=pnpm@10.18.0
Using Node.js <version> (resolved from 20.18.0)
Using pnpm <version> (resolved from 10.18.0)
```

## `VP_SHELL=BASH vp env use 20.18.0 pnpm@10.18.0 --no-install`

should detect case-insensitive bash

```
export VP_NODE_VERSION=20.18.0
export VP_PACKAGE_MANAGER=pnpm@10.18.0
Using Node.js <version> (resolved from 20.18.0)
Using pnpm <version> (resolved from 10.18.0)
```

## `VP_SHELL=FISH vp env use 20.18.0 pnpm@10.18.0 --no-install`

should detect case-insensitive fish

```
set -gx VP_NODE_VERSION 20.18.0
set -gx VP_PACKAGE_MANAGER pnpm@10.18.0
Using Node.js <version> (resolved from 20.18.0)
Using pnpm <version> (resolved from 10.18.0)
```

## `VP_SHELL=POWERSHELL vp env use 20.18.0 pnpm@10.18.0 --no-install`

should detect case-insensitive powershell

```
$env:VP_NODE_VERSION = "20.18.0"
$env:VP_PACKAGE_MANAGER = "pnpm@10.18.0"
Using Node.js <version> (resolved from 20.18.0)
Using pnpm <version> (resolved from 10.18.0)
```
