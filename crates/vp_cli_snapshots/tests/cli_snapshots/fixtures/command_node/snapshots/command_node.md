# command_node

## `vp node -v`

Shorthand: version resolved from package.json engines.node

```
<version>
```

## `vp node script.js`

Execute a local JS file (primary use case)

```
node version: <version>
script args: []
```

## `vp node script.js foo bar --flag`

Forward script args to the local file

```
node version: <version>
script args: ["foo","bar","--flag"]
```

## `vp node -e 'console.log('\''Hello from vp node'\'')'`

Inline script via -e

```
Hello from vp node
```

## `vp env exec node -v`

Equivalence check: same output as shorthand

```
<version>
```
