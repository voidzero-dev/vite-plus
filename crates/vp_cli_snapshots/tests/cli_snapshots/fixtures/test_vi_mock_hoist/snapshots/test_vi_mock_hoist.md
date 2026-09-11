# test_vi_mock_hoist

## `vp test run src/vi-mock-hoist.test.ts`

vi.mock() from 'vite-plus/test' must hoist via upstream mocker (no vite-plus patch/shim)

```

 RUN  <version> <workspace>

 ✓ src/vi-mock-hoist.test.ts (1 test) <duration>
   ✓ hoists vi.mock() above imports for the vite-plus/test specifier <duration>

 Test Files  1 passed (1)
      Tests  1 passed (1)
   Start at  <time>
   Duration  <duration> (transform <duration>, setup <duration>, import <duration>, tests <duration>, environment <duration>)
```
