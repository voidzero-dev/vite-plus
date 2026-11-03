# jest_dom_browser_type_order

Known upstream type conflict: browser-first declarations reject valid Node jest-dom assertions. Check upstream imports without Vite+ installed, plus the opposite-order control; this is not a passed release gate.

## `vp install --ignore-scripts`


## `node verify.mjs`

```
vitest: browser-first=true, Node jest-dom type errors=2
vitest: browser-first=false, Node jest-dom type errors=0
Release blocker reproduced: browser and jest-dom declaration order changes Node matcher types
```
