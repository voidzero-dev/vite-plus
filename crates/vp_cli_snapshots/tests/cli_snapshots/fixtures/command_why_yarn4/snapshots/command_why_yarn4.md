# command_why_yarn4

## `vp install -- --mode=update-lockfile`

should install packages first

```
VITE+ - The Unified Toolchain for the Web

➤ YN0000: · Yarn <version>
➤ YN0000: ┌ Resolution step
➤ YN0085: │ + test-vite-plus-package-optional@npm:1.0.0, test-vite-plus-package@npm:1.0.0, testnpm2@npm:1.0.1
➤ YN0000: └ Completed
➤ YN0000: ┌ Fetch step
➤ YN0013: │ 3 packages were added to the project (+ <size> KiB).
➤ YN0000: └ Completed
➤ YN0000: ┌ Link step
➤ YN0073: │ Skipped due to mode=update-lockfile
➤ YN0000: └ Completed
➤ YN0000: · Done with warnings in <duration>
```

## `vp why testnpm2`

should show why package is installed

```
└─ command-why-yarn4@workspace:.
   └─ testnpm2@npm:1.0.1 (via npm:1.0.1)
```

## `vp explain testnpm2`

should work with explain alias

```
└─ command-why-yarn4@workspace:.
   └─ testnpm2@npm:1.0.1 (via npm:1.0.1)
```

## `vp why test-vite-plus-package`

should show why dev package is installed

```
└─ command-why-yarn4@workspace:.
   └─ test-vite-plus-package@npm:1.0.0 (via npm:1.0.0)
```

## `vp why testnpm2 -r`

should support recursive in yarn@2+

```
└─ command-why-yarn4@workspace:.
   └─ testnpm2@npm:1.0.1 (via npm:1.0.1)
```

## `vp why testnpm2 test-vite-plus-package`

should warn about multiple packages and use first

```
warn: yarn only supports checking one package at a time, using first package
└─ command-why-yarn4@workspace:.
   └─ testnpm2@npm:1.0.1 (via npm:1.0.1)
```

## `vp why testnpm2 --json`

should forward --json to yarn and emit NDJSON

```
{"value":"command-why-yarn4@workspace:.","children":{"testnpm2@npm:1.0.1":{"locator":"testnpm2@npm:1.0.1","descriptor":"testnpm2@npm:1.0.1"}}}
```

## `vp why testnpm2 --long`

should reject --long because yarn does not support it

**Exit code:** 1

```
yarn does not support --long.
```

## `vp why testnpm2 --parseable`

should reject --parseable because yarn does not support it

**Exit code:** 1

```
yarn does not support --parseable.
```

## `vp why testnpm2 -P`

should reject --prod because yarn does not support it

**Exit code:** 1

```
yarn does not support --prod.
```

## `vp why testnpm2 --find-by customFinder`

should reject --find-by because yarn does not support it

**Exit code:** 1

```
yarn does not support --find-by.
```

## `vp why testnpm2 --exclude-peers`

should exclude peers by removing --peers flag

```
└─ command-why-yarn4@workspace:.
   └─ testnpm2@npm:1.0.1 (via npm:1.0.1)
```
