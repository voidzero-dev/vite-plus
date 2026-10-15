# command_staged_output

Seven staged files produce one command result; quiet, verbose, debug, and failures retain their behavior.

## `git init`


## `git add package.json vite.config.ts fail.mjs output.mjs`


## `git -c 'user.name=Vite Plus' -c user.email=vite-plus@example.com commit -m init`


## `git add file1.ts file2.ts file3.ts file4.ts file5.ts file6.ts file7.ts`


## `vp staged`

One result for seven files in a terminal.

```
✔ vp check --fix
```

## `git diff --exit-code`

Task fixes are staged.

```
```

## `git show :file1.ts`

The staged content was formatted.

```
export const value1 = 1;
```

## `git stash list`

The backup stash was cleaned up.

```
```

## `vp staged`

Piped output is compact too.

```
✔ vp check --fix
```

## `vp staged --quiet`

Quiet mode has no success output.

```
```

## `vp staged --verbose`

Verbose mode retains successful task output.

```
✔ vp check --fix

→ vp check --fix:
pass: Formatting completed for checked files (<duration>)
pass: Found no warnings or lint errors in 7 files (<duration>, <n> threads)
```

## `vp staged --debug`

Debug mode retains the full workflow log.

```
lint-staged: Running `lint-staged@<version>` on Node.js <version> (<platform>) +<duration>
lint-staged:execGit: Running git command: [ 'version', '--build-options' ] +<duration>
lint-staged: git version <git-build> +<duration>
lint-staged:validateOptions: Validating options... +<duration>
lint-staged:validateOptions: Validated options: {
  all: false,
  allowEmpty: false,
  color: true,
  concurrent: true,
  configObject: {
    '*.ts': 'vp check --fix',
    '*.js': 'node fail.mjs',
    '*.out': 'node output.mjs'
  },
  configPath: undefined,
  continueOnError: false,
  cwd: undefined,
  debug: true,
  diff: undefined,
  diffFilter: undefined,
  failOnChanges: false,
  hideAll: false,
  hidePartiallyStaged: true,
  hideUnstaged: false,
  maxArgLength: <n>,
  quiet: false,
  relative: false,
  revert: true,
  stash: true,
  verbose: false
} +<duration>
lint-staged: Unset GIT_LITERAL_PATHSPECS (was `undefined`) +<duration>
lint-staged:runAll: Running all tasks... +<duration>
lint-staged:runAll: Using working directory `<workspace>` +<duration>
lint-staged:resolveGitRepo: Resolving git repo from `<workspace>` +<duration>
lint-staged:resolveGitRepo: Unset GIT_DIR (was `undefined`) +<duration>
lint-staged:resolveGitRepo: Unset GIT_WORK_TREE (was `undefined`) +<duration>
lint-staged:execGit: Running git command: [ 'rev-parse', '--show-cdup', '--show-toplevel', '--absolute-git-dir' ] +<duration>
lint-staged:resolveGitRepo: Resolved git repository top-level directory to be `<workspace>` +<duration>
lint-staged:resolveGitRepo: Resolved git config directory to be `<workspace>/.git` +<duration>
lint-staged:execGit: Running git command: [ 'log', '-1' ] +<duration>
lint-staged:execGit: Running git command: [
  'diff',
  '--diff-filter=ACMR',
  '--staged',
  '--ita-visible-in-index',
  '--raw',
  '-z'
] +<duration>
lint-staged:execGit: Running git command: [ 'diff', '--staged', '--ita-visible-in-index', '--name-only', '-z' ] +<duration>
lint-staged:execGit: Running git command: [ 'diff', '--staged', '--ita-invisible-in-index', '--name-only', '-z' ] +<duration>
lint-staged:searchConfigs: Searching for configuration files... +<duration>
lint-staged:searchConfigs: Using single direct configuration object... +<duration>
lint-staged:validateConfig: Validating config from `config object`... +<duration>
lint-staged:validateConfig: Validated config from `config object`:
{
  '*.ts': 'vp check --fix',
  '*.js': 'node fail.mjs',
  '*.out': 'node output.mjs'
} +<duration>
lint-staged:getIntentToAddFiles: Detected 0 files staged with "--intent-to-add": [] +<duration>
lint-staged:file: Reading file `<workspace>/.git/MERGE_HEAD` +<duration>
lint-staged:file: File `<workspace>/.git/MERGE_HEAD` doesn't exist, ignoring... +<duration>
lint-staged:runAll: Loaded list of staged files in git:
[
  {
    filepath: '<workspace>/file1.ts',
    status: 'A'
  },
  {
    filepath: '<workspace>/file2.ts',
    status: 'A'
  },
  {
    filepath: '<workspace>/file3.ts',
    status: 'A'
  },
  {
    filepath: '<workspace>/file4.ts',
    status: 'A'
  },
  {
    filepath: '<workspace>/file5.ts',
    status: 'A'
  },
  {
    filepath: '<workspace>/file6.ts',
    status: 'A'
  },
  {
    filepath: '<workspace>/file7.ts',
    status: 'A'
  }
] +<duration>
lint-staged:groupFilesByConfig: Grouping 7 files by 1 configurations +<duration>
lint-staged:generateTasks: Generating tasks +<duration>
lint-staged:generateTasks: Generated task:
{
  pattern: '*.ts',
  commands: 'vp check --fix',
  fileList: [
    {
      filepath: '<workspace>/file1.ts',
      status: 'A'
    },
    {
      filepath: '<workspace>/file2.ts',
      status: 'A'
    },
    {
      filepath: '<workspace>/file3.ts',
      status: 'A'
    },
    {
      filepath: '<workspace>/file4.ts',
      status: 'A'
    },
    {
      filepath: '<workspace>/file5.ts',
      status: 'A'
    },
    {
      filepath: '<workspace>/file6.ts',
      status: 'A'
    },
    {
      filepath: '<workspace>/file7.ts',
      status: 'A'
    }
  ]
} +<duration>
lint-staged:generateTasks: Generated task:
{ pattern: '*.js', commands: 'node fail.mjs', fileList: [] } +<duration>
lint-staged:generateTasks: Generated task:
{ pattern: '*.out', commands: 'node output.mjs', fileList: [] } +<duration>
lint-staged:getSpawnedTasks: Creating tasks for commands 'vp check --fix' +<duration>
lint-staged:getSpawnedTasks: Creating tasks for commands 'node fail.mjs' +<duration>
lint-staged:getSpawnedTasks: Creating tasks for commands 'node output.mjs' +<duration>
lint-staged:getSpawnedTask: cmd: vp +<duration>
lint-staged:getSpawnedTask: args: [ 'check', '--fix' ] +<duration>
lint-staged:getSpawnedTask: Tinyexec options: {
  nodeOptions: {
    cwd: '<workspace>',
    env: { FORCE_COLOR: 'true' }
  }
} +<duration>
⋯ Backing up original state…
lint-staged:GitWorkflow: Backing up merge state... +<duration>
lint-staged:file: Reading file `<workspace>/.git/MERGE_HEAD` +<duration>
lint-staged:file: File `<workspace>/.git/MERGE_HEAD` doesn't exist, ignoring... +<duration>
lint-staged:file: Reading file `<workspace>/.git/MERGE_MODE` +<duration>
lint-staged:file: File `<workspace>/.git/MERGE_MODE` doesn't exist, ignoring... +<duration>
lint-staged:file: Reading file `<workspace>/.git/MERGE_MSG` +<duration>
lint-staged:file: File `<workspace>/.git/MERGE_MSG` doesn't exist, ignoring... +<duration>
lint-staged:GitWorkflow: Done backing up merge state! +<duration>
lint-staged:GitWorkflow: Getting partially staged files... +<duration>
lint-staged:execGit: Running git command: [ 'status', '-z', '--untracked-files=no' ] +<duration>
lint-staged:GitWorkflow: Found partially staged files: [] +<duration>
lint-staged:execGit: Running git command: [ 'stash', 'create' ] +<duration>
lint-staged:execGit: Running git command: [
  'stash',
  'store',
  '--quiet',
  '--message',
  'lint-staged automatic backup (<id>)',
  '<hash>'
] +<duration>
lint-staged:GitWorkflow: Created backup stash with commit hash <hash> +<duration>
lint-staged:execGit: Running git command: [ 'rev-parse', '--short', '<hash>' ] +<duration>
✔ Done backing up original state (<hash>)!
⋯ Running tasks for staged files…
lint-staged:execGit: Running git command: [
  'diff',
  '--binary',
  '--unified=0',
  '--no-color',
  '--no-ext-diff',
  '--src-prefix=a/',
  '--dst-prefix=b/',
  '--patch',
  '--submodule=short',
  '--raw',
  '-z',
  '--no-renames',
  '--no-textconv'
] +<duration>
lint-staged:runParallelTasks: Running parallel tasks with concurrency: Infinity +<duration>
lint-staged:runParallelTasks: Running configuration: Config object — 7 files +<duration>
lint-staged:runParallelTasks: Running glob: *.ts — 7 files +<duration>
    *.ts — 7 files
lint-staged:runParallelTasks: Running task: vp check --fix +<duration>
      ⋯ vp check --fix
lint-staged:runParallelTasks: Skipped glob from configuration: Config object — 7 files *.js — 0 files +<duration>
lint-staged:runParallelTasks: Skipped glob from configuration: Config object — 7 files *.out — 0 files +<duration>

lint-staged:runParallelTasks: Running workers with concurrency: 1 +<duration>
lint-staged:runParallelTasks: Running task: vp check --fix +<duration>
lint-staged:runParallelTasks: Done running task: vp check --fix +<duration>
✔ vp check --fix
lint-staged:runParallelTasks: Done running workers +<duration>

✔ Done running tasks for staged files!
⋯ Staging changes from tasks…
lint-staged:execGit: Running git command: [
  'diff',
  '--binary',
  '--unified=0',
  '--no-color',
  '--no-ext-diff',
  '--src-prefix=a/',
  '--dst-prefix=b/',
  '--patch',
  '--submodule=short',
  '--raw',
  '-z',
  '--no-renames',
  '--no-textconv'
] +<duration>
lint-staged:execGit: Running git command: [ 'diff', '--diff-filter=ACMR', '--staged', '--name-only', '-z' ] +<duration>
✔ Done staging changes from tasks!
⋯ Cleaning up temporary files…
lint-staged:execGit: Running git command: [ 'stash', 'list', '--format="%H %s"', '-z' ] +<duration>
lint-staged:execGit: Running git command: [ 'stash', 'drop', '--quiet', '0' ] +<duration>
✔ Done cleaning up temporary files!
lint-staged: Tasks were executed successfully! +<duration>
```

## `git -c 'user.name=Vite Plus' -c user.email=vite-plus@example.com commit -m checked`


## `vp staged`

No staged files is still reported.

```
→ lint-staged could not find any staged files.
```

## `vpt write-file note.txt 'no matching task
'`


## `git add note.txt`


## `vp staged`

No matching tasks is still reported.

```
→ lint-staged could not find any staged files matching configured tasks.
```

## `vpt write-file output.out 'output
'`


## `git add output.out`


## `vp staged --quiet --verbose`

Quiet verbose mode preserves indented task output.

```
    Task output
```

## `git -c 'user.name=Vite Plus' -c user.email=vite-plus@example.com commit -m output`


## `vpt write-file fail.js 'export const fail = true;
'`


## `git add fail.js`


## `vp staged`

Failures retain diagnostics and rollback status.

**Exit code:** 1

```
✖ node fail.mjs
✖ Failed to run tasks for staged files!
↓ Skipped staging changes from tasks…
✔ Done reverting to original state!

✖ node fail.mjs:
Task diagnostic
```

## `git diff --exit-code`

The working tree was restored.

```
```

## `git show :fail.js`

The index retains the original staged content.

```
export const fail = true;
```

## `git stash list`

Failure cleanup removed the backup stash.

```
```
