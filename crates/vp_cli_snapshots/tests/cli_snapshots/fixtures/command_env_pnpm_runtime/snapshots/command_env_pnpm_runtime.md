# command_env_pnpm_runtime

## `node assert-env-files.mjs managed`

managed mode disables pnpm runtime management in every generated shell environment

```
All shell environments match managed mode
```

## `node -e 'const {execFileSync}=require('\''node:child_process'\'');const text=execFileSync('\''vp'\'',['\''env'\'','\''doctor'\''],{encoding:'\''utf8'\'',env:{...process.env,PNPM_CONFIG_RUNTIME:'\''false'\''}}).replace(/\u001b\[[0-9;]*m/g,'\'''\'');const line=text.split('\''\n'\'').find(line=>line.includes('\''pnpm runtime'\''));if('\!'line)process.exit(1);console.log(line.trim());'`

doctor reports the managed-mode pnpm runtime setting

```
✓ pnpm runtime      PNPM_CONFIG_RUNTIME=false
```

## `vp env off`


## `node assert-env-files.mjs system-first`

system-first mode removes the setting from every generated shell environment

```
All shell environments match system-first mode
```

## `node -e 'const {execFileSync}=require('\''node:child_process'\'');const text=execFileSync('\''vp'\'',['\''env'\'','\''doctor'\''],{encoding:'\''utf8'\''}).replace(/\u001b\[[0-9;]*m/g,'\'''\'');const line=text.split('\''\n'\'').find(line=>line.includes('\''pnpm runtime'\''));if('\!'line)process.exit(1);console.log(line.trim());'`

doctor reports the system-first pnpm runtime setting

```
✓ pnpm runtime      PNPM_CONFIG_RUNTIME unset
```

## `vp env on`


## `node assert-env-files.mjs managed`

managed mode restores the setting

```
All shell environments match managed mode
```
