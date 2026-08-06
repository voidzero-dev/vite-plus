# pm_dedupe_npm

## `vp dedupe -- --json`

npm deduplicates dependencies

```
{
  "add": [],
  "added": 0,
  "audited": 1,
  "change": [],
  "changed": 0,
  "funding": 0,
  "remove": [],
  "removed": 0,
  "audit": {
    "vulnerabilities": {
      "info": 0,
      "low": 0,
      "moderate": 0,
      "high": 0,
      "critical": 0,
      "total": 0
    },
    "dependencies": {
      "prod": 1,
      "dev": 0,
      "optional": 0,
      "peer": 0,
      "peerOptional": 0,
      "total": 0
    }
  }
}
```

## `vpt print-file package.json`

verify npm completed

```
{
  "name": "pm-dedupe-npm",
  "version": "1.0.0",
  "private": true,
  "license": "MIT",
  "packageManager": "npm@11.13.0"
}
```
