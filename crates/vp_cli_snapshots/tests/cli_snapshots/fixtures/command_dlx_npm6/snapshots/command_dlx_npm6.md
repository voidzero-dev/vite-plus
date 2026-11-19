# command_dlx_npm6

npm 6 has no `npm exec`, so `vp dlx` runs its standalone npx.

## `vp dlx -s vp-test-echo hello --flag`

should run vp-test-echo with npx and pass its flags through

```
hello --flag
```

## `vp dlx -s vp-test-echo@1.0.0 hello`

should run specific version

```
hello
```

## `vp dlx -s -p vp-test-upper vp-test-echo@1.0.0 hello`

should install the versioned command next to the extra package

```
hello
```

## `vp dlx -s -p vp-test-echo -p vp-test-upper -c 'echo hello-from-shell | vp-test-echo | vp-test-upper'`

should run shell mode command

```
HELLO-FROM-SHELL
```
