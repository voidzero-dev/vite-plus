# fmt_explicit_config

## `vp fmt --config custom-fmt.json index.js`

an explicit config overrides the inline format options

```
Finished in <duration> on 1 files using <n> threads.
```

## `vpt print-file index.js`

```
console.log('hello');
```

## `vp fmt --check -c custom-fmt.json index.js`

```
Checking formatting...

All matched files use the correct format.
Finished in <duration> on 1 files using <n> threads.
```

## `vp fmt --check --config=custom-fmt.json index.js`

```
Checking formatting...

All matched files use the correct format.
Finished in <duration> on 1 files using <n> threads.
```

## `vp fmt --check -c=custom-fmt.json index.js`

```
Checking formatting...

All matched files use the correct format.
Finished in <duration> on 1 files using <n> threads.
```

## `vp fmt --check -ccustom-fmt.json index.js`

```
Checking formatting...

All matched files use the correct format.
Finished in <duration> on 1 files using <n> threads.
```

## `vp fmt index.js`

use inline format options when no config flag is supplied

```
Finished in <duration> on 1 files using <n> threads.
```

## `vpt print-file index.js`

```
console.log("hello");
```

## `vpt write-file vite.config.ts 'throw new Error('\''Vite config must not be loaded'\'');
'`


## `vp fmt --config custom-fmt.json index.js`

explicit format configs do not require a working Vite config

```
Finished in <duration> on 1 files using <n> threads.
```

## `vpt print-file index.js`

```
console.log('hello');
```
