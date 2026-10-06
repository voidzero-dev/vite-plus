# command_why_npm10_with_workspace

## `vpt json-edit package.json dependencies.test-vite-plus-top-package '"1.0.0"'`


## `vp install`

```
VITE+ - The Unified Toolchain for the Web

added 7 packages, and audited 10 packages in <duration>

found 0 vulnerabilities
```

## `vp why testnpm2 --filter app`

should check why in specific workspace using --workspace

```
testnpm2@1.0.0
node_modules/testnpm2
  testnpm2@"1.0.0" from @vite-plus-test/utils@undefined
  packages/utils
    @vite-plus-test/utils@undefined
    node_modules/@vite-plus-test/utils
      @vite-plus-test/utils@"*" from app@undefined
      packages/app
        app@undefined
        node_modules/app
          workspace packages/app from the root project
      workspace packages/utils from the root project
  testnpm2@"1.0.0" from app@undefined
  packages/app
    app@undefined
    node_modules/app
      workspace packages/app from the root project
  testnpm2@"1.0.0" from the root project
```

## `vp why test-vite-plus-package --filter app`

should check why dev dependencies in app workspace

```
test-vite-plus-package@1.0.0
node_modules/test-vite-plus-package
  test-vite-plus-package@"1.0.0" from test-vite-plus-top-package@1.0.0
  node_modules/test-vite-plus-top-package
    test-vite-plus-top-package@"1.0.0" from the root project
  dev test-vite-plus-package@"1.0.0" from app@undefined
  packages/app
    app@undefined
    node_modules/app
      workspace packages/app from the root project
```

## `vp why test-vite-plus-top-package test-vite-plus-install --recursive --json`

recursive workspace selection excludes the root-only dependency

```
[
  {
    "name": "test-vite-plus-install",
    "version": "1.0.0",
    "location": "node_modules/test-vite-plus-install",
    "isWorkspace": false,
    "dependents": [
      {
        "type": "prod",
        "name": "test-vite-plus-install",
        "spec": "1.0.0",
        "from": {
          "name": "app",
          "errors": [
            {}
          ],
          "package": {
            "name": "app",
            "dependencies": {
              "@vite-plus-test/utils": "*",
              "test-vite-plus-install": "1.0.0",
              "testnpm2": "1.0.0"
            },
            "devDependencies": {
              "test-vite-plus-package": "1.0.0"
            },
            "optionalDependencies": {
              "test-vite-plus-other-optional": "1.0.0"
            }
          },
          "location": "packages/app",
          "isWorkspace": true,
          "dependents": [],
          "linksIn": [
            {
              "name": "app",
              "errors": [
                {}
              ],
              "package": {
                "dependencies": {
                  "@vite-plus-test/utils": "*",
                  "test-vite-plus-install": "1.0.0",
                  "testnpm2": "1.0.0"
                },
                "devDependencies": {
                  "test-vite-plus-package": "1.0.0"
                },
                "optionalDependencies": {
                  "test-vite-plus-other-optional": "1.0.0"
                },
                "name": "app"
              },
              "location": "node_modules/app",
              "isWorkspace": true,
              "dependents": [
                {
                  "type": "workspace",
                  "name": "app",
                  "spec": "file:<workspace>/packages/app",
                  "from": {
                    "location": "<workspace>"
                  }
                }
              ]
            }
          ]
        }
      }
    ],
    "dev": false,
    "optional": false,
    "devOptional": false,
    "peer": false,
    "bundled": false,
    "overridden": false
  }
]
```
