# command_pack_pnpm10_with_workspace

## `vp pm pack`

should pack current workspace root

```
📦  command-pack-pnpm10-with-workspace@1.0.0
Tarball Contents
package.json
packages/app/package.json
packages/utils/package.json
pnpm-workspace.yaml
Tarball Details
command-pack-pnpm10-with-workspace-1.0.0.tgz
```

## `vpt rm -f command-pack-pnpm10-with-workspace-1.0.0.tgz app-1.0.0.tgz vite-plus-test-utils-1.0.0.tgz`


## `node -e 'const {execFileSync}=require('\''node:child_process'\'');const out=JSON.parse(execFileSync('\''vp'\'',['\''pm'\'','\''pack'\'','\''--recursive'\'','\''--json'\''],{encoding:'\''utf8'\''}));out.sort((a,b)=>a.name<b.name?-1:a.name>b.name?1:0);console.log(JSON.stringify(out,null,2));'`

should pack all packages in workspace (sorted by name for determinism)

```
[
  {
    "name": "@vite-plus-test/utils",
    "version": "1.0.0",
    "filename": "<workspace>/vite-plus-test-utils-1.0.0.tgz",
    "files": [
      {
        "path": "package.json"
      }
    ]
  },
  {
    "name": "app",
    "version": "1.0.0",
    "filename": "<workspace>/app-1.0.0.tgz",
    "files": [
      {
        "path": "package.json"
      }
    ]
  },
  {
    "name": "command-pack-pnpm10-with-workspace",
    "version": "1.0.0",
    "filename": "command-pack-pnpm10-with-workspace-1.0.0.tgz",
    "files": [
      {
        "path": "package.json"
      },
      {
        "path": "packages/app/package.json"
      },
      {
        "path": "packages/utils/package.json"
      },
      {
        "path": "pnpm-workspace.yaml"
      }
    ]
  }
]
```

## `vpt print-file out.json`

**Exit code:** 1

```
out.json: not found
missing file
```

*(skipped 1 step(s) to the next line boundary: step failed)*

## `vp pm pack --filter app`

should pack specific package (uses --filter app pack)

```
📦  app@1.0.0
Tarball Contents
package.json
Tarball Details
<workspace>/app-1.0.0.tgz
```

## `vpt rm -f command-pack-pnpm10-with-workspace-1.0.0.tgz app-1.0.0.tgz vite-plus-test-utils-1.0.0.tgz`


## `node -e 'const {execFileSync}=require('\''node:child_process'\'');const out=JSON.parse(execFileSync('\''vp'\'',['\''pm'\'','\''pack'\'','\''--filter'\'','\''app'\'','\''--filter'\'','\''@vite-plus-test/utils'\'','\''--json'\''],{encoding:'\''utf8'\''}));out.sort((a,b)=>a.name<b.name?-1:a.name>b.name?1:0);console.log(JSON.stringify(out,null,2));'`

should pack multiple packages (sorted by name for determinism)

```
[
  {
    "name": "@vite-plus-test/utils",
    "version": "1.0.0",
    "filename": "<workspace>/vite-plus-test-utils-1.0.0.tgz",
    "files": [
      {
        "path": "package.json"
      }
    ]
  },
  {
    "name": "app",
    "version": "1.0.0",
    "filename": "<workspace>/app-1.0.0.tgz",
    "files": [
      {
        "path": "package.json"
      }
    ]
  }
]
```

## `vpt print-file out.json`

**Exit code:** 1

```
out.json: not found
missing file
```

*(skipped 1 step(s) to the next line boundary: step failed)*

## `vp pm pack --out ./dist/package.tgz`

should pack with output file

```
📦  command-pack-pnpm10-with-workspace@1.0.0
Tarball Contents
app-1.0.0.tgz
package.json
packages/app/package.json
packages/utils/package.json
pnpm-workspace.yaml
vite-plus-test-utils-1.0.0.tgz
Tarball Details
<workspace>/dist/package.tgz
```

## `vpt rm -rf ./dist`

```
```

## `vp pm pack --pack-destination ./dist`

should pack with destination

```
📦  command-pack-pnpm10-with-workspace@1.0.0
Tarball Contents
app-1.0.0.tgz
package.json
packages/app/package.json
packages/utils/package.json
pnpm-workspace.yaml
vite-plus-test-utils-1.0.0.tgz
Tarball Details
<workspace>/dist/command-pack-pnpm10-with-workspace-1.0.0.tgz
```

## `vpt rm -rf ./dist`

```
```

## `vp pm pack --pack-gzip-level 9`

should pack with gzip compression level

```
📦  command-pack-pnpm10-with-workspace@1.0.0
Tarball Contents
app-1.0.0.tgz
package.json
packages/app/package.json
packages/utils/package.json
pnpm-workspace.yaml
vite-plus-test-utils-1.0.0.tgz
Tarball Details
command-pack-pnpm10-with-workspace-1.0.0.tgz
```

## `vpt rm -f command-pack-pnpm10-with-workspace-1.0.0.tgz app-1.0.0.tgz vite-plus-test-utils-1.0.0.tgz`


## `vp pm pack --json --out foo-%s-%v.tgz`

should pack with json output

```
{
  "name": "command-pack-pnpm10-with-workspace",
  "version": "1.0.0",
  "filename": "foo-command-pack-pnpm10-with-workspace-1.0.0.tgz",
  "files": [
    {
      "path": "package.json"
    },
    {
      "path": "packages/app/package.json"
    },
    {
      "path": "packages/utils/package.json"
    },
    {
      "path": "pnpm-workspace.yaml"
    }
  ]
}
```

## `vpt rm -f command-pack-pnpm10-with-workspace-1.0.0.tgz app-1.0.0.tgz vite-plus-test-utils-1.0.0.tgz`

