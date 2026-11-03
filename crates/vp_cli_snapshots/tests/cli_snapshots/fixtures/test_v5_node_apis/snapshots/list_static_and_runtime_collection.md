# list_static_and_runtime_collection

## `vp test list dynamic.test.js --json`

```
[
  {
    "name": "name",
    "file": "<workspace>/dynamic.test.js",
    "location": {
      "line": 5,
      "column": 28
    }
  }
]
```

## `vpt stat-file collection-ran.txt --assert missing`

```
collection-ran.txt: missing
```

## `vp test list dynamic.test.js --no-static-parse --json`

```
[
  {
    "name": "generated at collection time",
    "file": "<workspace>/dynamic.test.js"
  }
]
```

## `vpt stat-file collection-ran.txt --assert file`

```
collection-ran.txt: file
```
