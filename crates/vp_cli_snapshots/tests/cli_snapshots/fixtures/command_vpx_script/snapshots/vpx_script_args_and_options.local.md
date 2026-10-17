# vpx_script_args_and_options

Script arguments, Node.js options, and the exit code pass through unchanged.

## `vpx ./src/args.ts a -- --flag`

Arguments after the script belong to it; exit code 7 propagates

**Exit code:** 7

```
args ["a","--","--flag"]
greeting (unset)
setup ran false
```

## `vpx --env-file .env --import ./src/setup.ts ./src/args.ts`

Node.js options before the script, including a TypeScript --import preload

**Exit code:** 7

```
args []
greeting from-env-file
setup ran true
```

## `vpx --eval 'console.log('\''eval without a script'\'')'`

Node.js invocations without a script still run Node.js

```
eval without a script
```
