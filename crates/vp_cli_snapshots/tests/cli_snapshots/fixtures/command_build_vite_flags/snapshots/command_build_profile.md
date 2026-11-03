# command_build_profile

## `vp build --profile`

--profile records a CPU profile of the build

```
✓ 2 modules transformed.
computing gzip size...
dist/index.html  <size> kB │ gzip: <size> kB

✓ built in <duration>
CPU profile written to <workspace>/vite-profile-0.cpuprofile
```

## `vpt stat-file vite-profile-0.cpuprofile --assert file`

the profile is written to the project root

```
vite-profile-0.cpuprofile: file
```
