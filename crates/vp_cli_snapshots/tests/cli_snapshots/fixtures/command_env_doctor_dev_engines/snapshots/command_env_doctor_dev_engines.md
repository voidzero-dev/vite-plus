# command_env_doctor_dev_engines

## `node -e 'const {execFileSync}=require('\''node:child_process'\'');const text=execFileSync('\''vp'\'',['\''env'\'','\''doctor'\''],{encoding:'\''utf8'\''}).replace(/\u001b\[[0-9;]*m/g,'\'''\'');const lines=text.split('\''\n'\'');const start=lines.findIndex(l=>l.trim()==='\''devEngines'\'');if(start===-1){console.error('\''devEngines section not found in doctor output'\'');process.exit(1);}const out=[];for(let i=start;i<lines.length;i++){if(i>start&&lines[i].trim()==='\'''\''){break;}out.push(lines[i].trimEnd());}console.log(out.join('\''\n'\''));'`

print only the deterministic devEngines section of vp env doctor (the other sections are environment-dependent)

```
devEngines
  ⚠ Runtime           .node-version (20.18.0) does not satisfy devEngines.runtime "^24.0.0"
  ⚠ PackageManager    packageManager is "npm@10.5.0" but devEngines.packageManager requires "pnpm"
  note: This will become an error in a future release.
```
