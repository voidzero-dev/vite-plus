# pm_licenses_bun

## `vpt json-edit package.json packageManager bun@1.4.0`


## `vp install --ignore-scripts`


## `vp pm licenses list`

Bun lists installed dependencies at the minimum supported version

```
MIT (1)
└── licensed-dep@licensed-dep

Unknown (1)
└── unlicensed-dep@unlicensed-dep (dev)
```

## `vp pm licenses ls --json`

the ls alias preserves native JSON output, including unknown licenses

```
{
  "MIT": [
    {
      "name": "licensed-dep",
      "versions": ["licensed-dep"],
      "paths": ["<workspace>/node_modules/licensed-dep"],
      "license": "MIT"
    }
  ],
  "Unknown": [
    {
      "name": "unlicensed-dep",
      "versions": ["unlicensed-dep"],
      "paths": ["<workspace>/node_modules/unlicensed-dep"],
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
      "versions": ["licensed-dep"],
      "paths": ["<workspace>/node_modules/licensed-dep"],
      "license": "MIT"
    }
  ]
}
```
