# script_import_node22_sync

The complete preload contract also runs on Node.js 22 synchronous hooks.

## `vpt cp -r dependencies node_modules`


## `node runtime.mjs`

```
{
  "node": "22.23.2",
  "requireExport": "<repo>/packages/cli/dist/register.js",
  "importExport": "file://<repo>/packages/cli/dist/register.js",
  "sameExport": true
}
tsx: MODULE_NOT_FOUND
```

## `node --import=vite-plus/register ./src/main.ts`

```
{
  "value": "default",
  "remapped": "remapped",
  "format": "mts",
  "common": {
    "format": "cts"
  },
  "view": {
    "tag": "section",
    "children": [
      "tsx"
    ]
  },
  "jsx": {
    "tag": "section",
    "children": [
      "jsx"
    ]
  },
  "kind": "esm dependency",
  "dependency": "cjs dependency",
  "total": 5,
  "decorated": [
    "total"
  ]
}
```

## `node --import vite-plus/register ./src/main.cts`

```
{
  "common": {
    "format": "cts"
  },
  "dependency": "cjs dependency",
  "total": 5,
  "decorated": [
    "total"
  ]
}
```

## `node resolved-import.mjs url`

A resolved file URL and an encoded script filename reach Node unchanged.

```
preload: file://<repo>/packages/cli/dist/register.js
encoded filename
```

## `NODE_OPTIONS=--import=vite-plus/register node ./src/main.ts`

```
{
  "value": "default",
  "remapped": "remapped",
  "format": "mts",
  "common": {
    "format": "cts"
  },
  "view": {
    "tag": "section",
    "children": [
      "tsx"
    ]
  },
  "jsx": {
    "tag": "section",
    "children": [
      "jsx"
    ]
  },
  "kind": "esm dependency",
  "dependency": "cjs dependency",
  "total": 5,
  "decorated": [
    "total"
  ]
}
```

## `NODE_OPTIONS=--import vite-plus/register node ./src/main.cts`

```
{
  "common": {
    "format": "cts"
  },
  "dependency": "cjs dependency",
  "total": 5,
  "decorated": [
    "total"
  ]
}
```

## `node src/argv-host.mjs --import=vite-plus/register --import ./src/setup.ts ./src/argv.ts one -- --flag 'space argument'`

Report argv, preload state, process identity, and the script's exit code.

**Exit code:** 7

```
{
  "sameExecutable": true,
  "directChild": true,
  "absoluteScript": true,
  "script": "src/argv.ts",
  "setup": "loaded",
  "args": [
    "one",
    "--",
    "--flag",
    "space argument"
  ],
  "execArgv": [
    "--import=vite-plus/register",
    "--import",
    "./src/setup.ts"
  ]
}
```

## `NODE_OPTIONS=--import=vite-plus/register node src/argv-host.mjs --import ./src/setup.ts ./src/argv.ts one -- --flag 'space argument'`

**Exit code:** 7

```
{
  "sameExecutable": true,
  "directChild": true,
  "absoluteScript": true,
  "script": "src/argv.ts",
  "setup": "loaded",
  "args": [
    "one",
    "--",
    "--flag",
    "space argument"
  ],
  "execArgv": [
    "--import",
    "./src/setup.ts"
  ]
}
```

## `node --import=vite-plus/register --input-type=module --eval 'const { format } = await import('\''./src/module.mts'\''); console.log(format);'`

```
mts
```

## `node --import=vite-plus/register --eval console.log(require('./src/common.cts').format);`

```
cts
```

## `vpt pipe-stdin 'const { format } = await import('\''./src/module.mts'\''); console.log(format);' -- node --import=vite-plus/register --input-type=module -`

```
mts
```

## `vpt pipe-stdin console.log(require('./src/common.cts').format); -- node --import=vite-plus/register -`

```
cts
```

## `node --input-type=module --eval 'await import('\''vite-plus/register'\''); const { format } = await import('\''./src/module.mts'\''); console.log(format);'`

```
mts
```

## `node --import=vite-plus/register --test --test-reporter=dot ./tests/enum.test.ts`

```
.
```

## `cd tests && node --import=vite-plus/register --test --test-reporter=dot`

Node discovers the TypeScript test without an explicit filename.

```
.
```

## `node --import=vite-plus/register ./src/children.mjs`

```
{
  "fork": {
    "message": "fork: default",
    "exitCode": 0
  },
  "esmWorker": {
    "message": "esm worker: cts: default",
    "exitCode": 0
  },
  "cjsWorker": {
    "message": "cjs worker: cts",
    "exitCode": 0
  }
}
```

## `NODE_OPTIONS=--import=vite-plus/register node ./src/children.mjs`

```
{
  "fork": {
    "message": "fork: default",
    "exitCode": 0
  },
  "esmWorker": {
    "message": "esm worker: cts: default",
    "exitCode": 0
  },
  "cjsWorker": {
    "message": "cjs worker: cts",
    "exitCode": 0
  }
}
```

