# pm_licenses_pnpm

## `vpt json-edit package.json packageManager pnpm@11.11.0`


## `vp install --ignore-scripts`


## `vp pm licenses list`

pnpm lists installed dependencies

```
┌──────────────────────┬─────────┐
│ Package              │ License │
├──────────────────────┼─────────┤
│ licensed-dep         │ MIT     │
├──────────────────────┼─────────┤
│ unlicensed-dep (dev) │ Unknown │
└──────────────────────┴─────────┘
```

## `vp pm licenses ls --json`

the ls alias preserves native JSON output

```
{
  "MIT": [
    {
      "name": "licensed-dep",
      "versions": [
        null
      ],
      "paths": [
        "<workspace>/node_modules/.pnpm/licensed-dep@file+licensed-dep/node_modules/licensed-dep"
      ],
      "license": "MIT"
    }
  ],
  "Unknown": [
    {
      "name": "unlicensed-dep",
      "versions": [
        null
      ],
      "paths": [
        "<workspace>/node_modules/.pnpm/unlicensed-dep@file+unlicensed-dep/node_modules/unlicensed-dep"
      ],
      "license": "Unknown"
    }
  ]
}
```

## `vp pm licenses list --json -- --prod --long`

native options limit the report to production dependencies

```
{
  "MIT": [
    {
      "name": "licensed-dep",
      "versions": [
        null
      ],
      "paths": [
        "<workspace>/node_modules/.pnpm/licensed-dep@file+licensed-dep/node_modules/licensed-dep"
      ],
      "license": "MIT"
    }
  ]
}
```
