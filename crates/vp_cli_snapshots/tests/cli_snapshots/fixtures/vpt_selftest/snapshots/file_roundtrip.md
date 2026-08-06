# file_roundtrip

vpt setup/assertion helpers behave identically across platforms.

## `vpt write-file notes/hello.txt 'hello from vpt'`

```
```

## `vpt print-file notes/hello.txt`

```
hello from vpt
```

## `vpt stat-file notes/hello.txt missing.txt`

```
notes/hello.txt: file
missing.txt: missing
```

## `vpt list-dir notes`

```
hello.txt
```

## `vpt json-edit package.json scripts.build 'vp build'`

```
```

## `vpt print-file package.json`

```
{
  "name": "vpt-selftest",
  "private": true,
  "scripts": {
    "build": "vp build"
  }
}
```

## `vpt touch-file created-by-touch.txt`

touch-file creates missing files

```
```

## `vpt stat-file created-by-touch.txt notes`

stat-file reports the entry type: file, dir, symlink, or missing

```
created-by-touch.txt: file
notes: dir
```

## `vpt rm -f never-existed.txt`

rm -f ignores missing targets

```
```

## `vpt cp created-by-touch.txt notes`

cp into an existing directory, like real cp

```
```

## `vpt list-dir notes`

```
created-by-touch.txt
hello.txt
```

## `vpt list-dir notes/hello.txt`

list-dir on a file prints the path, like ls

```
notes/hello.txt
```

## `vpt chmod +x created-by-touch.txt`

symbolic +x is accepted (no-op on Windows)

```
```

## `vpt pipe-stdin -- vpt read-stdin`

empty pipe-stdin data means empty stdin, not a bare newline

```
```

## `vpt pipe-stdin hello -- vpt read-stdin`

```
hello
```

## `vpt touch-file multi-a.txt multi-b.txt`

touch-file creates every operand

```
```

## `vpt stat-file multi-b.txt`

```
multi-b.txt: file
```

## `vpt mkdir existing-dir`

```
```

## `vpt cp -r notes existing-dir`

cp -r into an existing directory nests like real cp

```
```

## `vpt list-dir existing-dir/notes`

```
created-by-touch.txt
hello.txt
```

## `vpt grep-file notes/hello.txt 'from vpt'`

grep-file succeeds on a match

```
notes/hello.txt: found "from vpt"
```

## `vpt grep-file notes/hello.txt 'absent text'`

grep-file fails like grep when the pattern is absent

**Exit code:** 1

```
notes/hello.txt: missing "absent text"
pattern not found
```

## `vpt print-file no-such-file.txt`

print-file fails like cat on a missing operand

**Exit code:** 1

```
no-such-file.txt: not found
missing file
```

## `vpt exit 3`

Nonzero exit codes are recorded in the snapshot.

**Exit code:** 3

```
```
