# command_link_yarn4

## `vpt mkdir -p ../test-lib-yarn`

create test library

```
```

## `vpt write-file ../test-lib-yarn/package.json '{"name": "test-lib-yarn", "version": "1.0.0"}
'`

```
```

## `vp link ../test-lib-yarn`

should link local directory

```
➤ YN0000: · Yarn <version>
➤ YN0000: ┌ Resolution step
➤ YN0000: └ Completed
➤ YN0000: ┌ Fetch step
➤ YN0000: └ Completed
➤ YN0000: ┌ Link step
➤ YN0000: └ Completed
➤ YN0000: · Done in <duration> <duration>
```

## `vpt print-file package.json`

```
{
  "name": "command-link-yarn4",
  "version": "1.0.0",
  "packageManager": "yarn@4.0.0",
  "resolutions": {
    "test-lib-yarn": "portal:<case>/test-lib-yarn"
  }
}
```

## `vp ln ../test-lib-yarn`

should work with ln alias

```
➤ YN0000: · Yarn <version>
➤ YN0000: ┌ Resolution step
➤ YN0000: └ Completed
➤ YN0000: ┌ Fetch step
➤ YN0000: └ Completed
➤ YN0000: ┌ Link step
➤ YN0000: └ Completed
➤ YN0000: · Done in <duration> <duration>
```

## `vpt print-file package.json`

```
{
  "name": "command-link-yarn4",
  "version": "1.0.0",
  "packageManager": "yarn@4.0.0",
  "resolutions": {
    "test-lib-yarn": "portal:<case>/test-lib-yarn"
  }
}
```

## `vp unlink test-lib-yarn`

cleanup temp states

```
➤ YN0000: · Yarn <version>
➤ YN0000: ┌ Resolution step
➤ YN0000: └ Completed
➤ YN0000: ┌ Fetch step
➤ YN0000: └ Completed
➤ YN0000: ┌ Link step
➤ YN0000: └ Completed
➤ YN0000: · Done in <duration> <duration>
```

## `vpt print-file package.json`

```
{
  "name": "command-link-yarn4",
  "version": "1.0.0",
  "packageManager": "yarn@4.0.0"
}
```
