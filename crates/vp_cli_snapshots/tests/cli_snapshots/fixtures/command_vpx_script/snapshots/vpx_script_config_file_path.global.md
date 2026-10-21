# vpx_script_config_file_path

Before Node.js 24.19 and 26.1, --experimental-config-file takes a separate .json path, so --tsconfig, --watch, and a watcher option after it keep their meaning.

## `vpx --experimental-config-file custom.json --tsconfig cli.json --env-file=.env ./alias.ts`

--tsconfig after the path wins over the TS_NODE_PROJECT in .env

```
tsconfig cli.json
```

## `vpx --experimental-config-file custom.json --watch --include extra.json`

--watch after the path selects the watcher, which takes --include and reports the missing entry

**Exit code:** 1

```
vpx --watch: missing <entry>
Usage: vpx --watch [options] [node options] [--] <entry> [script args...]
Run "vpx --watch --help" for the options.
```
