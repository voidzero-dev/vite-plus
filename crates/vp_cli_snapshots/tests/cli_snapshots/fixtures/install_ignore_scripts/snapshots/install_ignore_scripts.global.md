# install_ignore_scripts

Install and add preserve --ignore-scripts before or after the package name. A plain install without the flag runs both root and dependency postinstall scripts.

## `npm pack ./scripted-dep --ignore-scripts`


## `vp install --ignore-scripts ./scripted-dep-1.0.0.tgz`


## `vpt stat-file node_modules/scripted-dep/package.json --assert file`

```
node_modules/scripted-dep/package.json: file
```

## `vpt stat-file root-postinstall-ran --assert missing`

```
root-postinstall-ran: missing
```

## `vpt stat-file node_modules/scripted-dep/postinstall-ran --assert missing`

```
node_modules/scripted-dep/postinstall-ran: missing
```

## `vpt rm -rf node_modules package-lock.json`


## `vp install ./scripted-dep-1.0.0.tgz --ignore-scripts`


## `vpt stat-file node_modules/scripted-dep/package.json --assert file`

```
node_modules/scripted-dep/package.json: file
```

## `vpt stat-file root-postinstall-ran --assert missing`

```
root-postinstall-ran: missing
```

## `vpt stat-file node_modules/scripted-dep/postinstall-ran --assert missing`

```
node_modules/scripted-dep/postinstall-ran: missing
```

## `vpt rm -rf node_modules package-lock.json`


## `vp add --ignore-scripts ./scripted-dep-1.0.0.tgz`


## `vpt stat-file node_modules/scripted-dep/package.json --assert file`

```
node_modules/scripted-dep/package.json: file
```

## `vpt stat-file root-postinstall-ran --assert missing`

```
root-postinstall-ran: missing
```

## `vpt stat-file node_modules/scripted-dep/postinstall-ran --assert missing`

```
node_modules/scripted-dep/postinstall-ran: missing
```

## `vpt rm -rf node_modules package-lock.json`


## `vp add ./scripted-dep-1.0.0.tgz --ignore-scripts`


## `vpt stat-file node_modules/scripted-dep/package.json --assert file`

```
node_modules/scripted-dep/package.json: file
```

## `vpt stat-file root-postinstall-ran --assert missing`

```
root-postinstall-ran: missing
```

## `vpt stat-file node_modules/scripted-dep/postinstall-ran --assert missing`

```
node_modules/scripted-dep/postinstall-ran: missing
```

## `vpt rm -rf node_modules package-lock.json`


## `vp install`


## `vpt stat-file root-postinstall-ran --assert file`

```
root-postinstall-ran: file
```

## `vpt stat-file node_modules/scripted-dep/postinstall-ran --assert file`

```
node_modules/scripted-dep/postinstall-ran: file
```
