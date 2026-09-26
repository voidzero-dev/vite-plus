# command_list_yarn1

## `vp install`

should install packages first

```
VITE+ - The Unified Toolchain for the Web

yarn install <version>
warning package.json: No license field
info No lockfile found.
warning command-list-yarn1@1.0.0: No license field
[1/4] Resolving packages...
[2/4] Fetching packages...
[3/4] Linking dependencies...
[4/4] Building fresh packages...

success Saved lockfile.

Done in <duration>.
```

## `NODE_ENV=production vp pm list`

production environment excludes installed dev dependencies

```
yarn list <version>
warning package.json: No license field
warning command-list-yarn1@1.0.0: No license field
└─ testnpm2@1.0.1

Done in <duration>.
```

## `vp pm list testnpm2`

should list specific package

```
yarn list <version>
warning package.json: No license field
warning command-list-yarn1@1.0.0: No license field
warning Filtering by arguments is deprecated. Please use the pattern option instead.
└─ testnpm2@1.0.1

Done in <duration>.
```

## `vp pm list --depth 0`

should list packages with depth limit

```
yarn list <version>
warning package.json: No license field
warning command-list-yarn1@1.0.0: No license field
├─ test-vite-plus-package@1.0.0
└─ testnpm2@1.0.1

Done in <duration>.
```

## `vp pm list --json`

should list packages in JSON format

```
{"type":"warning","data":"package.json: No license field"}
{"type":"warning","data":"command-list-yarn1@1.0.0: No license field"}
{"type":"activityStart","data":{"id":0}}
{"type":"activityTick","data":{"id":0,"name":"testnpm2@1.0.1"}}
{"type":"activityTick","data":{"id":0,"name":"test-vite-plus-package@1.0.0"}}
{"type":"activityEnd","data":{"id":0}}
{"type":"tree","data":{"type":"list","trees":[{"name":"testnpm2@1.0.1","children":[],"hint":null,"color":"bold","depth":0},{"name":"test-vite-plus-package@1.0.0","children":[],"hint":null,"color":"bold","depth":0}]}}
```

## `vp pm list --prod`

should list production dependencies using yarn --production

```
yarn list <version>
warning package.json: No license field
warning command-list-yarn1@1.0.0: No license field
└─ testnpm2@1.0.1

Done in <duration>.
```

## `NODE_ENV=production vp pm list --dev`

include installed dev dependencies using yarn --production=false

```
yarn list <version>
warning package.json: No license field
warning command-list-yarn1@1.0.0: No license field
├─ test-vite-plus-package@1.0.0
└─ testnpm2@1.0.1

Done in <duration>.
```

## `vp pm list --no-optional`

should reject --no-optional because yarn@1 does not support it

**Exit code:** 1

```
yarn does not support --no-optional.
```

## `vp pm list --exclude-peers`

should reject --exclude-peers because yarn@1 does not support it

**Exit code:** 1

```
yarn does not support --exclude-peers.
```

## `vp pm list --only-projects`

should reject --only-projects because yarn@1 does not support it

**Exit code:** 1

```
yarn does not support --only-projects.
```

## `vp pm list --find-by customFinder`

should reject --find-by because yarn@1 does not support it

**Exit code:** 1

```
yarn does not support --find-by.
```

## `vp pm list --recursive`

should reject --recursive because yarn@1 does not support it

**Exit code:** 1

```
yarn does not support --recursive.
```

## `vp pm list --filter app`

should reject --filter because yarn@1 does not support it

**Exit code:** 1

```
yarn does not support --filter.
```

## `vp pm list -- --loglevel=warn`

should support pass through arguments

```
yarn list <version>
warning package.json: No license field
warning command-list-yarn1@1.0.0: No license field
├─ test-vite-plus-package@1.0.0
└─ testnpm2@1.0.1

Done in <duration>.
```
