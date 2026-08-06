# command_pack_yarn4

## `vp pm pack`

should pack current package

```
➤ YN0000: package.json
➤ YN0000: Package archive generated in <workspace>/package.tgz
➤ YN0000: Done in <duration> <duration>
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

## `vp pm pack -- --dry-run`

should support pass through arguments

```
➤ YN0000: dist/package.tgz
➤ YN0000: package.json
➤ YN0000: Done in <duration> <duration>
```
