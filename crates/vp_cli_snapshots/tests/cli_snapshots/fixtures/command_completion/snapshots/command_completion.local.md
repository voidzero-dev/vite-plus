# command_completion

The hidden completion protocol returns local commands, options, values, tasks, packages, and the vpr view.

## `vp __complete_word__ --shell nu --line 'vp cr'`

completes a local command

```
create
```

## `vp __complete_word__ --shell nu --line 'vp staged --di'`

completes JavaScript command options

```
--diff  Override the default --staged flag of git diff
--diff-filter   Override the default --diff-filter=ACMR flag of git diff
```

## `vp __complete_word__ --shell nu --line 'vp create --package-manager p'`

completes an option value

```
pnpm
```

## `vp __complete_word__ --shell nu --line 'vp run bu'`

completes a package script

```
build
```

## `vp __complete_word__ --shell nu --line 'vp run --filter completion'`

completes a workspace package

```
completion-fixture
```

## `vp __complete_word__ --shell nu --line 'vpr --lo'`

completes the vpr executable view

```
--log   How task output is displayed.
```
