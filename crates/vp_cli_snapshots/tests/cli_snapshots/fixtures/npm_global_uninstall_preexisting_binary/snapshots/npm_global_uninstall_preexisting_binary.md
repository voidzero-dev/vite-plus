# npm_global_uninstall_preexisting_binary

## `vpt write-file $VP_HOME/bin/npm-global-preexist-cli '#'\!'/bin/sh
echo preexisting-binary-works'`

Create user-owned binary

```
```

## `vpt chmod +x $VP_HOME/bin/npm-global-preexist-cli`

```
```

## `npm-global-preexist-cli`

Verify it works before

```
preexisting-binary-works
```

## `npm install -g ./npm-global-preexist-pkg`

Install pkg that declares same bin name

```

added 1 package in <duration>
```

## `npm uninstall -g npm-global-preexist-pkg`

Should NOT remove the pre-existing binary

```

removed 1 package in <duration>
```

## `npm-global-preexist-cli`

Should still work

```
preexisting-binary-works
```

## `vpt rm $VP_HOME/bin/npm-global-preexist-cli`

Cleanup

```
```
