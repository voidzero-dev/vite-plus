# command_create_template_args

Create forwards every token after the first -- without changes.

## `vp create recorder --no-interactive --no-agent --no-editor --no-hooks -- --directory output --flag=value -x -- literal`

```

Generating project…

Running: node <workspace>/packages/args-template/bin/index.mjs --directory output --flag=value -x -- literal

Monorepo integration...

Formatting code...

Code formatted
◇ Scaffolded packages/output
• Node <version>  pnpm <version>
→ Next: cd packages/output && vp run
```

## `vpt print-file packages/output/template-args.json`

```
["--directory", "output", "--flag=value", "-x", "--", "literal"]
```