## `NODE_OPTIONS=--import=vite-plus/register node ./src/spawn.mjs`

```
{
  "execArgv": [],
  "exitCode": 0,
  "signal": null
}
{
  "value": "default",
  "remapped": "remapped",
  "format": "mts",
  "common": {
    "format": "cts"
  },
  "view": {
    "tag": "section",
    "children": [
      "tsx"
    ]
  },
  "jsx": {
    "tag": "section",
    "children": [
      "jsx"
    ]
  },
  "kind": "esm dependency",
  "dependency": "cjs dependency",
  "total": 5,
  "decorated": [
    "total"
  ]
}
```

## `TSX_TSCONFIG_PATH=configs with spaces/compat.json node --import=vite-plus/register ./src/children.mjs`

```
{
  "fork": {
    "message": "fork: compat",
    "exitCode": 0
  },
  "esmWorker": {
    "message": "esm worker: cts: compat",
    "exitCode": 0
  },
  "cjsWorker": {
    "message": "cjs worker: cts",
    "exitCode": 0
  }
}
```

## `TSX_TSCONFIG_PATH=configs with spaces/parent.json TS_NODE_PROJECT=configs with spaces/parent.json OXC_TSCONFIG_PATH=configs with spaces/parent.json node --import=vite-plus/register ./src/config-workers.mjs`

```
{
  "parentBefore": {
    "value": "parent",
    "writes": 0,
    "environment": {
      "VP_SCRIPT_TSCONFIG": null,
      "TSX_TSCONFIG_PATH": "configs with spaces/parent.json",
      "TS_NODE_PROJECT": "configs with spaces/parent.json",
      "OXC_TSCONFIG_PATH": "configs with spaces/parent.json"
    },
    "sameEnvironment": true
  },
  "compat": {
    "workers": [
      {
        "ready": {
          "stage": "ready",
          "initial": "worker-one"
        },
        "result": {
          "stage": "result",
          "value": "worker-one",
          "writes": 1
        }
      },
      {
        "ready": {
          "stage": "ready",
          "initial": "worker-two"
        },
        "result": {
          "stage": "result",
          "value": "worker-two",
          "writes": 0
        }
      }
    ],
    "parentDuring": {
      "value": "parent",
      "writes": 0,
      "environment": {
        "VP_SCRIPT_TSCONFIG": null,
        "TSX_TSCONFIG_PATH": "configs with spaces/parent.json",
        "TS_NODE_PROJECT": "configs with spaces/parent.json",
        "OXC_TSCONFIG_PATH": "configs with spaces/parent.json"
      },
      "sameEnvironment": true
    },
    "parentAfter": {
      "value": "parent",
      "writes": 0,
      "environment": {
        "VP_SCRIPT_TSCONFIG": null,
        "TSX_TSCONFIG_PATH": "configs with spaces/parent.json",
        "TS_NODE_PROJECT": "configs with spaces/parent.json",
        "OXC_TSCONFIG_PATH": "configs with spaces/parent.json"
      },
      "sameEnvironment": true
    }
  },
  "upstream": {
    "workers": [
      {
        "ready": {
          "stage": "ready",
          "initial": "worker-one"
        },
        "result": {
          "stage": "result",
          "value": "worker-one",
          "writes": 1
        }
      },
      {
        "ready": {
          "stage": "ready",
          "initial": "worker-two"
        },
        "result": {
          "stage": "result",
          "value": "worker-two",
          "writes": 0
        }
      }
    ],
    "parentDuring": {
      "value": "parent",
      "writes": 0,
      "environment": {
        "VP_SCRIPT_TSCONFIG": null,
        "TSX_TSCONFIG_PATH": "configs with spaces/parent.json",
        "TS_NODE_PROJECT": "configs with spaces/parent.json",
        "OXC_TSCONFIG_PATH": "configs with spaces/parent.json"
      },
      "sameEnvironment": true
    },
    "parentAfter": {
      "value": "parent",
      "writes": 0,
      "environment": {
        "VP_SCRIPT_TSCONFIG": null,
        "TSX_TSCONFIG_PATH": "configs with spaces/parent.json",
        "TS_NODE_PROJECT": "configs with spaces/parent.json",
        "OXC_TSCONFIG_PATH": "configs with spaces/parent.json"
      },
      "sameEnvironment": true
    }
  },
  "explicitAndDefault": {
    "workers": [
      {
        "ready": {
          "stage": "ready",
          "initial": "worker-one"
        },
        "result": {
          "stage": "result",
          "value": "worker-one",
          "writes": 1
        }
      },
      {
        "ready": {
          "stage": "ready",
          "initial": "default"
        },
        "result": {
          "stage": "result",
          "value": "default",
          "writes": 0
        }
      }
    ],
    "parentDuring": {
      "value": "parent",
      "writes": 0,
      "environment": {
        "VP_SCRIPT_TSCONFIG": null,
        "TSX_TSCONFIG_PATH": "configs with spaces/parent.json",
        "TS_NODE_PROJECT": "configs with spaces/parent.json",
        "OXC_TSCONFIG_PATH": "configs with spaces/parent.json"
      },
      "sameEnvironment": true
    },
    "parentAfter": {
      "value": "parent",
      "writes": 0,
      "environment": {
        "VP_SCRIPT_TSCONFIG": null,
        "TSX_TSCONFIG_PATH": "configs with spaces/parent.json",
        "TS_NODE_PROJECT": "configs with spaces/parent.json",
        "OXC_TSCONFIG_PATH": "configs with spaces/parent.json"
      },
      "sameEnvironment": true
    }
  }
}
```

