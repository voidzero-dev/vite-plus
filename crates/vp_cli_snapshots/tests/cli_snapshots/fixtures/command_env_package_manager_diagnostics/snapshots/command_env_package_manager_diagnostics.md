# command_env_package_manager_diagnostics

## `node -e 'const {execFileSync}=require('\''node:child_process'\'');const info=JSON.parse(execFileSync('\''vp'\'',['\''env'\'','\''current'\'','\''--json'\''],{encoding:'\''utf8'\''}));if(info.package_manager?.name'\!'=='\''npm'\''||info.package_manager?.version'\!'=='\''10.9.4'\''||info.package_manager?.source'\!'=='\''packageManager'\'')process.exit(1);console.log('\''current reports npm packageManager'\'')'`

current reports the npm packageManager pin

```
current reports npm packageManager
```

## `node -e 'const {execFileSync}=require('\''node:child_process'\'');const text=execFileSync('\''vp'\'',['\''env'\'','\''which'\'','\''npm'\''],{encoding:'\''utf8'\''});if('\!'text.includes('\''Package:'\'')||'\!'text.includes('\''npm@10.9.4'\'')||'\!'text.includes('\''package.json'\''))process.exit(1);console.log('\''which reports npm packageManager'\'')'`

which reports the npm packageManager pin

```
which reports npm packageManager
```

## `node -e 'const {execFileSync}=require('\''node:child_process'\'');const text=execFileSync('\''vp'\'',['\''env'\'','\''which'\'','\''npx'\''],{encoding:'\''utf8'\''});if('\!'text.includes('\''Package:'\'')||'\!'text.includes('\''npm@10.9.4'\'')||'\!'text.includes('\''package.json'\''))process.exit(1);console.log('\''which reports npx packageManager'\'')'`

which reports the npx packageManager pin

```
which reports npx packageManager
```
