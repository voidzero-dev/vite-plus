# pm_licenses_yarn

## `vpt json-edit package.json packageManager yarn@1.22.22`


## `vp install --ignore-scripts`


## `vp pm licenses list`

Yarn Classic lists installed dependencies

```
yarn licenses <version>
├─ MIT
│  └─ licensed-dep@1.0.0
└─ UNKNOWN
   └─ unlicensed-dep@1.0.0

Done in <duration>.
```

## `vp pm licenses ls --json`

the ls alias preserves Yarn Classic's JSON events

```
{"type":"progressStart","data":{"id":0,"total":2}}
{"type":"progressTick","data":{"id":0,"current":1}}
{"type":"progressTick","data":{"id":0,"current":2}}
{"type":"progressFinish","data":{"id":0}}
{"type":"table","data":{"head":["Name","Version","License","URL","VendorUrl","VendorName"],"body":[["licensed-dep","1.0.0","MIT","Unknown","Unknown","Unknown"],["unlicensed-dep","1.0.0","UNKNOWN","Unknown","Unknown","Unknown"]]}}
```

## `vp pm licenses list --json -- --production`

Yarn receives its native production option

```
{"type":"progressStart","data":{"id":0,"total":1}}
{"type":"progressTick","data":{"id":0,"current":1}}
{"type":"progressFinish","data":{"id":0}}
{"type":"table","data":{"head":["Name","Version","License","URL","VendorUrl","VendorName"],"body":[["licensed-dep","1.0.0","MIT","Unknown","Unknown","Unknown"]]}}
```