## `node ./src/worker-host.mjs`

```
{
  "message": "esm worker: cts: default"
}
```

## `TSX_TSCONFIG_PATH=configs with spaces/compat.json node --import=vite-plus/register ./src/cluster.mjs`

```
{
  "message": "fork: compat",
  "exitCode": 0
}
```

## `node ./src/internal-host.mjs`

```
{
  "internal": true,
  "unchanged": true
}
```

## `node --import=vite-plus/register --import vite-plus/register ./src/dedup.mjs`

```
{
  "transformType": "function",
  "sameTransform": true,
  "sameResolve": true,
  "format": "mts"
}
```

## `NODE_OPTIONS=--import=vite-plus/register node --import=vite-plus/register ./src/dedup.mjs`

```
{
  "transformType": "function",
  "sameTransform": true,
  "sameResolve": true,
  "format": "mts"
}
```

## `node --import=vite-plus/register ./src/config.ts`

```
{"value":"default","explicit":null,"compat":null,"upstream":null,"tsnode":null}
```

## `TSX_TSCONFIG_PATH=configs with spaces/compat.json node --import=vite-plus/register ./src/config.ts`

```
{"value":"compat","explicit":null,"compat":"configs with spaces/compat.json","upstream":null,"tsnode":null}
```

## `OXC_TSCONFIG_PATH=configs with spaces/upstream.json node --import=vite-plus/register ./src/config.ts`

```
{"value":"upstream","explicit":null,"compat":null,"upstream":"configs with spaces/upstream.json","tsnode":null}
```

## `TS_NODE_PROJECT=configs with spaces/upstream.json node --import=vite-plus/register ./src/config.ts`

```
{"value":"upstream","explicit":null,"compat":null,"upstream":null,"tsnode":"configs with spaces/upstream.json"}
```

## `TS_NODE_PROJECT=configs with spaces/compat.json OXC_TSCONFIG_PATH=configs with spaces/upstream.json node --import=vite-plus/register ./src/config.ts`

```
{"value":"compat","explicit":null,"compat":null,"upstream":"configs with spaces/upstream.json","tsnode":"configs with spaces/compat.json"}
```

## `TSX_TSCONFIG_PATH=configs with spaces/compat.json OXC_TSCONFIG_PATH=configs with spaces/upstream.json node --import=vite-plus/register ./src/config.ts`

```
{"value":"compat","explicit":null,"compat":"configs with spaces/compat.json","upstream":"configs with spaces/upstream.json","tsnode":null}
```

## `VP_SCRIPT_TSCONFIG= TSX_TSCONFIG_PATH=configs with spaces/compat.json node --import=vite-plus/register ./src/config.ts`

```
{"value":"compat","explicit":"","compat":"configs with spaces/compat.json","upstream":null,"tsnode":null}
```

## `VP_SCRIPT_TSCONFIG=configs with spaces/explicit.json TSX_TSCONFIG_PATH=configs with spaces/compat.json OXC_TSCONFIG_PATH=configs with spaces/upstream.json TS_NODE_PROJECT=configs with spaces/upstream.json node --import=vite-plus/register ./src/config.ts`

```
{"value":"explicit","explicit":"configs with spaces/explicit.json","compat":"configs with spaces/compat.json","upstream":"configs with spaces/upstream.json","tsnode":"configs with spaces/upstream.json"}
```

## `node --import=vite-plus/register ./src/error-report.mjs ./error.ts`

Show the application source-map location.

**Exit code:** 1

```
mapped failure
source: new Broken (<workspace>/src/error.ts:4:11)
```

## `node --import=vite-plus/register ./src/error-report.mjs ./dependency-error.ts`

Show the installed dependency's source-map location.

**Exit code:** 1

```
dependency failure
source: fail (<workspace>/node_modules/fixture-ts-import/error.ts:3:9)
```

## `vpx --import=vite-plus/register ./src/dedup.mjs`

The CLI and public preload share one registration.

```
{
  "transformType": "function",
  "sameTransform": true,
  "sameResolve": true,
  "format": "mts"
}
```
