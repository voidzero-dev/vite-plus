# test_inline_snapshot_indent

## `vp test run -u src/inline-snapshot.test.ts`

write inline snapshot via --update (regression test for #1553)

```

 RUN  <version> <workspace>

 ✓ src/inline-snapshot.test.ts (1 test) <duration>
   ✓ inline snapshot indentation (1)
     ✓ writes multiline snapshots using the surrounding file indentation style <duration>

  Snapshots  1 written
 Test Files  1 passed (1)
      Tests  1 passed (1)
   Start at  <time>
   Duration  <duration> (transform <duration>, setup <duration>, import <duration>, tests <duration>, environment <duration>)
```

## `vpt print-file src/inline-snapshot.test.ts`

snapshot must use 2-space indentation, not tabs

```
import { describe, expect, it } from 'vite-plus/test';

describe('inline snapshot indentation', () => {
  it('writes multiline snapshots using the surrounding file indentation style', () => {
    expect('alpha\nbeta').toMatchInlineSnapshot(`
      "alpha
      beta"
    `);
  });
});
```
