# shim_package_manager_defaults_are_independent

Package-manager defaults are per family so changing Bun cannot silently replace the pnpm shim's configured version.

## `vp env default pnpm@10.18.0`


## `vp env default bun@1.2.0`


## `vpt print-file $VP_HOME/config.json`

pnpm and Bun defaults are persisted independently

```
{
  "defaultPackageManagerVersions": {
    "bun": "1.2.0",
    "pnpm": "10.18.0"
  },
  "packageManagerShimModes": {
    "bun": "managed",
    "npm": "managed",
    "pnpm": "managed",
    "yarn": "managed"
  }
}
```

## `node -e 'const {execFileSync}=require('\''node:child_process'\'');const pnpm=execFileSync('\''pnpm'\'',['\''--version'\''],{encoding:'\''utf8'\''}).trim();const bun=execFileSync('\''bun'\'',['\''--version'\''],{encoding:'\''utf8'\''}).trim();if(pnpm'\!'=='\''10.18.0'\''||bun'\!'=='\''1.2.0'\'')throw new Error(`expected pnpm 10.18.0 and bun 1.2.0, got pnpm ${pnpm} and bun ${bun}`);console.log('\''direct shims use independent defaults'\'')'`

direct package-manager shims use their own configured versions

```
direct shims use independent defaults
```
