# berry_user_config_preserves_project

## `vpt json-edit package.json packageManager yarn@4.18.0`


## `vp pm config set npmRegistryServer https://home.example.com --location user`

User writes target the home config without creating a project config.

```
➤ YN0000: Successfully set npmRegistryServer to 'https://home.example.com'
```

## `vpt stat-file .yarnrc.yml --assert missing`

```
.yarnrc.yml: missing
```

## `vp pm config get npmRegistryServer --location user`

```
https://home.example.com
```

## `vp pm config get npmRegistryServer --global`

Global reads stay merged because yarn config get has no --home.

```
https://home.example.com
```

## `vp pm config set npmRegistryServer https://project.example.com --location project`

```
➤ YN0000: Successfully set npmRegistryServer to 'https://project.example.com'
```

## `vp pm config set npmRegistryServer https://updated-home.example.com --location user`

The home value changes, but Yarn reports the effective project value.

```
➤ YN0000: Successfully set npmRegistryServer to 'https://project.example.com'
```

## `vp pm config get npmRegistryServer --location user`

Reads retain project precedence, matching npm and pnpm.

```
https://project.example.com
```

## `vp pm config delete npmRegistryServer --location project`

```
➤ YN0000: Successfully unset npmRegistryServer
```

## `vp pm config get npmRegistryServer --location user`

Removing the project override reveals the updated home value.

```
https://updated-home.example.com
```

## `vp pm config set npmRegistryServer https://project.example.com --location project`

```
➤ YN0000: Successfully set npmRegistryServer to 'https://project.example.com'
```

## `vp pm config delete npmRegistryServer --location user`

Deleting from home must not remove the project setting.

```
➤ YN0000: Successfully unset npmRegistryServer
```

## `vpt print-file .yarnrc.yml`

```
npmRegistryServer: "https://project.example.com"
```

## `vp pm config delete npmRegistryServer --location project`

```
➤ YN0000: Successfully unset npmRegistryServer
```

## `vp pm config get npmRegistryServer --location user`

With both settings deleted, the native default is restored.

```
https://registry.yarnpkg.com
```
