# restore_skipped_generator_output

## `node generate.mjs`

Start with up-to-date outputs.

```
```

## `vp run codegen`

```
$ node generate.mjs
Skipped unchanged message
```

## `vpt rm -r generated manifest.txt`

```
```

## `vp run codegen`

```
$ node generate.mjs ◉ cache hit, replaying
Skipped unchanged message

---
vp run: cache hit, <duration> saved.
```

## `vpt stat-file generated/message.txt --assert missing`

```
generated/message.txt: missing
```

## `vpt print-file manifest.txt`

```
hello
```

## `vpt rm manifest.txt`

Regenerate both outputs before filling the cache with the output glob.

```
```

## `node generate.mjs`

```
```

## `vp run codegen-with-glob`

```
$ node generate.mjs
Skipped unchanged message
```

## `vpt rm -r generated manifest.txt`

```
```

## `vp run codegen-with-glob`

```
$ node generate.mjs ◉ cache hit, replaying
Skipped unchanged message

---
vp run: cache hit, <duration> saved.
```

## `vpt print-file generated/message.txt`

```
HELLO
```

## `vpt print-file manifest.txt`

```
hello
```
