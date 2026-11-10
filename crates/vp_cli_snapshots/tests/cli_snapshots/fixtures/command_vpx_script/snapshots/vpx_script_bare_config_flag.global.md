# vpx_script_bare_config_flag

From Node.js 24.19 and 26.1, a bare --experimental-config-file reads node.config.json and takes no value, so the arguments after it keep their meaning.

## `vpx --experimental-config-file ./main.ts --watch`

--watch after the script belongs to the script

```
args ["--watch"] config true
```

## `vpx --experimental-config-file --tsconfig cli.json --env-file=.env ./alias.ts`

--tsconfig is still vpx's, and wins over the TS_NODE_PROJECT in .env

```
tsconfig cli.json
```

## `vpx --experimental-config-file --watch --include extra.json`

--watch still selects the watcher, which takes --include and reports the missing entry

**Exit code:** 1

```
vpx --watch: missing <entry>
Usage: vpx --watch [options] [node options] [--] <entry> [script args...]
Run "vpx --watch --help" for the options.
```
