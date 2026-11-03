# @voidzero-dev/vite-plus-prompts

Terminal prompts shared by Vite+ and tools that use its visual identity.

```ts
import { isCancel, log, note, select } from '@voidzero-dev/vite-plus-prompts';

const framework = await select({
  message: 'Choose a framework:',
  options: [
    { value: 'react', label: 'React', hint: 'TSX Pages' },
    { value: 'vue', label: 'Vue', hint: 'Vue SFC Pages' },
  ],
});

if (!isCancel(framework)) {
  log.success(`Selected ${framework}`);
  note('Run vp dev to start developing.', 'Next steps');
}
```

Blue `›` pointers identify focused choices and active prompts. Completed steps
and rounded box borders use gray; warnings use yellow and errors use red.
Inactive choices are dimmed, and descriptive hints appear on the focused choice.
Logs retain severity symbols without guide bars and use compact spacing.
Notes have complete borders and readable content by default.

Guides are off by default. Pass `withGuide: true` for one call, or use
`updateSettings({ withGuide: true })` to enable them globally. This controls the
prompt gutter, not box borders. Logs accept `spacing` for explicit blank rows;
notes accept `format` and boxes accept `formatBorder` and `rounded`.

Select and multiselect prompts show compact keyboard instructions. Pass
`showInstructions: false` to hide them. Text, password, path, and autocomplete
validation supports synchronous or asynchronous functions and Standard Schema.
Passwords accept a placeholder and always mask entered values. Path prompts
support directory navigation and Tab completion; general autocomplete can opt
in with `completeOnTab: true`.

`accessible: true`, `updateSettings({ accessible: true })`, or `ACCESSIBLE=1`
enables static prompt output for screen readers. Spinners use static output in
accessible mode, CI, and non-TTY streams. Their `pause()` and `resume()` methods
preserve elapsed time while excluding pauses. `CANCEL_SYMBOL` is exported for
callers that need the exact cancellation sentinel; `isCancel()` narrows results.

The interaction engine is `@clack/core` 1.5.1 or newer. Rendering and the shared
visual defaults are maintained in this package, so consumers can use the same
appearance without implementing their own prompt renderers.

## Verification

From the repository root:

```sh
vp check packages/prompts/src
vp test run packages/prompts/src/__tests__
pnpm --filter @voidzero-dev/vite-plus-prompts build
```
