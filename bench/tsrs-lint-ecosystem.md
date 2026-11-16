# Ecosystem correctness comparison

Local results from **2026-11-16** compare `tsrs@7.1.0-beta.tsrs2` with
`oxlint-tsgolint@7.0.2007` across **83 catalog forks plus vinext**.
All **740 commands** use the published PR #3352 preview at
`3345a7bf89b1f8c5e3a3afa6d852446151256835`. Only the lint backend changes within each pair.

No command changes from pass with Go to fail with tsrs in this sweep.
This does not mean every project passes: shared source, configuration, dependency,
and formatting failures remain visible in the complete table.

## Tested toolchain

| Component            | Version / source                                                                                                                                                       |
| -------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Published preview    | `0.0.0-commit.3345a7bf89b1f8c5e3a3afa6d852446151256835`                                                                                                                |
| Preview build        | [Build run](https://github.com/voidzero-dev/vite-plus/actions/runs/43871369416), [publication run](https://github.com/voidzero-dev/vite-plus/actions/runs/43875765725) |
| Oxlint / Oxfmt       | `1.93.0` / `0.78.0`                                                                                                                                                    |
| Go control           | `oxlint-tsgolint@7.0.2007`                                                                                                                                             |
| Default backend      | `tsrs@7.1.0-beta.tsrs2`                                                                                                                                                |
| TypeScript reference | `7.1.0-beta`, commit `3e9236361fa97a9e6c685f4833ac457ae2d8c50a`                                                                                                        |
| Machine              | Apple M4 Max, 48 GiB RAM, macOS `27.0.1`, ARM64                                                                                                                        |
| Node.js              | `24.22.2` or `26.13.0`, according to the project engine requirement                                                                                                    |
| Catalog              | [83 entries at `5a74ee011`](https://github.com/vite-plus-ecosystem-ci/.github/blob/5a74ee011971dfcb4545b08b740a4d713dc90290/ecosystem.json), plus vinext               |

Both controls use the same application commits, installed dependencies, generated
files, commands, and published frontend. The table pins every tested source.
The sweep runs two repository batches concurrently; elapsed times from this run
are not performance measurements. See the [separate benchmark](./tsrs-lint.md)
for sequential timing, memory, and disk measurements.

## npmx.dev and vinext

Vinext passes all four lint/check workloads under both backends.
npmx.dev passes default lint under both. Its explicit type-check workload reports
260 errors under each backend, and `vp check` stops on shared formatting issues.
Its declared `test:types` task also fails under both. Matching exit codes and
error counts do not establish identical diagnostics.

## Exit-status summary

| Workload                            | Both pass | Both fail | Go passes, tsrs fails | Go fails, tsrs passes |
| ----------------------------------- | --------: | --------: | --------------------: | --------------------: |
| `vp lint`                           |        51 |        33 |                     0 |                     0 |
| `vp lint --type-aware`              |        37 |        47 |                     0 |                     0 |
| `vp lint --type-aware --type-check` |        23 |        61 |                     0 |                     0 |
| `vp check`                          |        32 |        52 |                     0 |                     0 |
| Declared project type-check task    |        15 |        19 |                     0 |                     0 |

`Both fail` includes configuration, formatting, dependency, and source errors. It does not identify a backend regression.

## Complete fork results

Each exit-code pair is **Go / tsrs**: `0` means success; `1` or `2` means failure. All source links pin the tested commit.
The error column counts error-severity diagnostics in the explicit type-check workload; warnings are excluded. Equal counts do not establish identical diagnostics.
`—` means that no suitable declared type-check task was found. Tasks retain their own compiler selection; switching the lint backend does not replace `tsc`, `vue-tsc`, or `svelte-check`.

| Repository / commit                                                                                                                                              | Lint  | Type-aware lint | Explicit type check | Type-check errors | `vp check` | Project task / exits |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----- | --------------- | ------------------- | ----------------: | ---------- | -------------------- |
| [agentsview `4a9d932b2`](https://github.com/vite-plus-ecosystem-ci/agentsview/tree/4a9d932b2b41ee6ee8d30e215b99b258c61c3163)                                     | 0 / 0 | 0 / 0           | 0 / 0               |             0 / 0 | 0 / 0      | `check` 0 / 0        |
| [anywidget `35160ac08`](https://github.com/vite-plus-ecosystem-ci/anywidget/tree/35160ac08e75918faee008a3b8996ab2a0a16a46)                                       | 1 / 1 | 1 / 1           | 1 / 1               |           14 / 14 | 1 / 1      | —                    |
| [arrow-hero `e6dedbefb`](https://github.com/vite-plus-ecosystem-ci/arrow-hero/tree/e6dedbefb94f9e93b3a8c8d9479a94b2b0c0d45a)                                     | 0 / 0 | 1 / 1           | 1 / 1               |             6 / 6 | 0 / 0      | —                    |
| [better-ajv-errors `03ce620b1`](https://github.com/vite-plus-ecosystem-ci/better-ajv-errors/tree/03ce620b155357aa2968459bf754949b7e07e3e5)                       | 1 / 1 | 1 / 1           | 1 / 1               |             1 / 1 | 1 / 1      | —                    |
| [bippy `fbafc407c`](https://github.com/vite-plus-ecosystem-ci/bippy/tree/fbafc407ca31ad82f391404c6c6539008f9e0d79)                                               | 0 / 0 | 0 / 0           | 1 / 1               |         257 / 257 | 0 / 0      | `typecheck` 0 / 0    |
| [brazilian-utils-javascript `0f0f9a153`](https://github.com/vite-plus-ecosystem-ci/brazilian-utils-javascript/tree/0f0f9a1539d7263a08e245e9b03f8f7dffc7cd9e)     | 1 / 1 | 1 / 1           | 1 / 1               |    Config blocked | 1 / 1      | —                    |
| [cloudflare-cf `e2c720597`](https://github.com/vite-plus-ecosystem-ci/cloudflare-cf/tree/e2c720597887fb742e52cbe79132e4bdfec92f16)                               | 0 / 0 | 0 / 0           | 1 / 1               |           41 / 41 | 0 / 0      | `check:type` 1 / 1   |
| [cloudflare-os `1541cc07f`](https://github.com/vite-plus-ecosystem-ci/cloudflare-os/tree/1541cc07fc47c1beafeb98cd9927bb3f112d523f)                               | 0 / 0 | 1 / 1           | 1 / 1               |       8105 / 8120 | 0 / 0      | —                    |
| [codiff `2167e44b3`](https://github.com/vite-plus-ecosystem-ci/codiff/tree/2167e44b3a362480d8fca47a7fefcf98f8a24a27)                                             | 1 / 1 | 1 / 1           | 1 / 1               |           53 / 53 | 1 / 1      | —                    |
| [Cookies-EU-banner `1193c4c41`](https://github.com/vite-plus-ecosystem-ci/Cookies-EU-banner/tree/1193c4c41a91f26d722c680564bc9f0af05903ed)                       | 1 / 1 | 1 / 1           | 1 / 1               |             6 / 6 | 1 / 1      | —                    |
| [critical `03897c82e`](https://github.com/vite-plus-ecosystem-ci/critical/tree/03897c82e255416ca7104774a9d19142bd1e2744)                                         | 0 / 0 | 0 / 0           | 0 / 0               |             0 / 0 | 0 / 0      | —                    |
| [crust `f007d2d0d`](https://github.com/vite-plus-ecosystem-ci/crust/tree/f007d2d0d3d6ca030a39cb3d1d978c707115b01d)                                               | 1 / 1 | 1 / 1           | 1 / 1               |       1173 / 1173 | 1 / 1      | `check:types` 1 / 1  |
| [cs-demo-manager `164e48745`](https://github.com/vite-plus-ecosystem-ci/cs-demo-manager/tree/164e487451ed24478a02c6225f558cf0268efa65)                           | 1 / 1 | 1 / 1           | 1 / 1               |             3 / 3 | 1 / 1      | `compile` 0 / 0      |
| [css-modules-kit `1dec592b9`](https://github.com/vite-plus-ecosystem-ci/css-modules-kit/tree/1dec592b930c6b65cf15169dc85cbabdd44ef64f)                           | 0 / 0 | 0 / 0           | 0 / 0               |             0 / 0 | 0 / 0      | —                    |
| [delta-comic `d2b007579`](https://github.com/vite-plus-ecosystem-ci/delta-comic/tree/d2b007579b703f9d1561e4bd1b39cd7e7c17b6ab)                                   | 0 / 0 | 1 / 1           | 1 / 1               |         288 / 288 | 1 / 1      | `typecheck` 1 / 1    |
| [digital-signature `2d6e49c24`](https://github.com/vite-plus-ecosystem-ci/digital-signature/tree/2d6e49c24448fca9929bfa1973c29bca09bebdc8)                       | 0 / 0 | 0 / 0           | 1 / 1               |             1 / 1 | 0 / 0      | `check` 0 / 0        |
| [docx-editor `2457e2765`](https://github.com/vite-plus-ecosystem-ci/docx-editor/tree/2457e2765d76e231cff7c87a2b2993cb6071e668)                                   | 1 / 1 | 1 / 1           | 1 / 1               |       2580 / 2555 | 1 / 1      | `type-check` 2 / 2   |
| [dotabod-backend `65d80c3ef`](https://github.com/vite-plus-ecosystem-ci/dotabod-backend/tree/65d80c3ef8168150f5a32c8586ec136a2d877e01)                           | 1 / 1 | 1 / 1           | 1 / 1               |       5379 / 5381 | 1 / 1      | `typecheck` 1 / 1    |
| [dotabod-frontend `e06184067`](https://github.com/vite-plus-ecosystem-ci/dotabod-frontend/tree/e06184067abf0f97c8f74abf6e992638bc50b90d)                         | 1 / 1 | 1 / 1           | 1 / 1               |       6774 / 6774 | 1 / 1      | `typecheck` 0 / 0    |
| [electron-vite-template `4b1f4d64b`](https://github.com/vite-plus-ecosystem-ci/electron-vite-template/tree/4b1f4d64b36bab216391668efc24f12598553628)             | 0 / 0 | 0 / 0           | 1 / 1               |             7 / 7 | 1 / 1      | —                    |
| [elysia-clerk `6e42ab83b`](https://github.com/vite-plus-ecosystem-ci/elysia-clerk/tree/6e42ab83b4fd8401f5ddb13ae206cbf4887df469)                                 | 0 / 0 | 0 / 0           | 0 / 0               |             0 / 0 | 1 / 1      | —                    |
| [eslint-plugin-oxlint `4eab8b2d8`](https://github.com/vite-plus-ecosystem-ci/eslint-plugin-oxlint/tree/4eab8b2d87d80c508247209fde18e749160bf840)                 | 0 / 0 | 0 / 0           | 0 / 0               |             0 / 0 | 0 / 0      | —                    |
| [f3-app `b03939e5c`](https://github.com/vite-plus-ecosystem-ci/f3-app/tree/b03939e5c9ac280dcc96454a69c950dec9bcd243)                                             | 1 / 1 | 1 / 1           | 1 / 1               |             9 / 9 | 1 / 1      | —                    |
| [faasjs `813638437`](https://github.com/vite-plus-ecosystem-ci/faasjs/tree/813638437070cd8f30bec0b93d2c730903db6579)                                             | 1 / 1 | 1 / 1           | 1 / 1               |           78 / 78 | 1 / 1      | —                    |
| [fate `97a730445`](https://github.com/vite-plus-ecosystem-ci/fate/tree/97a73044585c748b5a23be8f363ba6a6bb606295)                                                 | 1 / 1 | 1 / 1           | 1 / 1               |         100 / 100 | 1 / 1      | —                    |
| [gitify `3a843faed`](https://github.com/vite-plus-ecosystem-ci/gitify/tree/3a843faedca74e64dfadb82cfc8355c0f58a22ed)                                             | 0 / 0 | 0 / 0           | 0 / 0               |             0 / 0 | 0 / 0      | —                    |
| [graph-explorer `7a696be53`](https://github.com/vite-plus-ecosystem-ci/graph-explorer/tree/7a696be53875c4241826b3e55ce1e0b4d721ce62)                             | 0 / 0 | 0 / 0           | 0 / 0               |             0 / 0 | 0 / 0      | —                    |
| [hono `61f7c59e5`](https://github.com/vite-plus-ecosystem-ci/hono/tree/61f7c59e50244ab4d2dd83c151928cf7ea448510)                                                 | 0 / 0 | 1 / 1           | 1 / 1               |         419 / 421 | 1 / 1      | —                    |
| [Ivy-Framework `f6b1d065f`](https://github.com/vite-plus-ecosystem-ci/Ivy-Framework/tree/f6b1d065f725d4fc27ec99b69e773750920f4145)                               | 0 / 0 | 0 / 0           | 1 / 1               |             1 / 1 | 0 / 0      | —                    |
| [kaneo `ea9eb6d3d`](https://github.com/vite-plus-ecosystem-ci/kaneo/tree/ea9eb6d3d66cb4b216ee9f4314f089ea331b224b)                                               | 0 / 0 | 1 / 1           | 1 / 1               |       1640 / 1640 | 0 / 0      | `typecheck` 1 / 1    |
| [local-studio `f65c8af78`](https://github.com/vite-plus-ecosystem-ci/local-studio/tree/f65c8af78582f741b62546c93d7c66439f0cd5b1)                                 | 1 / 1 | 1 / 1           | 1 / 1               |           34 / 34 | 1 / 1      | `typecheck` 0 / 0    |
| [matterviz `91ce15d7b`](https://github.com/vite-plus-ecosystem-ci/matterviz/tree/91ce15d7b032252ee0939d04148d09e881f1a8b4)                                       | 1 / 1 | 1 / 1           | 1 / 1               |           71 / 80 | 2 / 2      | `typecheck` 1 / 1    |
| [mcp-omnisearch `03bb264b8`](https://github.com/vite-plus-ecosystem-ci/mcp-omnisearch/tree/03bb264b8edd72dac96f45ffdfe582682eca96ee)                             | 0 / 0 | 0 / 0           | 0 / 0               |             0 / 0 | 0 / 0      | —                    |
| [media-processors `79c87cd5c`](https://github.com/vite-plus-ecosystem-ci/media-processors/tree/79c87cd5c066a0a9abea979044c38c69603a85e8)                         | 1 / 1 | 1 / 1           | 1 / 1               |           91 / 91 | 1 / 1      | `typecheck` 1 / 1    |
| [museeks `7163f6019`](https://github.com/vite-plus-ecosystem-ci/museeks/tree/7163f6019a3184043c06352caeec0ed1bf6ccff5)                                           | 1 / 1 | 1 / 1           | 1 / 1               |           73 / 73 | 1 / 1      | `test:typings` 2 / 2 |
| [nostalgist `436e23e4c`](https://github.com/vite-plus-ecosystem-ci/nostalgist/tree/436e23e4cac64e61e71825bcb1c001cf3728b736)                                     | 0 / 0 | 0 / 0           | 0 / 0               |             0 / 0 | 0 / 0      | —                    |
| [notero `011be2ca9`](https://github.com/vite-plus-ecosystem-ci/notero/tree/011be2ca9ddc6b34326f7e5ce75089266cbf94bc)                                             | 0 / 0 | 0 / 0           | 0 / 0               |             0 / 0 | 1 / 1      | `typecheck` 0 / 0    |
| [npmx.dev `ef6105e1e`](https://github.com/vite-plus-ecosystem-ci/npmx.dev/tree/ef6105e1e11d93b81f51c78c6ccb8b35fe8c19fe)                                         | 0 / 0 | 1 / 1           | 1 / 1               |         260 / 260 | 1 / 1      | `test:types` 2 / 2   |
| [nshipster.com `ccdd0b5dc`](https://github.com/vite-plus-ecosystem-ci/nshipster.com/tree/ccdd0b5dc49100bdb2fce2fbfbdd12cff4414d8f)                               | 1 / 1 | 1 / 1           | 1 / 1               |           36 / 36 | 1 / 1      | `check` 1 / 1        |
| [ohbug `261f2807c`](https://github.com/vite-plus-ecosystem-ci/ohbug/tree/261f2807cfe94475bc5094a75a5a2e874dafa715)                                               | 1 / 1 | 1 / 1           | 1 / 1               |           16 / 16 | 1 / 1      | —                    |
| [OneSignal-Website-SDK `836367e7f`](https://github.com/vite-plus-ecosystem-ci/OneSignal-Website-SDK/tree/836367e7fccf5420d4ef31cf750d21fdad76277a)               | 1 / 1 | 1 / 1           | 1 / 1               |             1 / 2 | 1 / 1      | —                    |
| [oxc-walker `d35924c04`](https://github.com/vite-plus-ecosystem-ci/oxc-walker/tree/d35924c04f3cbd48ae53ba20f28933b508cfd244)                                     | 0 / 0 | 0 / 0           | 0 / 0               |             0 / 0 | 0 / 0      | `test:types` 0 / 0   |
| [oxlint-tailwindcss `7477ab484`](https://github.com/vite-plus-ecosystem-ci/oxlint-tailwindcss/tree/7477ab4849330e46802e8753af2cbb6968c58f3d)                     | 0 / 0 | 0 / 0           | 1 / 1               |         704 / 704 | 1 / 1      | `typecheck` 0 / 0    |
| [pinia-shared-state `1b1ec6109`](https://github.com/vite-plus-ecosystem-ci/pinia-shared-state/tree/1b1ec6109b6f46079e4ddb5aeb9feff1d7188c24)                     | 0 / 0 | 0 / 0           | 0 / 0               |             0 / 0 | 0 / 0      | —                    |
| [pocket-id `8b5784206`](https://github.com/vite-plus-ecosystem-ci/pocket-id/tree/8b5784206b886f484caa98dd306f179b3a128b73)                                       | 1 / 1 | 1 / 1           | 1 / 1               |           19 / 19 | 2 / 2      | —                    |
| [raindrop-mcp `20161f2ef`](https://github.com/vite-plus-ecosystem-ci/raindrop-mcp/tree/20161f2efeb38311002e5bed0b0ba320800a02b3)                                 | 0 / 0 | 0 / 0           | 0 / 0               |             0 / 0 | 0 / 0      | `type-check` 0 / 0   |
| [ramen-api `9e31be0d3`](https://github.com/vite-plus-ecosystem-ci/ramen-api/tree/9e31be0d3410948f6ee086c068c692d087ee07c7)                                       | 0 / 0 | 0 / 0           | 0 / 0               |             0 / 0 | 0 / 0      | —                    |
| [react-data-grid `6f6469d88`](https://github.com/vite-plus-ecosystem-ci/react-data-grid/tree/6f6469d88dac7d7263862ef2d449b7b06d98e1f7)                           | 0 / 0 | 0 / 0           | 1 / 1               |         623 / 621 | 0 / 0      | `typecheck` 2 / 2    |
| [react-hookz-web `35944fbc4`](https://github.com/vite-plus-ecosystem-ci/react-hookz-web/tree/35944fbc4c8d5ab6c05a1c7f0b21a58b75d78fd6)                           | 1 / 1 | 1 / 1           | 1 / 1               |           22 / 22 | 1 / 1      | `typecheck` 0 / 0    |
| [react-native-onesignal `3fde3921d`](https://github.com/vite-plus-ecosystem-ci/react-native-onesignal/tree/3fde3921d22a030109987b615938ad8b53538586)             | 1 / 1 | 1 / 1           | 1 / 1               |             2 / 2 | 1 / 1      | —                    |
| [react-useanimations `f7af6f53f`](https://github.com/vite-plus-ecosystem-ci/react-useanimations/tree/f7af6f53f6f1a97486ad95756b281a34e9f7c670)                   | 0 / 0 | 0 / 0           | 0 / 0               |             0 / 0 | 0 / 0      | —                    |
| [redis-me `2db9d859b`](https://github.com/vite-plus-ecosystem-ci/redis-me/tree/2db9d859ba0a4c6fe2868c7774560f96f321ae90)                                         | 0 / 0 | 0 / 0           | 0 / 0               |             0 / 0 | 0 / 0      | —                    |
| [regle `c2668d6f0`](https://github.com/vite-plus-ecosystem-ci/regle/tree/c2668d6f02ec414bdf01609f323fbdbd6036c838)                                               | 0 / 0 | 1 / 1           | 1 / 1               |       1165 / 1166 | 1 / 1      | `typecheck` 1 / 1    |
| [remdx `4da4d8f94`](https://github.com/vite-plus-ecosystem-ci/remdx/tree/4da4d8f945cb63056efc9a5cb94b32bf2b0b2777)                                               | 0 / 0 | 0 / 0           | 0 / 0               |             0 / 0 | 1 / 1      | —                    |
| [saasyland.com `ca0fd03bf`](https://github.com/vite-plus-ecosystem-ci/saasyland.com/tree/ca0fd03bf6a754b224ff5c2bc75e3cc4132f1079)                               | 1 / 1 | 1 / 1           | 1 / 1               |         164 / 164 | 1 / 1      | —                    |
| [sandbox-sdk `61cac81b5`](https://github.com/vite-plus-ecosystem-ci/sandbox-sdk/tree/61cac81b53334b314e4a4daf04486b9cee9b65c2)                                   | 1 / 1 | 1 / 1           | 1 / 1               |           34 / 34 | 1 / 1      | —                    |
| [sigle `31767a4d8`](https://github.com/vite-plus-ecosystem-ci/sigle/tree/31767a4d8f16e8448f88c3ff9b3d68efb47541c8)                                               | 1 / 1 | 1 / 1           | 1 / 1               |    Config blocked | 1 / 1      | —                    |
| [solid-services `add4b036b`](https://github.com/vite-plus-ecosystem-ci/solid-services/tree/add4b036b72de7e6372b07faf0aed6b3f2e506d4)                             | 0 / 0 | 0 / 0           | 0 / 0               |             0 / 0 | 1 / 1      | —                    |
| [soybean-admin `d2dfefd75`](https://github.com/vite-plus-ecosystem-ci/soybean-admin/tree/d2dfefd757e37d8d6077ea1cea6181fb628d6097)                               | 0 / 0 | 1 / 1           | 1 / 1               |         117 / 117 | 0 / 0      | `typecheck` 0 / 0    |
| [soybean-admin-antd `02a002d12`](https://github.com/vite-plus-ecosystem-ci/soybean-admin-antd/tree/02a002d120a51bdc3c255b0c6582b00a8bcdddf0)                     | 0 / 0 | 1 / 1           | 1 / 1               |         130 / 130 | 1 / 1      | `typecheck` 0 / 0    |
| [spark-ui `0436591bc`](https://github.com/vite-plus-ecosystem-ci/spark-ui/tree/0436591bc916bd17e7c5df28c1f58eb55066a3cb)                                         | 1 / 1 | 1 / 1           | 1 / 1               |             6 / 4 | 1 / 1      | `typecheck` 1 / 1    |
| [svelte-meta-tags `57339bc3f`](https://github.com/vite-plus-ecosystem-ci/svelte-meta-tags/tree/57339bc3feefc1035914245dcbc37b08fb131680)                         | 0 / 0 | 1 / 1           | 1 / 1               |             4 / 4 | 2 / 2      | —                    |
| [svelte-multiselect `29a64aa5c`](https://github.com/vite-plus-ecosystem-ci/svelte-multiselect/tree/29a64aa5cd8e18867fee9b469882b1a0715f8b5a)                     | 1 / 1 | 1 / 1           | 1 / 1               |           81 / 84 | 2 / 2      | —                    |
| [sveltekit-embed `e7818c891`](https://github.com/vite-plus-ecosystem-ci/sveltekit-embed/tree/e7818c891e584070f5b00072e3bb85c070f71a6c)                           | 1 / 1 | 1 / 1           | 1 / 1               |             3 / 3 | 2 / 2      | —                    |
| [tiptap `228df820d`](https://github.com/vite-plus-ecosystem-ci/tiptap/tree/228df820d22a807310f7dcd47eb145557027c8e5)                                             | 0 / 0 | 1 / 1           | 1 / 1               |           57 / 57 | 0 / 0      | —                    |
| [trpc-nuxt `6b2b2ba86`](https://github.com/vite-plus-ecosystem-ci/trpc-nuxt/tree/6b2b2ba86e6b15adb7d5c88a785adba1f050cac5)                                       | 1 / 1 | 1 / 1           | 1 / 1               |           13 / 13 | 1 / 1      | —                    |
| [typed-xlsx `91ea9a725`](https://github.com/vite-plus-ecosystem-ci/typed-xlsx/tree/91ea9a725631f8d7ec8b6ff70d55fac14bf45dbb)                                     | 0 / 0 | 1 / 1           | 1 / 1               |           39 / 39 | 1 / 1      | `typecheck` 1 / 1    |
| [unplugin-turbo-console `ea1986d13`](https://github.com/vite-plus-ecosystem-ci/unplugin-turbo-console/tree/ea1986d1329a7c1a059e3ae0731384ecfbd9dba1)             | 0 / 0 | 1 / 1           | 1 / 1               |           41 / 41 | 0 / 0      | —                    |
| [urllib `7aba43082`](https://github.com/vite-plus-ecosystem-ci/urllib/tree/7aba43082d8fce3d967e1855ae326627a9ce139c)                                             | 0 / 0 | 0 / 0           | 0 / 0               |             0 / 0 | 0 / 0      | `typecheck` 0 / 0    |
| [v-offline `d0ed7de37`](https://github.com/vite-plus-ecosystem-ci/v-offline/tree/d0ed7de3707f5c43ed5f0b066e22f3347eea413e)                                       | 0 / 0 | 0 / 0           | 1 / 1               |           31 / 29 | 0 / 0      | `typecheck` 0 / 0    |
| [varlet `4f56984db`](https://github.com/vite-plus-ecosystem-ci/varlet/tree/4f56984dbf90036ebdea4167217ff7659f2e4c9f)                                             | 0 / 0 | 0 / 0           | 1 / 1               |         274 / 274 | 0 / 0      | `compile` 1 / 1      |
| [vean-ui `7c5493801`](https://github.com/vite-plus-ecosystem-ci/vean-ui/tree/7c5493801fe91cd2df33dec910bca3436d85a3df)                                           | 0 / 0 | 1 / 1           | 1 / 1               |       2610 / 2605 | 0 / 0      | `typecheck` 1 / 1    |
| [vibe-dashboard `9b0ec30a3`](https://github.com/vite-plus-ecosystem-ci/vibe-dashboard/tree/9b0ec30a39b9d77a44656a7cdb520c15c224b3e1)                             | 1 / 1 | 1 / 1           | 1 / 1               |             1 / 1 | 1 / 1      | —                    |
| [vinext `42d940ece`](https://github.com/vite-plus-ecosystem-ci/vinext/tree/42d940eceb8f8199195d374711a14498b248dd61)                                             | 0 / 0 | 0 / 0           | 0 / 0               |             0 / 0 | 0 / 0      | —                    |
| [vitepress-carbon `b9c0ecf06`](https://github.com/vite-plus-ecosystem-ci/vitepress-carbon/tree/b9c0ecf0685603ee7fae0776b479724539818f89)                         | 0 / 0 | 0 / 0           | 0 / 0               |             0 / 0 | 1 / 1      | —                    |
| [vitepress-plugin-group-icons `602761b05`](https://github.com/vite-plus-ecosystem-ci/vitepress-plugin-group-icons/tree/602761b05108e4b4df2da9f69e08a406cd6540f0) | 0 / 0 | 0 / 0           | 1 / 1               |             3 / 3 | 0 / 0      | —                    |
| [VRM-Addon-for-Blender `f886e7fe2`](https://github.com/vite-plus-ecosystem-ci/VRM-Addon-for-Blender/tree/f886e7fe283f08035273f5406fadebea6db64d60)               | 0 / 0 | 0 / 0           | 1 / 1               |             6 / 6 | 0 / 0      | —                    |
| [vue-clamp `6f0da3991`](https://github.com/vite-plus-ecosystem-ci/vue-clamp/tree/6f0da3991f804587ba402a7ca22aec4564eab938)                                       | 0 / 0 | 0 / 0           | 0 / 0               |             0 / 0 | 1 / 1      | —                    |
| [vue-core `8f946dc4a`](https://github.com/vite-plus-ecosystem-ci/vue-core/tree/8f946dc4a2f0bb515a1a8070388b9b865a2a2b9d)                                         | 0 / 0 | 0 / 0           | 1 / 1               |         541 / 548 | 1 / 1      | `check` 2 / 2        |
| [vue-devtools `bb6f283f9`](https://github.com/vite-plus-ecosystem-ci/vue-devtools/tree/bb6f283f9eab90bbfbe06376ac92fcc116d43b52)                                 | 1 / 1 | 1 / 1           | 1 / 1               |           70 / 70 | 1 / 1      | `typecheck` 2 / 2    |
| [vueleaflet `254f84e5f`](https://github.com/vite-plus-ecosystem-ci/vueleaflet/tree/254f84e5f8660e914ecbd1f8a0d0763709edf514)                                     | 0 / 0 | 0 / 0           | 1 / 1               |           18 / 18 | 1 / 1      | —                    |
| [web-app-template `7cc5e4e15`](https://github.com/vite-plus-ecosystem-ci/web-app-template/tree/7cc5e4e152429100f8dc230b2fbe9e516f6a1c8c)                         | 1 / 1 | 1 / 1           | 1 / 1               |             1 / 1 | 1 / 1      | —                    |
| [yaak `f7402f5f4`](https://github.com/vite-plus-ecosystem-ci/yaak/tree/f7402f5f4f9752c4722a478cdb51dd54ad87cc34)                                                 | 0 / 0 | 0 / 0           | 1 / 1               |         327 / 327 | 1 / 1      | —                    |
| [yielded-auth `788239010`](https://github.com/vite-plus-ecosystem-ci/yielded-auth/tree/788239010e5f806d28f7fb3151e9b37c1e59a39c)                                 | 0 / 0 | 0 / 0           | 0 / 0               |             0 / 0 | 1 / 1      | —                    |

## Diagnostic changes in already-failing projects

The following projects already fail explicit type checking with Go and have additional error diagnostics with tsrs. Counts compare file, code, severity, first-label position, and message; a changed message or position can count as one addition and one removal. These differences have not all been independently attributed to the upstream compiler.

| Repository            | Added errors | Removed errors | Added diagnostic codes                                                                                                                                                    |
| --------------------- | -----------: | -------------: | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| cloudflare-os         |          266 |            251 | `TS2339` × 22, `TS2345` × 2, `TS2550` × 38, `TS2740` × 2, `TS5115` × 3, `TS7031` × 8, `TS7053` × 1, `no-unnecessary-type-assertion` × 1, `no-unsafe-type-assertion` × 189 |
| delta-comic           |            1 |              1 | `TS2345` × 1                                                                                                                                                              |
| docx-editor           |           31 |             56 | `TS2322` × 7, `TS2339` × 2, `TS2345` × 11, `TS2352` × 1, `TS2551` × 2, `TS2741` × 2, `TS7031` × 4, `TS7053` × 2                                                           |
| dotabod-backend       |            4 |              2 | `TS2345` × 1, `TS5115` × 2, `no-unsafe-type-assertion` × 1                                                                                                                |
| dotabod-frontend      |            8 |              8 | `no-unsafe-argument` × 5, `no-unsafe-type-assertion` × 3                                                                                                                  |
| faasjs                |            1 |              1 | `TS2322` × 1                                                                                                                                                              |
| fate                  |            1 |              1 | `TS2345` × 1                                                                                                                                                              |
| hono                  |            5 |              3 | `TS2875` × 3, `TS7031` × 2                                                                                                                                                |
| kaneo                 |           19 |             19 | `TS2322` × 10, `TS2339` × 4, `TS2345` × 4, `TS2352` × 1                                                                                                                   |
| local-studio          |            1 |              1 | `TS2741` × 1                                                                                                                                                              |
| matterviz             |            9 |              0 | `TS7031` × 9                                                                                                                                                              |
| OneSignal-Website-SDK |            1 |              0 | `TS2741` × 1                                                                                                                                                              |
| regle                 |            1 |              0 | `TS2344` × 1                                                                                                                                                              |
| svelte-multiselect    |            3 |              0 | `TS18046` × 3                                                                                                                                                             |
| tiptap                |            3 |              3 | `TS2339` × 2, `TS2741` × 1                                                                                                                                                |
| varlet                |            1 |              1 | `TS2322` × 1                                                                                                                                                              |
| vean-ui               |            8 |             13 | `TS2322` × 1, `no-unsafe-type-assertion` × 7                                                                                                                              |
| vue-core              |            9 |              2 | `TS2322` × 2, `TS2353` × 1, `TS5115` × 1, `TS7006` × 2, `TS7031` × 3                                                                                                      |
| yaak                  |            1 |              1 | `TS2345` × 1                                                                                                                                                              |

## Diagnostic-scope fix: 2026-11-16

This release includes [tsrs #998](https://github.com/oxc-project/tsrs/pull/998).
Compiler diagnostics respect lint paths and ignore patterns. Other TypeScript
program files still provide type information.

The six [minimal reproduction](https://github.com/why-reproductions-are-required/tsrs-lint-diagnostic-scope)
scenarios were checked with Go and tsrs, including a separate `TSRS_FAST=0` control.
All **18 invocations** matched the expected results; both tsrs modes match Go.

| Configuration                                                | Go       | tsrs, both modes |
| ------------------------------------------------------------ | -------- | ---------------- |
| Erroneous fixture selected for linting                       | `TS2322` | `TS2322`         |
| Fixture ignored by lint, included in the TS program          | Pass     | Pass             |
| Same lint ignore, type checking disabled                     | Pass     | Pass             |
| Also excluded by `tsconfig.json`, without imports            | Pass     | Pass             |
| Excluded by `tsconfig.json`, but imported by selected source | Pass     | Pass             |
| CLI targets only `src`; fixture remains in the TS program    | Pass     | Pass             |

The selected-file control checks two files and reports its expected error.
Every other scenario checks one selected file. Type checking remains active
except in the explicitly disabled control.
Compiler-version differences are separate from file-selection behavior.

## Method and coverage limits

Each backend runs the following workloads. Lint commands also use
`--format json --threads 4` for structured comparison:

```sh
vp lint
vp lint --type-aware
vp lint --type-aware --type-check
vp check
```

Available declared type-check or compile tasks run through `vp run --no-cache` in 34 repositories.
Compiler build-info files outside `node_modules` are cleared before each task
control and restored afterwards. These tasks retain their own compiler.
Type-aware lint can also type-check when the project already enables `typeCheck`.

Backend launchers or native processes were observed in both explicit type-check controls for 82 repositories.
Both native processes were sampled for 78 repositories; short processes can escape sampling.
Configuration blocks a comparable explicit type-check result in: `brazilian-utils-javascript`, `sigle`.

- `brazilian-utils-javascript` imports `vite-plus/test/browser-webdriverio`, which the tested preview does not export.
- `sigle` requests the `effecttsgo` plugin, which both unpatched backends reject. Its patched Effect integration is outside this comparison.

Formatting can stop `vp check` before lint. Independent lint workloads provide
coverage for those projects. No retained command times out or exits from a signal.
All source commits, tracked files, and lockfile hashes remain unchanged.

The setup preserves application-specific inputs:

- Existing lockfiles and installed dependency trees are reused. Required generators run before comparison.
- The preview is linked into workspace and pnpm peer importers; temporary links are restored afterwards.
- Notero retains `@types/node@25.6.0`; nostalgist retains `@types/chai@5.2.3`.
- Dotabod projects use temporary Vite wrappers around their standalone lint/format configs. They and oxlint-tailwindcss receive temporary `vite-plus` declarations.
- Hono keeps its six lint scopes. Nostalgist targets `src tests`; docx-editor and digital-signature retain their configured source scopes.
- Dotabod-frontend disables nested config and ignores `supabase/functions/**`. Vinext uses sorted checker scheduling under both controls.

This sweep covers lint and type-check commands on macOS ARM64. It does not cover
every application build or test. Raw output, process traces, dependency receipts,
and diagnostic comparisons remain outside the repository.

## pnpm installation compatibility

`tsrs@7.1.0-beta.tsrs2` still fails installation with pnpm `12.21.0` and
`trustPolicy: no-downgrade`. Installation succeeds with the exact reviewed
`tsrs@7.1.0-beta.tsrs2` exception. Migration preserves the policy and adds only
this version to `trustPolicyExclude`. The [publishing-history issue](https://github.com/oxc-project/tsrs/issues/945)
is separate from compiler diagnostics.

## Ecosystem CI

The [PR-head run](https://github.com/voidzero-dev/vite-plus/actions/runs/43871369444)
passes all **51 project/platform jobs across 31 repositories** at `3345a7bf8`.
Its Linux/Windows matrix uses separate inputs and workloads from the local fork sweep.
Both runs use the current tsrs release.
