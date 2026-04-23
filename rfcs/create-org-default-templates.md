# RFC: Organization Default Templates for `vp create`

> Status: Draft (Round 4) — adds support for bundled subdirectory templates
> inside `@org/create` itself (`./templates/foo` paths), so orgs can ship
> one package containing N templates instead of publishing N packages.
> See the "Resolved Decisions" section at the bottom for the settled list.

## Summary

Give organizations a single, branded entry point into their curated set of
project templates through `vp create @org`. When `@org/create` publishes a
`vp.templates` manifest in its `package.json`, Vite+ renders an interactive
picker over the listed templates; when it doesn't, the command executes
`@org/create` as a normal template (current behavior). A
`create.defaultTemplate` option in `vite.config.ts` lets a repo promote an
org's picker to the default for a bare `vp create`.

## Background

Organizations often maintain a collection of internal project templates
(web app, mobile app, server, library, etc.) and need a first-class way to
expose them as a single, branded entry point — so that engineers can pick
from an interactive list of "web / mobile / server / library" style choices
without having to remember individual per-template package names.

Reference:

- [RFC: Vite+ Code Generator](./code-generator.md) — the parent RFC that
  establishes `vp create` as a dual-mode (bingo + universal `create-*`) tool.
  This RFC is a consumer-facing extension on top of the existing universal
  `create-*` mode.
