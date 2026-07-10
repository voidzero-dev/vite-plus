# command_pack_yarn4_with_workspace

## `vp install -- --mode=update-lockfile`

should install packages first

```
VITE+ - The Unified Toolchain for the Web

➤ YN0000: · Yarn <version>
➤ YN0000: ┌ Resolution step
➤ YN0000: └ Completed
➤ YN0000: ┌ Fetch step
➤ YN0000: └ Completed
➤ YN0000: ┌ Link step
➤ YN0073: │ Skipped due to mode=update-lockfile
➤ YN0000: └ Completed
➤ YN0000: · Done with warnings in <duration> <duration>
```

## `vp pm pack`

should pack current workspace root

```
➤ YN0000: package.json
➤ YN0000: Package archive generated in <workspace>/package.tgz
➤ YN0000: Done in <duration> <duration>
```

## `vp pm pack --recursive`

should pack all packages in workspace (uses workspaces foreach --all pack)

```
[command-pack-yarn4-with-workspace]: Process started
[command-pack-yarn4-with-workspace]: ➤ YN0000: package.json
[command-pack-yarn4-with-workspace]: ➤ YN0000: Package archive generated in <workspace>/package.tgz
[command-pack-yarn4-with-workspace]: ➤ YN0000: Done in <duration> <duration>
[command-pack-yarn4-with-workspace]: Process exited (exit code 0), completed in <duration> <duration>

[app]: Process started
[app]: ➤ YN0000: package.json
[app]: ➤ YN0000: Package archive generated in <workspace>/packages/app/package.tgz
[app]: ➤ YN0000: Done in <duration> <duration>
[app]: Process exited (exit code 0), completed in <duration> <duration>

[@vite-plus-test/utils]: Process started
[@vite-plus-test/utils]: ➤ YN0000: package.json
[@vite-plus-test/utils]: ➤ YN0000: Package archive generated in <workspace>/packages/utils/package.tgz
[@vite-plus-test/utils]: ➤ YN0000: Done in <duration> <duration>
[@vite-plus-test/utils]: Process exited (exit code 0), completed in <duration> <duration>

Done in <duration> <duration>
```

## `vp pm pack --filter app`

should pack specific package (uses workspaces foreach --all --include app pack)

```
[app]: Process started
[app]: ➤ YN0000: package.json
[app]: ➤ YN0000: Package archive generated in <workspace>/packages/app/package.tgz
[app]: ➤ YN0000: Done in <duration> <duration>
[app]: Process exited (exit code 0), completed in <duration> <duration>

Done in <duration> <duration>
```

## `vp pm pack --filter app --filter @vite-plus-test/utils`

should pack multiple packages

```
[app]: Process started
[app]: ➤ YN0000: package.json
[app]: ➤ YN0000: Package archive generated in <workspace>/packages/app/package.tgz
[app]: ➤ YN0000: Done in <duration> <duration>
[app]: Process exited (exit code 0), completed in <duration> <duration>

[@vite-plus-test/utils]: Process started
[@vite-plus-test/utils]: ➤ YN0000: package.json
[@vite-plus-test/utils]: ➤ YN0000: Package archive generated in <workspace>/packages/utils/package.tgz
[@vite-plus-test/utils]: ➤ YN0000: Done in <duration> <duration>
[@vite-plus-test/utils]: Process exited (exit code 0), completed in <duration> <duration>

Done in <duration> <duration>
```

## `vp pm pack --out ./dist/package.tgz`

should pack with output file

```
➤ YN0000: package.json
➤ YN0000: Package archive generated in <workspace>/dist/package.tgz
➤ YN0000: Done in <duration> <duration>
```

## `vp pm pack --json`

should pack with json output

```
{"base":"<workspace>"}
{"location":"dist/package.tgz"}
{"location":"package.json"}
{"output":"<workspace>/package.tgz"}
```
