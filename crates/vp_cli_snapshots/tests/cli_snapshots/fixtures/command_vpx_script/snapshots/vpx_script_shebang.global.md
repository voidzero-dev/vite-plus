# vpx_script_shebang

#!/usr/bin/env vpx works for executable script files.

## `vpt chmod +x bin/tool.ts`


## `vpt chmod +x bin/tool`


## `vpt chmod +x bin/commonjs/tool`


## `sh -c ./bin/tool.ts`

An executable .ts file

```
hello from an executable .ts file
```

## `sh -c ./bin/tool`

An extensionless file runs as JavaScript instead of re-entering vpx

```
hello from an extensionless file
```

## `sh -c ./bin/commonjs/tool`

In a package without type, an extensionless file with ES module syntax runs as an ES module

```
hello from tool, an extensionless ES module
```
