# oxlint_typeaware

## `vp run lint`

```
$ vp lint ./src
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vpt write-file types.ts 'export type Foo = number;
//comment
'`

append //comment to types.ts

```
```

## `vp run lint`

non-type-aware linting doesn't read types.ts

```
$ vp lint ./src ◉ cache hit, replaying
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.

---
vp run: cache hit, <duration> saved.
```

## `vp run lint-typeaware`

```
$ vp lint --type-aware ./src
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vpt write-file types.ts 'export type Foo = number;
//comment
//comment
'`

append another //comment to types.ts

```
```

## `vp run lint-typeaware`

type-aware linting reads types.ts

```
$ vp lint --type-aware ./src ○ cache miss: 'types.ts' modified, executing
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```
