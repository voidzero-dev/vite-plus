# command_outdated_global

## `vp install -g testnpm2@1.0.0`

should prepare global outdated package


## `vp outdated definitely-not-installed-vite-plus-snap-pkg -g --format json`

should support empty global json output

```
{}
```

## `vp outdated testnpm2 -g --format json`

should support global json output

**Exit code:** 1

```
{
  "testnpm2": {
    "current": "1.0.0",
    "wanted": "1.0.1",
    "latest": "1.0.1",
    "dependent": "global",
    "location": "<home>/.vite-plus/packages/testnpm2#<uuid>/lib/node_modules/testnpm2"
  }
}
```

## `vp outdated testnpm2 -g --format list --concurrency 5`

should support global list output

**Exit code:** 1

```
testnpm2 (global)
1.0.0 => 1.0.1
```
