# command_env_doctor_package_manager_modes

Doctor may collapse identical family modes, but must expose per-family rows as soon as one package manager differs.

## `node print-doctor-configuration.cjs`

doctor keeps one package-manager row when every mode matches

```
Configuration
  ✓ Node.js           managed mode
  ✓ Package manager   managed mode
```

## `vp env off pnpm`


## `node print-doctor-configuration.cjs`

doctor prints each package manager when their modes differ

```
Configuration
  ✓ Node.js           managed mode
  ✓ npm               managed mode
  ✓ pnpm              system-first mode
  ✓ Yarn              managed mode
  ✓ Bun               managed mode
```
