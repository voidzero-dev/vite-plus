# command_link_npm10

## `vpt mkdir -p ../test-lib-npm`

create test library

```
```

## `vpt write-file ../test-lib-npm/package.json '{"name": "test-lib-npm", "version": "1.0.0"}
'`

```
```

## `vp link ../test-lib-npm`

should link local directory

```

added 1 package, and audited 3 packages in <duration>

found 0 vulnerabilities
```

## `vpt print-file package.json`

```
{
  "name": "command-link-npm10",
  "version": "1.0.0",
  "packageManager": "npm@10.0.0"
}
```

## `vp ln ../test-lib-npm`

should work with ln alias

```

up to date, audited 3 packages in <duration>

found 0 vulnerabilities
```

## `vpt print-file package.json`

```
{
  "name": "command-link-npm10",
  "version": "1.0.0",
  "packageManager": "npm@10.0.0"
}
```

## `vp unlink test-lib-npm`

cleanup temp states

```

removed 1 package, and audited 2 packages in <duration>

found 0 vulnerabilities
```

## `vpt print-file package.json`

```
{
  "name": "command-link-npm10",
  "version": "1.0.0",
  "packageManager": "npm@10.0.0"
}
```
