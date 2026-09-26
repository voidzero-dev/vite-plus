# classic_config_location_preserves_home

## `vp pm config set vite-plus-pm-config-test-key user-value --location user`

Explicit user scope uses Yarn Classic's default home config without a warning.

```
yarn config <version>
warning package.json: No license field
success Set "vite-plus-pm-config-test-key" to "user-value".

Done in <duration>.
```

## `vp pm config get vite-plus-pm-config-test-key --location user`

```
warning package.json: No license field
user-value
```

## `vpt stat-file .yarnrc --assert missing`

```
.yarnrc: missing
```

## `vp pm config delete vite-plus-pm-config-test-key --location user`

```
yarn config <version>
warning package.json: No license field
success Deleted "vite-plus-pm-config-test-key".

Done in <duration>.
```

## `vp pm config get vite-plus-pm-config-test-key --location user`

```
warning package.json: No license field
undefined
```
