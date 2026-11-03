# shim_bun_default_latest_matches_fallback

## `node -e 'const {execFileSync}=require('\''node:child_process'\'');const {writeFileSync}=require('\''node:fs'\'');writeFileSync(process.env.VP_HOME+'\''/bun-fallback-version'\'',execFileSync('\''bun'\'',['\''--version'\''],{encoding:'\''utf8'\''}))'`


## `vp env default bun@latest`


## `node assert-registry-cache.mjs bun`

the explicit floating default behaves like the unconfigured Bun fallback

```
default bun@latest matches the unconfigured fallback
```
