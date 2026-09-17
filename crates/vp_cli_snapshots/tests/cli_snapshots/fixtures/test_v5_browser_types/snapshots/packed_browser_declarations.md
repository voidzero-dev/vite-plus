# packed_browser_declarations

Packed browser declarations must preserve DOM matchers, provider augmentations, and role types without importing all aliases together.

## `vp install --ignore-scripts`


## `node verify.mjs`

```
Standalone browser matcher declarations compile
browser: provider augmentations and role types preserved
context: provider augmentations and role types preserved
browser/context: provider augmentations and role types preserved
plugins/browser-context: provider augmentations and role types preserved
browser-playwright/context: provider augmentations and role types preserved
browser-preview/context: provider augmentations and role types preserved
browser-webdriverio/context: provider augmentations and role types preserved
browser/providers/playwright/context: provider augmentations and role types preserved
browser/providers/preview/context: provider augmentations and role types preserved
browser/providers/webdriverio/context: provider augmentations and role types preserved
```
