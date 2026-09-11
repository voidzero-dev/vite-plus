# command_version_no_side_effects

## `vp --version`

should print version

```
VITE+ - The Unified Toolchain for the Web

vp <version>

Local vite-plus:
  vite-plus  <version>

Tools:
  vite             <version>
  rolldown         <version>
  vitest           <version>
  oxfmt            <version>
  oxlint           <version>
  oxlint-tsgolint  <version>
  tsdown           <version>

Environment:
  Package manager  Not found
  Node.js          <version>
```

## `vpt stat-file .node-version --assert missing`

no .node-version side effect

```
.node-version: missing
```
