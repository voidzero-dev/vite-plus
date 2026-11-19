# migration_import_rule_runtime_no_install

Reading mode in a dynamic config does not trigger an install on an up-to-date project.

## `vpt write-file vite.config.ts 'export default ({ mode }) => ({ mode });
'`


## `vpt write-file .pnpmfile.cjs 'require('\''node:fs'\'').writeFileSync(require('\''node:path'\'').join(__dirname, '\''install-requested'\''), '\''install'\'');
module.exports = {};
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

● Formatting code...
● Code formatted
◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
✓ Dependencies installed in <duration>
• 2 files had imports rewritten
• Package manager settings configured
```

## `vpt stat-file install-requested --assert file`

The upgrade's required install activates the probe.

```
install-requested: file
```

## `vpt rm install-requested`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vpt stat-file install-requested --assert missing`

```
install-requested: missing
```
