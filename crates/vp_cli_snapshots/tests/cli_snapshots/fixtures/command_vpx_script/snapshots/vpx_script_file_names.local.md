# vpx_script_file_names

File names with # and % run the named file, even when the name cut at the # or decoded at the % also exists.

## `vpx ./main#hash.ts`

./main also exists

```
ran main#hash.ts
```

## `vpx ./main%41.ts`

./mainA.ts also exists

```
ran main%41.ts
```

## `vpx ./main%file.ts`

Not a valid percent escape

```
ran main%file.ts
```

## `vpx ./hash#dir/main.ts`

A # in a directory name

```
ran hash#dir/main.ts
```

## `vpx ./fragment.ts`

In an import, # starts a URL fragment, even when a#b.ts exists

```
loaded a.ts: in an import, #b.ts is a fragment
```
