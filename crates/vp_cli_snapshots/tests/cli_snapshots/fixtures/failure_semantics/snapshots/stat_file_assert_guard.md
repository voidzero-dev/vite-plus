# stat_file_assert_guard

`test -f x && cmd` migrates to a stat-file --assert guard: on mismatch the
guard fails and the guarded command is skipped to the line boundary, exactly
the shell's short-circuit.

## `vpt write-file marker-present.txt here`

```
```

## `vpt stat-file marker-absent.txt --assert file`

guard on a missing marker fails

**Exit code:** 1

```
marker-absent.txt: missing
stat-file assertion failed
```

*(skipped 1 step(s) to the next line boundary: step failed)*

## `vpt stat-file marker-present.txt --assert file`

guard on an existing marker passes

```
marker-present.txt: file
```

## `vpt print 'guarded, runs'`

```
guarded, runs
```
