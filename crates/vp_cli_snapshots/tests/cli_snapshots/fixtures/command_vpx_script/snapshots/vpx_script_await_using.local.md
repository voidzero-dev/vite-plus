# vpx_script_await_using

Node.js 22.18.0 exercises asynchronous loader hooks and lowered await using; Node.js 24 exercises synchronous hooks and native await using. Both reject require() of the asynchronous module.

## `vpx ./src/await-using.mts`

Node.js 22 disposes the resource after the module body runs

```
resource active
resource disposed
```

## `vpx ./src/require-await-using.cts`

Node.js 22 rejects require() of the lowered asynchronous module

```
require() of await using: ERR_REQUIRE_ASYNC_MODULE
```

## `VP_NODE_VERSION=24.22.1 vpx ./src/await-using.mts`

Node.js 24 runs await using natively and produces the same disposal output

```
resource active
resource disposed
```

## `VP_NODE_VERSION=24.22.1 vpx ./src/require-await-using.cts`

Node.js 24 rejects require() when the only top-level await is an await using declaration

```
require() of await using: ERR_REQUIRE_ASYNC_MODULE
```
