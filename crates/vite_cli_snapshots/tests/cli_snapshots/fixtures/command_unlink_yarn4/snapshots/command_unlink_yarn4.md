# command_unlink_yarn4

## `vpt mkdir -p ../unlink-test-lib-yarn`

create test library

```
```

## `vpt write-file ../unlink-test-lib-yarn/package.json '{"name": "unlink-test-lib-yarn", "version": "1.0.0"}
'`

```
```

## `vp link ../unlink-test-lib-yarn`

link the library first

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
  "name": "command-unlink-yarn4",
  "version": "1.0.0",
  "packageManager": "yarn@4.0.0",
  "resolutions": {
    "unlink-test-lib-yarn": "portal:<case>/unlink-test-lib-yarn"
  }
}
```

## `vp unlink unlink-test-lib-yarn`

should unlink the package

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
  "name": "command-unlink-yarn4",
  "version": "1.0.0",
  "packageManager": "yarn@4.0.0"
}
```

## `vp link ../unlink-test-lib-yarn`

link again

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
  "name": "command-unlink-yarn4",
  "version": "1.0.0",
  "packageManager": "yarn@4.0.0",
  "resolutions": {
    "unlink-test-lib-yarn": "portal:<case>/unlink-test-lib-yarn"
  }
}
```

## `vp unlink --recursive`

should unlink all with --all flag

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
  "name": "command-unlink-yarn4",
  "version": "1.0.0",
  "packageManager": "yarn@4.0.0"
}
```

## `vp unlink -r`

should work with -r short form

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
  "name": "command-unlink-yarn4",
  "version": "1.0.0",
  "packageManager": "yarn@4.0.0"
}
```
