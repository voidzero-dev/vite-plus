# Tool-Reported Caching

Tool-reported caching lets a tool report cache metadata to Vite Task while it runs. The tool can report inputs, outputs, and environment variables that affect its result.

Use this page when a tool can report cache behavior that task config should not duplicate.

## Current Support

Vite+ supports tool-reported caching for `vp build` today.

When a task runs `vp build`, Vite reports build cache metadata to Vite Task. For a standard Vite build, you do not need to add these entries yourself:

- `env: ['VITE_*']` or `env: ['NODE_ENV']`
- `output: ['dist/**']`
- explicit input globs that replace automatic input tracking

Define the task through `vp run`:

```ts [vite.config.ts]
import { defineConfig } from 'vite-plus';

export default defineConfig({
  run: {
    tasks: {
      build: 'vp build',
    },
  },
});
```

Then run it with:

```bash
vp run build
```

## When To Add Manual Config

Tool-reported caching does not remove manual config. Add [`input`](/config/run#input), [`output`](/config/run#output), or [`env`](/config/run#env) when your project has behavior the tool cannot know.

Common cases:

- Add lockfiles or generated metadata that should affect [CI cache reuse](/guide/github-actions-cache).
- Exclude generated output directories from automatic inputs.
- Add custom environment variables that your own build wrapper reads.

For example, a CI build can keep automatic inputs and add the lockfile:

```ts [vite.config.ts]
build: {
  command: 'vp build',
  input: [{ auto: true }, 'pnpm-lock.yaml', '!dist', '!dist/**'],
  output: ['dist/**'],
}
```

## Environment Variables

For `vp build`, Vite reports the Vite environment variables that affect the build. Do not add `VITE_*` or `NODE_ENV` to [`env`](/config/run#env) or [`untrackedEnv`](/config/run#untrackedenv) for a standard Vite build.

For other commands, use `env` when a variable changes the result, and use `untrackedEnv` only when the task needs the variable but the value does not affect cache behavior.

## Third-Party Tools

Third-party tools can report cache metadata with [`@voidzero-dev/vite-task-client`](https://npmx.dev/package/@voidzero-dev/vite-task-client).

Tool-reported caching works with Vite Task's [automatic file tracking](/guide/cache#automatic-file-tracking). Vite Task still observes file reads and writes, and the tool report adds metadata the tool knows at runtime.

## Future Support

Vite+ will add tool-reported caching to more first-party tools as those integrations are built.

Until a tool supports tool-reported caching, configure cache behavior with [`input`](/config/run#input), [`output`](/config/run#output), and [`env`](/config/run#env).
