# line_boundary_on_failure

Locks in the failure-flow contract: a failing step skips the rest of its
line (up to and including the next continue-on-failure step, the line
terminator in migrated fixtures) and the following line resumes; without a
boundary ahead, the case stops.

## `vpt exit 1`

chain member fails: the rest of this line is skipped

**Exit code:** 1

```
```

*(skipped 1 step(s) to the next line boundary: step failed)*

## `vpt print 'next line still runs'`

```
next line still runs
```
