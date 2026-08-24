# command_env_powershell

## `vp env setup --refresh`


## `vpt write-file .node-version '22.18.0
'`


## `EXPECTED_VP_HOME=${VP_HOME} pwsh -NoLogo -NoProfile -NonInteractive -File assert.ps1`

dot-sources env.ps1 and verifies PowerShell environment setup and wrapper behavior

```
Using Node.js <version> (resolved from 20.18.0)
Reverted to file-based Node.js version resolution
Using Node.js <version> (resolved from .node-version)
PowerShell environment checks passed
```
