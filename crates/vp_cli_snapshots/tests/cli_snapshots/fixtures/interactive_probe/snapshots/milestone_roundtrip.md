# milestone_roundtrip

Proves the interactive machinery end-to-end without product instrumentation:
wait for a milestone, capture the rendered screen, type a line, wait for the
next milestone, capture again.

## `vpt probe`

**→ expect-milestone:** `probe:ask`

```
What is your name?
```

**← write-line:** `vite-plus`

**→ expect-milestone:** `probe:done`

```
What is your name?
vite-plus
Hello, vite-plus!
```

```
What is your name?
vite-plus
Hello, vite-plus!
```
