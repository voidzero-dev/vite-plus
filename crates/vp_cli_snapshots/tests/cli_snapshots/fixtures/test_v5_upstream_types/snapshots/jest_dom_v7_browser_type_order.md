# jest_dom_v7_browser_type_order

Check whether the current jest-dom release resolves the upstream declaration-order conflict.

## `vpt json-edit package.json devDependencies.@testing-library/jest-dom 7.0.1`


## `vpt json-edit package.json devDependencies.@testing-library/dom 10.4.1`


## `vp install --ignore-scripts`


## `node verify.mjs`

```
vitest: browser-first=true, Node jest-dom type errors=2
vitest: browser-first=false, Node jest-dom type errors=0
Release blocker reproduced: browser and jest-dom declaration order changes Node matcher types
```
