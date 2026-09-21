# github_actions_oidc_is_untracked

## `ACTIONS_ID_TOKEN_REQUEST_URL=https://example.invalid/oidc ACTIONS_ID_TOKEN_REQUEST_TOKEN=test-token-1 vp run oidc`

```
VITE+ - The Unified Toolchain for the Web

$ node check-oidc.mjs
OIDC request variables are available
```

## `ACTIONS_ID_TOKEN_REQUEST_URL=https://example.invalid/another-oidc ACTIONS_ID_TOKEN_REQUEST_TOKEN=test-token-2 vp run oidc`

Changing OIDC credentials preserves the cache hit

```
VITE+ - The Unified Toolchain for the Web

$ node check-oidc.mjs ◉ cache hit, replaying
OIDC request variables are available

---
vp run: cache hit, <duration> saved.
```
