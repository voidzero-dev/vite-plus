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

## `vp pm list`

should list installed packages

```
yarn list <version>
warning package.json: No license field
warning command-list-yarn1@1.0.0: No license field
├─ test-vite-plus-package@1.0.0
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

should show warning that --prod not supported by yarn@1

```
warn: yarn@1 does not support --prod, ignoring --prod flag
yarn list <version>
warning package.json: No license field
warning command-list-yarn1@1.0.0: No license field
├─ test-vite-plus-package@1.0.0
└─ testnpm2@1.0.1

Done in <duration>.
```

## `vp pm list --dev`

should show warning that --dev not supported by yarn@1

```
warn: yarn@1 does not support --dev, ignoring --dev flag
yarn list <version>
warning package.json: No license field
warning command-list-yarn1@1.0.0: No license field
├─ test-vite-plus-package@1.0.0
└─ testnpm2@1.0.1

Done in <duration>.
```

## `vp pm list --no-optional`

should show warning that --no-optional not supported by yarn@1

```
warn: yarn@1 does not support --no-optional, ignoring --no-optional flag
yarn list <version>
warning package.json: No license field
warning command-list-yarn1@1.0.0: No license field
├─ test-vite-plus-package@1.0.0
└─ testnpm2@1.0.1

Done in <duration>.
```

## `vp pm list --exclude-peers`

should show warning that --exclude-peers not supported by yarn@1

```
warn: yarn@1 does not support --exclude-peers, ignoring flag
yarn list <version>
warning package.json: No license field
warning command-list-yarn1@1.0.0: No license field
├─ test-vite-plus-package@1.0.0
└─ testnpm2@1.0.1

Done in <duration>.
```

## `vp pm list --only-projects`

should show warning that --only-projects not supported by yarn@1

```
warn: yarn@1 does not support --only-projects, ignoring flag
yarn list <version>
warning package.json: No license field
warning command-list-yarn1@1.0.0: No license field
├─ test-vite-plus-package@1.0.0
└─ testnpm2@1.0.1

Done in <duration>.
```

## `vp pm list --find-by customFinder`

should show warning that --find-by not supported by yarn@1

```
warn: yarn@1 does not support --find-by, ignoring flag
yarn list <version>
warning package.json: No license field
warning command-list-yarn1@1.0.0: No license field
├─ test-vite-plus-package@1.0.0
└─ testnpm2@1.0.1

Done in <duration>.
```

## `vp pm list --recursive`

should show warning that --recursive not supported by yarn@1

```
warn: yarn@1 does not support --recursive, ignoring --recursive flag
yarn list <version>
warning package.json: No license field
warning command-list-yarn1@1.0.0: No license field
├─ test-vite-plus-package@1.0.0
└─ testnpm2@1.0.1

Done in <duration>.
```

## `vp pm list --filter app`

should show warning that --filter not supported by yarn@1

```
warn: yarn@1 does not support --filter, ignoring --filter flag
yarn list <version>
warning package.json: No license field
warning command-list-yarn1@1.0.0: No license field
├─ test-vite-plus-package@1.0.0
└─ testnpm2@1.0.1

Done in <duration>.
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
