# shim_bun_default_latest_matches_fallback

## `node -e 'const {execFileSync}=require('\''node:child_process'\'');const {writeFileSync}=require('\''node:fs'\'');writeFileSync(process.env.VP_HOME+'\''/bun-fallback-version'\'',execFileSync('\''bun'\'',['\''--version'\''],{encoding:'\''utf8'\''}))'`


## `vp env default bun@latest`


## `NPM_CONFIG_REGISTRY=http://127.0.0.1:9 node -e 'const {execFileSync}=require('\''node:child_process'\'');const {readFileSync}=require('\''node:fs'\'');const expected=readFileSync(process.env.VP_HOME+'\''/bun-fallback-version'\'','\''utf8'\'');const actual=execFileSync('\''bun'\'',['\''--version'\''],{encoding:'\''utf8'\''});if(actual'\!'==expected)throw new Error(`expected ${expected.trim()}, got ${actual.trim()}`);console.log('\''default bun@latest matches the unconfigured fallback'\'')'`

the explicit floating default behaves like the unconfigured Bun fallback

```
default bun@latest matches the unconfigured fallback
```
