# stateless_rerun

After upgrading, preserve new v5 choices on reruns without migration metadata. Ignore a legacy preview state file without overwriting it.

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 files had imports rewritten
```

## `vpt stat-file .vite-plus/migrations.json --assert missing`

```
.vite-plus/migrations.json: missing
```

## `vpt write-file vite.config.ts 'export default { test: {} };
'`


## `vpt write-file example.test.ts 'import { expect, test } from '\''vite-plus/test'\'';
test('\''v5 assertion'\'', () => { expect(() => { throw new Error('\''boom'\''); }).toThrow('\'''\''); });
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vpt print-file vite.config.ts example.test.ts`

```
export default { test: {} };
import { expect, test } from 'vite-plus/test';
test('v5 assertion', () => { expect(() => { throw new Error('boom'); }).toThrow(''); });
```

## `vpt stat-file .vite-plus/migrations.json --assert missing`

```
.vite-plus/migrations.json: missing
```

## `vpt write-file .vite-plus/migrations.json 'legacy preview state: invalid JSON
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vpt print-file .vite-plus/migrations.json`

```
legacy preview state: invalid JSON
```