- [npm `create-*` convention](https://docs.npmjs.com/cli/v10/commands/npm-init)
  — the ecosystem convention `vp create` already honors via
  `expandCreateShorthand` (`packages/cli/src/create/discovery.ts:148-216`).

## Motivation

### The problem

Companies that own a portfolio of internal templates (web apps, libraries,
service scaffolds, CLI tools) have no clean way to present those templates as a
single product surface to their engineers. Today, to pick one of an org's
four templates, an engineer has to:

1. Know the exact package name of the template they want.
2. Type the full command: `vp create @nkzw/create-web`,
   `vp create @nkzw/create-mobile`, etc.
3. Find these names in a README, a wiki, or Slack.

This works, but it isn't discoverable, and it forces the org to document
package names in a medium that ages badly. The industry convention for
frameworks (Vite, Next, Nuxt) is "one command per framework" precisely because
a single memorable entry point outperforms a list of names.

### What engineers should be able to type

```bash
# Interactively pick a template from the @nkzw org
vp create @nkzw

# Pick a specific one directly
vp create @nkzw/web

# Inside a repo that sets @nkzw as the default:
vp create
```

The goal is that "the company's scaffolding toolchain" is spelled `@org`, not a
twelve-line README.

### Why not just document better READMEs?

READMEs can list templates, but:

- They don't power an interactive picker.
- They rot faster than code does.
- They can't be a project-level default that every clone of a repo inherits.

A manifest inside `@org/create`'s own `package.json` gives the org a single
source of truth, discoverable via `npm view`, versioned alongside the package.

## Existing Behavior (What Already Works)

This RFC is additive. A non-trivial amount of the feature already ships.

`packages/cli/src/create/discovery.ts:148-216` defines
`expandCreateShorthand`, which maps:

- `@org` → `@org/create`
- `@org/name` → `@org/create-name`
- `name` → `create-name` (with special cases for `nitro`, `svelte`,
  `@tanstack/start`)

So the following already works today:

```bash
# Already works: runs @nkzw/create
vp create @nkzw

# Already works: runs @nkzw/create-web
vp create @nkzw/web
```

The piece that doesn't exist yet is **discovering and choosing between multiple
templates owned by the same org**. That is what this RFC specifies.

## Proposed Solution

### High-level flow

1. User runs `vp create @org`.
2. `expandCreateShorthand` maps this to `@org/create` (unchanged).
3. Before dispatching to the template runner, `vp create` reads
   `@org/create`'s `package.json` from the npm registry.
4. If the `package.json` contains a `vp.templates` field, Vite+ renders an
   interactive picker over those entries.
5. After the user picks (or passes `@org/<name>` directly), Vite+ resolves the
   selected entry's `template` field through the existing `discoverTemplate`
   pipeline — which supports npm, GitHub, builtin `vite:*`, and local
   workspace packages.
6. If `vp.templates` is **absent**, Vite+ falls through to today's behavior
   and executes `@org/create` as a normal template. This keeps the feature
   zero-risk for org owners who haven't opted in.

### Command matrix

| Command                          | Manifest present? | Behavior                                                        |
| -------------------------------- | ----------------- | --------------------------------------------------------------- |
| `vp create @org`                 | yes               | Fetch manifest → picker → run chosen template                   |
| `vp create @org`                 | no                | Run `@org/create` as today (unchanged)                          |
| `vp create @org/name`            | yes, has `name`   | Run manifest entry `name` (manifest wins)                       |
| `vp create @org/name`            | yes, no `name`    | Fall back to `@org/create-name` shorthand                       |
| `vp create @org/name`            | no                | Run `@org/create-name` as today (unchanged)                     |
| `vp create` (in configured repo) | yes               | Same as `vp create @org` where `@org` is the configured default |
| `vp create <anything-else>`      | n/a               | Unchanged                                                       |

## Manifest Schema

The manifest lives at `vp.templates` in `@org/create`'s `package.json`.

```json
{
  "name": "@nkzw/create",
  "version": "1.0.0",
  "description": "Project templates from the @nkzw org",
  "vp": {
    "templates": [
      {
        "name": "monorepo",
        "description": "Full Nakazawa Tech monorepo scaffold",
        "template": "@nkzw/template-monorepo",
        "monorepo": true
      },
      {
        "name": "web",
        "description": "Web app template (Vite + React)",
        "template": "@nkzw/template-web",
        "keywords": ["web", "react", "app"]
      },
      {
        "name": "mobile",
        "description": "Mobile app (React Native) template",
        "template": "@nkzw/template-mobile"
      },
      {
        "name": "server",
        "description": "Server template (Node + Fastify)",
        "template": "github:nkzw-tech/template-server"
      },
      {
        "name": "library",
        "description": "TypeScript library template",
        "template": "@nkzw/template-library"
      },
      {
        "name": "demo",
        "description": "Bundled demo template (lives inside @nkzw/create)",
        "template": "./templates/demo"
      }
    ]
  }
}
```

### Field reference

| Field                        | Type              | Required | Notes                                                                                                                                                                                                                                                                                                                                                                              |
| ---------------------------- | ----------------- | -------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `vp.templates`               | `TemplateEntry[]` | yes      | Non-empty array. Empty arrays are treated as "no manifest" (fall through to `@org/create` run).                                                                                                                                                                                                                                                                                    |
| `vp.templates[].name`        | `string`          | yes      | Kebab-case. Used for `vp create @org/<name>` direct selection. Must be unique within the array.                                                                                                                                                                                                                                                                                    |
| `vp.templates[].description` | `string`          | yes      | One-line description shown in the picker.                                                                                                                                                                                                                                                                                                                                          |
| `vp.templates[].template`    | `string`          | yes      | One of: (a) an npm package specifier (`@nkzw/template-web`, optionally `@version`), (b) a GitHub URL (`github:user/repo`, `https://github.com/...`), (c) a `vite:*` builtin, (d) a local workspace package name, or (e) a relative path (`./templates/demo`, `../foo`) that resolves against the enclosing `@org/create` package root. See "Bundled subdirectory templates" below. |
| `vp.templates[].keywords`    | `string[]`        | no       | Filter terms for picker search.                                                                                                                                                                                                                                                                                                                                                    |
| `vp.templates[].monorepo`    | `boolean`         | no       | If `true`, marks this entry as a _monorepo-creating_ template. Hidden from the picker when `vp create` is invoked inside an existing monorepo. Mirrors the built-in behavior that filters `vite:monorepo` out of `getInitialTemplateOptions` (`packages/cli/src/create/initial-template-options.ts:9-31`). Defaults to `false`.                                                    |

### Invalid manifests

A present-but-invalid `vp.templates` field should **not** silently fall through
to the shorthand. It should produce a schema error with the offending field
path (e.g. `@nkzw/create: vp.templates[2].template is required`), because the
maintainer clearly intended to provide a manifest and should be told what's
wrong.

### Namespacing under `vp`

Using the `vp` object — rather than a top-level `vpTemplates` — keeps room for
future Vite+ package metadata without polluting the `package.json` root.
Conventions like `engines`, `bin`, and `files` already live in top-level
slots; tool-specific metadata is usually nested (cf. `jest`, `eslint`,
`prettier`).

### Bundled subdirectory templates

A very common real-world pattern — used by `create-vite`, `create-next-app`,
and many enterprise scaffolding kits — is a single package that contains
_all_ of its templates as subdirectories. For this pattern, a manifest entry
can use a relative path as the `template` value:

```json
{
  "name": "demo",
  "description": "Bundled demo template",
  "template": "./templates/demo"
}
```

Semantics:

- Paths starting with `./` or `../` resolve against the enclosing
  `@org/create` package root (the directory containing the published
  `package.json` — **not** the user's current working directory).
- The path must stay inside the package. Escapes via `../../..` that would
  reach outside the extracted tarball are rejected at schema-validation
  time.
- The referenced directory is scaffolded verbatim: file contents are copied
  to the target directory with no template-engine processing. (Variable
  substitution, Bingo-style transforms, etc. remain the domain of the
  `@org/template-*` or `bingo-template` branches.)
- Files like `package.json` inside the template subdirectory are used
  as-is. Org maintainers can pre-rewrite the package name at scaffold time
  via the existing `vp create` post-processing (name prompt, package-
  manager detection, etc.), matching today's builtin behavior.

**Why bundled paths matter for adoption**: without this, orgs with three or
four templates have to publish three or four packages, maintain their
independent release cadence, and document the mapping. With bundled paths,
a single `@org/create` package — containing the manifest and the templates
themselves — is the entire on-disk surface they need to ship.

**Tarball fetch and extract**: when `vp create` resolves a bundled path, it
fetches the tarball URL from the registry JSON it already pulled for the
manifest (`dist.tarball`), downloads it directly over HTTPS (honoring
`NPM_CONFIG_REGISTRY`), and extracts it to a per-version cache under
`$VP_HOME/tmp/create-org/<scope>/<name>/<version>/`. Subsequent invocations
reuse the cached extraction. A small tar-reader implementation (no external
install step, no spawning `npm pack`) keeps resolution fast and independent
of the user's package manager.

## Resolution Flow (implementation shape)

Hook point: inside `discoverTemplate`
(`packages/cli/src/create/discovery.ts:44-128`), immediately before the final
`expandCreateShorthand` branch at line 119.

Pseudo-code:

```ts
// After built-in / GitHub / local checks, before expandCreateShorthand.
if (templateName.startsWith('@')) {
  const { scope, name } = parseScoped(templateName);
  const manifest = await readOrgManifest(scope); // fetches @scope/create package.json

  if (manifest) {
    const entry =
      name === undefined
        ? await pickTemplate(manifest.templates, { interactive })
        : manifest.templates.find((t) => t.name === name);

    if (entry) {
      // Bundled subdirectory: resolve against the extracted tarball.
      if (entry.template.startsWith('./') || entry.template.startsWith('../')) {
        const extractedRoot = await ensureOrgPackageExtracted(
          manifest.packageName,
          manifest.version,
          manifest.tarballUrl,
        );
        const absPath = resolveBundledPath(extractedRoot, entry.template);
        return { command: 'copy-dir', args: [absPath, ...templateArgs], type: TemplateType.local, /* ... */ };
      }

      // Everything else: recurse through existing discoverTemplate.
      return discoverTemplate(entry.template, templateArgs, workspaceInfo, interactive);
    }
    // `vp create @org/name` with no matching entry → fall through to shorthand.
  }
}

// Existing expandCreateShorthand path.
const expandedName = expandCreateShorthand(templateName);
...
```

`readOrgManifest` is a new helper (co-located with
`checkNpmPackageExists` in `packages/cli/src/utils/package.ts:72-84`). It
should:

- Fetch `https://registry.npmjs.org/@scope/create` (with
  `NPM_CONFIG_REGISTRY` applied) and extract `versions[latest].vp.templates`.
  The registry response also carries `versions[latest].dist.tarball`, which
  is retained on the returned manifest so bundled-path entries can be
  extracted without a second registry round-trip.
- Return `null` on 404 (package doesn't exist → caller falls through to the
  existing shorthand path).
- **Throw** on network failures (timeout, DNS, non-404 HTTP errors) so the
  caller surfaces a hard error instead of silently skipping the picker.
- **Throw** a schema error when `vp.templates` is present but malformed.
- Return `null` when the package exists but has no `vp.templates` field
  (caller falls through to executing `@org/create` as today).

`ensureOrgPackageExtracted` is a new helper that:

- Computes the cache path
  `$VP_HOME/tmp/create-org/<scope>/<name>/<version>/` (reuses the existing
  `VP_HOME` machinery in `crates/vite_shared/src/home.rs`).
- Returns the cached root immediately if the extraction already exists.
- Otherwise streams the tarball over HTTPS, validates its integrity against
  the `dist.integrity` field from the registry JSON, and extracts with a
  small dependency-light tar reader (no child process, no package-manager
  involvement).
- `resolveBundledPath(extractedRoot, entry.template)` normalizes the
  relative path and rejects any result that escapes `extractedRoot` (i.e.
  `../` sequences that would leave the package root).

## Default Org Config

Add a `create` field to `UserConfig` in
`packages/cli/src/define-config.ts:14-35`:

```ts
declare module '@voidzero-dev/vite-plus-core' {
  interface UserConfig {
    // ... existing fields ...

    create?: {
      /**
       * When `vp create` is invoked with no template argument, use this org
       * as the default (equivalent to `vp create <defaultTemplate>`).
       *
       * Accepts any value that would work as the first argument to
       * `vp create` — typically a scope like `@nkzw`.
       */
      defaultTemplate?: string;
    };
  }
}
```

Example `vite.config.ts`:

```ts
import { defineConfig } from '@voidzero-dev/vite-plus';

export default defineConfig({
  create: {
    defaultTemplate: '@nkzw',
  },
});
```

### Precedence

`CLI argument` > `vite.config.ts create.defaultTemplate` > interactive
prompt for template name (today's behavior when bare `vp create` is typed with
no argument and no default).

### Keeping access to the Vite+ built-in templates

Setting `create.defaultTemplate` should never _hide_ the Vite+ built-in
defaults (`vite:monorepo`, `vite:application`, `vite:library`,
`vite:generator`) from an engineer who needs them. Without an escape hatch,
a repo that ships this config would force every contributor to remember the
exact `vite:*` specifier name, which defeats the purpose of interactive
discovery.

The org picker therefore always appends a trailing "Vite+ built-in
templates" entry. Selecting it drops the user into the existing
`getInitialTemplateOptions` picker
(`packages/cli/src/create/initial-template-options.ts:9-31`) unchanged:

```
? Pick a template from @nkzw
❯ monorepo   Full Nakazawa Tech monorepo scaffold
  web        Web app template (Vite + React)
  mobile     Mobile app (React Native) template
  server     Server template (Node + Fastify)
  library    TypeScript library template
  ──────────────────
  › Vite+ built-in templates   Use defaults (monorepo / application / library / generator)
```

Rules:

- The escape-hatch entry is appended by Vite+, not by the org manifest. It
  cannot be suppressed by the org — this is an intentional
  "user-agency-trumps-config" decision, similar to how most modern package
  managers always expose `--help` regardless of project config.
- Selecting it re-enters the standard flow: the picker shown is identical
  to what `vp create` renders in a repo without `defaultTemplate` set, and
  is itself already context-aware (omits `vite:monorepo` inside a monorepo,
  requires a monorepo for `vite:generator`, etc.).
- The entry is placed last, below a separator, so the org's own templates
  remain the visually dominant choice.

For scripted / non-interactive use, engineers can bypass the configured
default by passing any template argument directly — `vp create vite:library`,
`vp create vite:application`, etc. No new CLI flag is added; the existing
"pass an explicit specifier" escape hatch is sufficient for CI and scripts.

The `--no-interactive` error output for `vp create @org` mentions this in
the hint line, so an agent reading the table can pivot:

```
hint: rerun with an explicit selection, e.g. `vp create @nkzw/web`,
      or use a Vite+ built-in template like `vp create vite:application`.
```

### Intentionally out of scope

- **User-level default** at `~/.vite-plus/config.json`. Deferred to a future
  RFC to keep this one tight. Callers who want a personal default can commit
  the project config.
- **Multiple defaults** (e.g. a picker spanning `['@nkzw', '@vercel']`).
  If that need surfaces later, it warrants a separate field
  (`defaultTemplates: string[]`) rather than overloading the singular form.

## Interactive UX

### Picker

When `@org/create`'s manifest is found, `vp create @org` displays a list
prompt over the **context-filtered** entries (see "Context-aware filtering"
below), followed by a trailing **Vite+ built-in templates** entry (see
"Keeping access to the Vite+ built-in templates" above). Sketch:

```
? Pick a template from @nkzw
❯ web       Web app template (Vite + React)
  mobile    Mobile app (React Native) template
  server    Server template (Node + Fastify)
  library   TypeScript library template
  ──────────────────
  › Vite+ built-in templates   Use defaults (monorepo / application / library / generator)
```

### Context-aware filtering

The picker hides entries that don't make sense for the current workspace,
mirroring the existing logic in
`packages/cli/src/create/initial-template-options.ts:9-31` that omits
`vite:monorepo` when Vite+ already detects a monorepo root.

Rule:

- If an entry has `monorepo: true` **and** `vp create` was invoked inside an
  existing monorepo (`workspaceInfoOptional.isMonorepo === true`), the entry
  is filtered out before the picker renders.
- All other entries are shown.

If filtering empties the list entirely, `vp create @org` errors with a clear
message ("no templates from `@org/create` are applicable inside a
monorepo") rather than showing an empty picker.

### Direct-selection behavior

`vp create @org/<name>` bypasses the picker, so filtering does not apply —
but an explicit selection of a `monorepo: true` entry from _within_ a
monorepo is almost certainly a mistake. Vite+ already refuses
`vite:monorepo` in this situation at `packages/cli/src/create/bin.ts:468-472`.
The same error ("Cannot create a monorepo inside an existing monorepo")
extends to manifest entries with `monorepo: true`.

Keyword search: typing filters against `name`, `description`, and
`keywords`. Arrow keys + Enter select; Ctrl-C cancels.

**Decision**: reuse the `@inquirer/prompts` `select` primitive already wired
into `vp create` (`packages/cli/src/create/bin.ts`), with prefix filtering
over `name` / `description` / `keywords`. If real usage surfaces friction
(e.g., orgs with many templates), revisit with a fuzzy-search picker like
`@inquirer/search` in a follow-up.

### `--no-interactive`

When `@org` is passed without a name and interactive mode is disabled, the
command errors and prints the full manifest table — the same table a
dedicated `--list` flag would have produced. This keeps the surface small
(no extra flag), and, critically, gives AI agents reading the output enough
context (name, description, underlying template) to pick an appropriate
option and retry with `vp create @org/<name>`:

```
error: vp create @nkzw requires a template selection in non-interactive mode.

available templates from @nkzw/create:

  NAME     DESCRIPTION                          TEMPLATE
  web      Web app template (Vite + React)      @nkzw/template-web
  mobile   Mobile app (React Native) template   @nkzw/template-mobile
  server   Server template (Node + Fastify)     github:nkzw-tech/template-server
  library  TypeScript library template          @nkzw/template-library
  demo     Bundled demo template                ./templates/demo

hint: rerun with an explicit selection, e.g. `vp create @nkzw/web`,
      or use a Vite+ built-in template like `vp create vite:application`.
```

Notes:

- Output is stable and machine-parseable (fixed column order, whitespace-
  separated). Agents can parse it without a `--json` flag; if that turns out
  to be insufficient, a `--json` output mode is a cheap follow-up.
- The table includes `TEMPLATE` (the resolved specifier) so that a reader
  can understand what each choice actually scaffolds — e.g. whether it
  points to npm, GitHub, or a builtin.
- The table is **context-filtered**: entries with `monorepo: true` are
  omitted when the command runs inside an existing monorepo, matching the
  interactive picker's behavior. A footer line
  (`omitted 1 monorepo-only entry because this workspace is already a monorepo`)
  makes the filtering visible to both humans and agents.
- The error is written to stderr; the table itself can go to stdout so it
  remains usable when redirected.

## Authoring Guide for Org Maintainers

The manifest convention is intentionally cheap for orgs to adopt. There are
two common layouts; pick whichever matches the org's template count and
release cadence.

**Layout 1: Bundled templates in a single package (recommended for most
orgs).** All templates live as subdirectories of `@org/create` itself;
manifest entries use `./relative/path` to reference them. This is the same
pattern used by `create-vite`, `create-next-app`, and most enterprise
scaffolding kits — one repo, one publish, one versioning story.

```
@nkzw/create/
├── package.json              # "vp": { "templates": [{ "template": "./templates/demo" }, ...] }
├── templates/
│   ├── demo/
│   │   ├── package.json
│   │   └── src/...
│   ├── web/...
│   └── library/...
└── README.md
```

**Layout 2: Manifest-only, pointing to external packages.** Useful when the
org already publishes independent `@org/template-*` packages (or hosts
templates on GitHub) and wants `@org/create` to be a thin index. Manifest
entries use npm specifiers or `github:` URLs.

```
@nkzw/create/
├── package.json              # "vp": { "templates": [{ "template": "@org/template-web" }, ...] }
└── README.md
```

The two layouts can also be mixed — the example manifest higher up uses
external packages for most entries and `./templates/demo` for one.

No code is required if the manifest is your only surface. However, it is
strongly recommended that `@org/create` remains **runnable as a classic
`create-*` package** too, as a fallback for users on plain
`npm create` / `yarn create`. Typical layout adds a bin script:

```
@nkzw/create/
├── package.json         # "bin": { "create": "./bin.js" }, and "vp.templates"
├── bin.js               # small launcher that runs the picker for npm users
├── templates/...        # (if using Layout 1)
└── README.md
```

This gives you:

- `npm create @nkzw` / `yarn create @nkzw` → runs your `bin.js` (legacy path).
- `vp create @nkzw` → reads the manifest directly, no `bin.js` execution.

### Choosing what the manifest entries point to

Each `template` field is a specifier passed to Vite+'s `discoverTemplate`.
Common choices:

| Choice                            | When to use it                                                                                                      |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| `./templates/foo` (bundled path)  | The template lives as a subdirectory of `@org/create` itself. Lowest authoring overhead; recommended for most orgs. |
| `@org/template-foo` (npm package) | The template is independently published and versioned.                                                              |
| `github:org/template-foo`         | The template lives in a GitHub repo, not npm. Uses `degit`.                                                         |
| `vite:monorepo` / other builtins  | Defer to Vite+ builtins with your own wrapper entry.                                                                |
| Local workspace package name      | Template lives inside the same monorepo as `@org/create`. See bingo/local path in `discoverTemplate`.               |

### Marking monorepo-only templates

If a manifest entry scaffolds a **monorepo** (i.e., it creates a workspace
root, not a single package), mark it with `monorepo: true`. Vite+ will then
hide that entry from the picker when a user runs `vp create @org` from
inside an existing monorepo, and will error with a clear message if the
user explicitly types `vp create @org/<entry>` in that context. This mirrors
how Vite+ already filters its own `vite:monorepo` builtin in
`packages/cli/src/create/initial-template-options.ts:9-31`.

Typical usage: an org's `@org/create` manifest lists one `monorepo: true`
entry (for greenfield consumers) alongside several single-package entries
(web / mobile / server / library) that can also be used to scaffold
individual packages inside the monorepo.

### Versioning

The manifest is resolved against `@org/create@latest` by default. Org
maintainers can pin a specific version per entry (e.g.
`@nkzw/template-web@2.3.0`) inside the `template` field. We do not add a
separate `version` field on the manifest entry to avoid two competing knobs.

### Publishing checklist

1. Create `@org/create` (scoped npm package) if you don't already have one.
2. Add a `vp.templates` array to `package.json`.
3. (Optional) Provide a `bin` launcher for `npm create @org` compatibility.
4. Publish.
5. Verify with `vp create @org --no-interactive` (prints the available
   template names) or `vp create @org` (opens the picker).
6. (Optional) Commit `create: { defaultTemplate: '@org' }` in your
   internal template repos.

### Backwards compatibility

If you already publish `@org/create` as a single-template package, **adding
`vp.templates` is not a breaking change for `vp create` users** — the picker
replaces the direct execution, and each manifest entry can still point to
your existing template. Users on plain `npm create @org` are unaffected
either way; they continue to run your `bin` script.

## Error Handling

| Situation                                                                       | Behavior                                                                                                      |
| ------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| `@org/create` does not exist on npm                                             | Same "template not found" error as today.                                                                     |
| `@org/create` exists, no `vp.templates`                                         | Fall through to today's behavior: run `@org/create`. No error.                                                |
| `vp.templates` is not an array                                                  | Schema error: `@org/create: vp.templates must be an array`.                                                   |
| Manifest entry missing `name` / `description` / `template`                      | Schema error with the offending index and field.                                                              |
| Manifest entry has duplicate `name`                                             | Schema error listing the duplicate.                                                                           |
| Chosen template fails to resolve (404, bad URL)                                 | Downstream error with context: `selected 'web' from @nkzw/create: <downstream error>`.                        |
| Network failure fetching manifest                                               | Hard error. Never silently skip the picker when the user explicitly typed `@org`.                             |
| `--no-interactive` without `@org/<name>`                                        | Error listing valid names (see above).                                                                        |
| All manifest entries filtered (e.g. all `monorepo: true` inside a monorepo)     | Error: `no templates from @org/create are applicable inside a monorepo`.                                      |
| `vp create @org/<name>` where `name` has `monorepo: true` and cwd is a monorepo | Same error as the builtin: `Cannot create a monorepo inside an existing monorepo` (mirrors `bin.ts:468-472`). |
| Bundled path (`./foo`) resolves outside `@org/create` root                      | Schema error at manifest-validation time: `vp.templates[i].template escapes the package root`.                |
| Bundled path points to a directory that does not exist in the tarball           | Scaffolding error: `selected 'demo' from @nkzw/create: ./templates/demo not found in @nkzw/create@1.0.0`.     |
| Tarball download or extraction fails                                            | Hard error with the upstream cause. Cached partial extractions are cleaned up before retry.                   |

## Alternatives Considered

### (a) Dedicated `@org/vp-templates` package

An earlier proposal suggested a dedicated `@org/vp-templates` package,
which would introduce a new shorthand rule (`vp create @org` →
`@org/vp-templates`). **Rejected** because:

- The existing `@org/create` shorthand already matches the ecosystem
  convention (`npm create @org`, `yarn create @org`).
- Gating picker behavior on manifest presence cleanly separates the two
  modes without a new rule.
- Orgs that already publish `@org/create` don't need to publish a second
  package to adopt Vite+.

### (b) Separate `templates.json` file inside the package

**Rejected** because `package.json` `vp.templates` is readable via a single
`npm view` / registry HEAD request without fetching the package tarball.
`templates.json` would require either tarball download or degit-style git
fetch, both of which are slower and have more failure modes.

### (c) User-level default at `~/.vite-plus/config.json`

**Deferred** to a future RFC. Project-level config is the clear priority:
companies set this once in their repo and every clone inherits it. Solo
users who want a personal default can use a shell alias until the follow-up
RFC lands.

### (d) `exports['./templates']` JS-native manifest

**Rejected** because executing the package to enumerate templates means
network download + sandboxed run for what should be a static list. Also
forces every implementation of the picker (Vite+, future ports, docs tools)
to spin up a JS runtime.

### (e) Special-case `@org` at the CLI layer

**Rejected** because it's less composable with the existing
`discoverTemplate` pipeline. Hooking into `discoverTemplate` reuses all the
existing template-resolution, parent-directory inference, and runner
plumbing.

## Implementation Plan

Four phases, each independently shippable.

### Phase 1 — Core

- Add `readOrgManifest` helper in `packages/cli/src/utils/package.ts`.
- Extend `discoverTemplate` in
  `packages/cli/src/create/discovery.ts` with the manifest branch before
  `expandCreateShorthand`.
- Schema validation for `vp.templates` (simple hand-rolled checks; no new
  dep).
- Interactive picker using `@inquirer/prompts` `select`.
- Direct selection: `vp create @org/name` uses manifest when present.
- Tarball fetch + extract (`ensureOrgPackageExtracted`) with
  `$VP_HOME/tmp/create-org/<scope>/<name>/<version>/` cache, plus the
  directory-copy scaffold path for bundled `./` entries.

### Phase 2 — Config

- Add `create: { defaultTemplate }` to `UserConfig` in
  `packages/cli/src/define-config.ts`.
- Wire the config read in `packages/cli/src/create/bin.ts` so bare
  `vp create` with a configured default is equivalent to
  `vp create <defaultTemplate>`.
- CLI arg still wins over config.

### Phase 3 — UX polish

- `--no-interactive` error message listing available names.
- Clear error message on network failure when fetching the manifest.
- Context-aware filtering of `monorepo: true` entries when invoked inside
  an existing monorepo (mirrors `initial-template-options.ts:9-31`).

### Phase 4 — Docs

- Authoring guide on viteplus.dev (mirrors this RFC's authoring section).
- Changelog entry.
- Update `packages/cli/README.md` `vp create` section.

## Testing Strategy

Fixture additions under `packages/cli/snap-tests-global/create-org-*`:

| Fixture                                  | What it verifies                                                                                                  |
| ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| `create-org-with-manifest`               | `vp create @org` opens picker and runs selection                                                                  |
| `create-org-no-manifest`                 | `vp create @org` falls through to `@org/create`                                                                   |
| `create-org-direct-name-hit`             | `vp create @org/web` uses manifest entry                                                                          |
| `create-org-direct-name-miss`            | `vp create @org/web` falls back to shorthand                                                                      |
| `create-org-no-interactive-error`        | `--no-interactive` without name errors clearly and prints the full manifest table (name + description + template) |
| `create-org-config-default`              | `vp create` in repo with `defaultTemplate` uses it                                                                |
| `create-org-invalid-manifest`            | Invalid `vp.templates` produces a schema error                                                                    |
| `create-org-monorepo-filter`             | `monorepo: true` entries hidden from picker and `--no-interactive` output when run inside a monorepo              |
| `create-org-monorepo-direct-in-monorepo` | `vp create @org/<monorepo-entry>` errors with "cannot create a monorepo inside an existing monorepo"              |
| `create-org-builtin-escape-hatch`        | Org picker always ends with a "Vite+ built-in templates" entry that routes to `getInitialTemplateOptions`         |
| `create-org-bundled-subdir`              | Manifest entry with `./templates/demo` fetches tarball, extracts to `$VP_HOME/tmp/create-org/...`, scaffolds dir  |
| `create-org-bundled-escape-check`        | `./../outside` path is rejected at schema-validation time before any tarball fetch                                |

Unit tests:

- `readOrgManifest` parsing + schema validation.
- Resolution logic (manifest wins over shorthand).

Integration: the registry fetch is exercised against a local mock registry
/ stubbed `fetch`, so CI stays fast and offline. We do **not** publish a
dedicated `@voidzero-dev/create-test-fixture` package; the marginal value of
catching registry-surface regressions isn't worth the maintenance surface.
If the npm registry API ever changes in a breaking way, the bug will surface
the first time a user hits it and we can add a thin smoke test at that point.

## CLI Help Output

The relevant additions to `vp create --help`:

```
Usage: vp create [TEMPLATE] [OPTIONS] [-- TEMPLATE_OPTIONS]

Arguments:
  TEMPLATE           Template to scaffold from. May be:
                       - an org scope (e.g. @nkzw) for org templates
                       - a scoped name (e.g. @nkzw/web) for a specific
                         template from an org's manifest
                       - any value accepted today: create-*, github:*, vite:*,
                         local package name
                     When omitted, uses `create.defaultTemplate` from
                     vite.config.ts if set.

Options:
  ...existing flags...

Configuration (vite.config.ts):
  create.defaultTemplate   Default org/template used by bare `vp create`.
```

## Compatibility

- **Org packages that already exist as single-template `@org/create`**: keep
  working unchanged until they opt in by adding `vp.templates`.
- **Plain `npm create @org` / `yarn create @org`**: unaffected. Those
  consumers run the package's `bin` script, which is outside Vite+'s scope.
- **Existing `@org/create-name` shorthand**: preserved as a fallback when
  the manifest doesn't mention `name`.

## Real-World Usage Examples

### Org with a published `@org/create` manifest

```bash
# Discovery
vp create @nkzw
# → picker with: web, mobile, server, library

# Direct
vp create @nkzw/server

# Non-interactive (CI)
vp create @nkzw/library --no-interactive --directory ./packages/new-lib
```

### Enterprise monorepo with a default

```ts
// vite.config.ts at the company's template-seed repo
export default defineConfig({
  create: { defaultTemplate: '@acme' },
});
```

```bash
# Inside that repo: engineers just type `vp create`
vp create
# → picker from @acme/create, plus a trailing
#   "Vite+ built-in templates" entry for users who need vite:library etc.

# Explicit builtin (bypasses the configured default)
vp create vite:library
```

### Mixed-specifier manifest

```json
{
  "vp": {
    "templates": [
      { "name": "web", "description": "Next.js app", "template": "@acme/template-web" },
      { "name": "docs", "description": "Docs site", "template": "github:acme/template-docs" },
      { "name": "tool", "description": "CLI tool", "template": "vite:library" }
    ]
  }
}
```

## Future Enhancements

- **User-level default org** at `~/.vite-plus/config.json`.
- **Multiple default orgs** (picker spans multiple scopes when the config is
  an array).
- **Non-npm manifest sources** (raw URL, git repo) for orgs that don't
  publish to npm.
- **Manifest groups/categories** for orgs with >~10 templates.
- **Post-install hints** surfacing `vp create @org` when a user installs
  `@org/create` directly.

## Resolved Decisions

- **Picker implementation**: plain `@inquirer/prompts` `select` with prefix
  filtering. Upgrade to a fuzzy-search picker (`@inquirer/search`) in a
  follow-up if real usage reports friction.
- **No `--list` flag**: manifest inspection goes through
  `vp create @org --no-interactive`, which prints the full manifest table
  (name, description, resolved template specifier) as part of its error
  output. This gives scripts, CI logs, and AI agents enough context to pick
  a template without needing a dedicated `--list` flag.
- **Network failure = hard error**: never silently skip the picker when the
  user explicitly typed `@org`. Users on flaky networks get a clear,
  actionable error instead of mysteriously running a single-template
  fallback.
- **Built-in templates always reachable from the org picker**: when
  `create.defaultTemplate` is set, the org picker appends a trailing "Vite+
  built-in templates" entry that routes to the existing
  `getInitialTemplateOptions` flow. No new CLI flag; explicit specifiers
  like `vp create vite:application` remain the scripted escape hatch.
  (Resolves review feedback on #1398.)
- **Bundled subdirectory templates**: manifest entries may use relative
  paths (`./templates/demo`) that resolve against the enclosing
  `@org/create` package root. Vite+ fetches and extracts the tarball once
  per version into `$VP_HOME/tmp/create-org/<scope>/<name>/<version>/`,
  then scaffolds by directory copy. This lets an org ship N templates in a
  single package rather than publishing N independent `@org/template-*`
  packages — the dominant pattern in `create-*` ecosystems
  (`create-vite`, `create-next-app`, enterprise kits). Paths that escape
  the package root are rejected at schema validation.
- **Local test fixtures only**: snap-tests and unit tests use a local mock
  registry / stubbed `fetch`. No dedicated published fixture package —
  registry-surface regressions are low-frequency and caught downstream.
- **Config field name `defaultTemplate` (singular)**: reads naturally for a
  single value, which is all this RFC ships. If support for multiple default
  orgs is added later, it will live under a separate `defaultTemplates:
string[]` field rather than overloading the singular form.
- **No `--json` output mode on day one**: the fixed-column text table from
  `--no-interactive` is already machine-parseable. Revisit if downstream
  tooling reports friction.

## Conclusion

- `vp create @org` becomes a branded entry point backed by an org-owned
  manifest.
- Opt-in via a single `vp.templates` field in `@org/create`'s `package.json`.
- Adopt in a repo via `create: { defaultTemplate: '@org' }`.
- Zero-risk for existing `@org/create` publishers.
- Consistent with the `code-generator.md` dual-mode strategy.
