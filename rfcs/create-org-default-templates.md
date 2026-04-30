# RFC: Organization Default Templates for `vp create`

> Status: **Implemented** on branch `vp-create-support-org` (PR #1398).
> Sections below describe the design as shipped; the trailing "Resolved
> Decisions" list reflects every decision that landed during
> implementation, including ones that emerged from review (`.npmrc`
> registry/auth, `__vp_` reserved prefix, sanitized cache host segment,
> and others). The "Implementation State" section near the bottom
> points at the concrete files.

## Summary

Give organizations a single, branded entry point into their curated set of
project templates through `vp create @org`. When `@org/create` publishes a
`createConfig.templates` manifest in its `package.json`, Vite+ renders an interactive
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
2. Type the full command: `vp create @your-org/create-web`,
   `vp create @your-org/create-mobile`, etc.
3. Find these names in a README, a wiki, or Slack.

This works, but it isn't discoverable, and it forces the org to document
package names in a medium that ages badly. The industry convention for
frameworks (Vite, Next, Nuxt) is "one command per framework" precisely because
a single memorable entry point outperforms a list of names.

### What engineers should be able to type

```bash
# Interactively pick a template from the @your-org org
vp create @your-org

# Pick a specific manifest entry directly
vp create @your-org:web

# Inside a repo that sets @your-org as the default:
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
# Already works: runs @your-org/create
vp create @your-org

# Already works: runs @your-org/create-web
vp create @your-org/web
```

The piece that doesn't exist yet is **discovering and choosing between multiple
templates owned by the same org**. That is what this RFC specifies.

## Proposed Solution

### High-level flow

1. User runs `vp create @org`.
2. `expandCreateShorthand` maps this to `@org/create` (unchanged).
3. Before dispatching to the template runner, `vp create` reads
   `@org/create`'s `package.json` from the npm registry.
4. If the `package.json` contains a `createConfig.templates` field, Vite+ renders an
   interactive picker over those entries.
5. After the user picks (or passes `@org:<name>` directly — colon separator
   mirrors the existing `vite:monorepo` / `vite:library` builtin syntax and
   keeps manifest entries syntactically distinct from real `@org/package`
   npm specifiers), Vite+ resolves the selected entry's `template` field
   through the existing `discoverTemplate` pipeline — which supports npm,
   GitHub, builtin `vite:*`, and local workspace packages.
6. If `createConfig.templates` is **absent**, Vite+ falls through to today's behavior
   and executes `@org/create` as a normal template. This keeps the feature
   zero-risk for org owners who haven't opted in.

### Command matrix

| Command                          | Manifest present? | Behavior                                                                       |
| -------------------------------- | ----------------- | ------------------------------------------------------------------------------ |
| `vp create @org`                 | yes               | Fetch manifest → picker → run chosen template                                  |
| `vp create @org`                 | no                | Run `@org/create` as today (unchanged)                                         |
| `vp create @org:name`            | yes, has `name`   | Run manifest entry `name`                                                      |
| `vp create @org:name`            | yes, no `name`    | Hard error listing available manifest entry names                              |
| `vp create @org:name`            | no                | Same hard error — `:`-form is explicit manifest lookup, no silent fall-through |
| `vp create @org/name`            | n/a               | Unchanged from pre-feature: existing `@org/create-name` shorthand              |
| `vp create` (in configured repo) | yes               | Same as `vp create @org` where `@org` is the configured default                |
| `vp create <anything-else>`      | n/a               | Unchanged                                                                      |

## Manifest Schema

The manifest lives at `createConfig.templates` in `@org/create`'s `package.json`.

```json
{
  "name": "@your-org/create",
  "version": "1.0.0",
  "description": "Project templates from the @your-org org",
  "createConfig": {
    "templates": [
      {
        "name": "monorepo",
        "description": "Monorepo scaffold",
        "template": "@your-org/template-monorepo",
        "monorepo": true
      },
      {
        "name": "web",
        "description": "Web app template (Vite + React)",
        "template": "@your-org/template-web"
      },
      {
        "name": "mobile",
        "description": "Mobile app (React Native) template",
        "template": "@your-org/template-mobile"
      },
      {
        "name": "server",
        "description": "Server template (Node + Fastify)",
        "template": "github:your-org/template-server"
      },
      {
        "name": "library",
        "description": "TypeScript library template",
        "template": "@your-org/template-library"
      },
      {
        "name": "demo",
        "description": "Bundled demo template (lives inside @your-org/create)",
        "template": "./templates/demo"
      }
    ]
  }
}
```

### Field reference

| Field                                  | Type              | Required | Notes                                                                                                                                                                                                                                                                                                                                                                                  |
| -------------------------------------- | ----------------- | -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `createConfig.templates`               | `TemplateEntry[]` | yes      | Non-empty array. Empty arrays are treated as "no manifest" (fall through to `@org/create` run).                                                                                                                                                                                                                                                                                        |
| `createConfig.templates[].name`        | `string`          | yes      | Kebab-case. Used for `vp create @org:<name>` direct selection. Must be unique within the array. Names starting with `__vp_` are reserved for internal sentinel values and rejected at schema validation.                                                                                                                                                                               |
| `createConfig.templates[].description` | `string`          | yes      | One-line description shown in the picker.                                                                                                                                                                                                                                                                                                                                              |
| `createConfig.templates[].template`    | `string`          | yes      | One of: (a) an npm package specifier (`@your-org/template-web`, optionally `@version`), (b) a GitHub URL (`github:user/repo`, `https://github.com/...`), (c) a `vite:*` builtin, (d) a local workspace package name, or (e) a relative path (`./templates/demo`, `../foo`) that resolves against the enclosing `@org/create` package root. See "Bundled subdirectory templates" below. |
| `createConfig.templates[].monorepo`    | `boolean`         | no       | If `true`, marks this entry as a _monorepo-creating_ template. Hidden from the picker when `vp create` is invoked inside an existing monorepo. Mirrors the built-in behavior that filters `vite:monorepo` out of `getInitialTemplateOptions` (`packages/cli/src/create/initial-template-options.ts:9-31`). Defaults to `false`.                                                        |

### Invalid manifests

A present-but-invalid `createConfig.templates` field should **not** silently fall through
to the shorthand. It should produce a schema error with the offending field
path (e.g. `@your-org/create: createConfig.templates[2].template is required`), because the
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
`.npmrc` scope registries + `NPM_CONFIG_REGISTRY`), and extracts it to a
per-version cache under
`$VP_HOME/tmp/create-org/<host>/<scope>/create/<version>/`. The leading
`<host>` segment (sanitized for Windows-illegal characters) keeps two repos
that resolve the same `<scope>@<version>` through different registries from
sharing a cache slot. Subsequent invocations against the same host reuse
the cached extraction. A small tar-reader implementation (no external
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
    // `vp create @org:name` with no matching entry → hard error (no fall-through).
  }
}

// Existing expandCreateShorthand path.
const expandedName = expandCreateShorthand(templateName);
...
```

`readOrgManifest` lives in `packages/cli/src/create/org-manifest.ts`. It:

- Fetches the packument from the scope's registry (resolved via
  `getNpmRegistry(scope)` in `packages/cli/src/utils/npm-config.ts`, which
  layers `~/.npmrc` → project `.npmrc` → `npm_config_*` env vars and
  honors `@scope:registry=...` overrides).
- Anonymous on the first request; retries with the matching `_authToken`
  / `_auth` / `username:_password` from `.npmrc` only if the server
  returns 401/403, so public registries never see the token.
- Resolves the manifest version: when `parseOrgScopedSpec` extracted a
  version (`@scope@1.2.3`, `@scope:web@next`), looks it up in
  `dist-tags[...]` first, then `versions[...]` directly; otherwise
  `dist-tags.latest`. Unknown versions are a hard error.
- Returns `null` on 404 (package doesn't exist → scope-only input falls
  through to the existing shorthand path; `@org:name` is a hard error).
- **Throws** on non-404 HTTP errors and on schema violations.
- Carries `tarballUrl` and `integrity` on the returned manifest so
  bundled-path entries can be extracted without a second registry
  round-trip.

`ensureOrgPackageExtracted` (`packages/cli/src/create/org-tarball.ts`):

- Computes the cache path
  `$VP_HOME/tmp/create-org/<host>/<scope>/create/<version>/`. The
  `<host>` segment comes from `manifest.tarballUrl` (sanitized via
  `sanitizeHostForPath` to replace Windows-illegal characters like `:`
  in `localhost:4873`); two repos resolving the same
  `<scope>@<version>` through different registries don't collide on
  one cache slot.
- Returns the cached root immediately if the extraction already exists.
- Otherwise streams the tarball over HTTPS (auth retry mirrors the
  manifest fetch), enforces a 50 MB cap, validates `dist.integrity`,
  and extracts with `nanotar` to a staging dir that's atomically
  renamed into place. Sibling `.tmp-*` staging dirs older than 24h are
  pruned at the start of each fresh extract.
- Tar entries outside `package/` are skipped; their stored mode bits
  are preserved (so `gradlew` and friends stay executable).
- `resolveBundledPath(extractedRoot, entry.template)` normalizes the
  relative path and rejects any result that escapes `extractedRoot`
  (`../` sequences that would leave the package root).

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
       * `vp create` — typically a scope like `@your-org`.
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
    defaultTemplate: '@your-org',
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
? Pick a template from @your-org
❯ monorepo   Monorepo scaffold
  web        Web app template (Vite + React)
  mobile     Mobile app (React Native) template
  server     Server template (Node + Fastify)
  library    TypeScript library template
  ──────────────────
  › Vite+ built-in templates   Use defaults (monorepo / application / library)
```

The hint trailing "Vite+ built-in templates" matches what
`getInitialTemplateOptions` actually offers for the current workspace
context — inside an existing monorepo the hint reads "Use defaults
(application / library)" since `vite:monorepo` is filtered out and
`vite:generator` isn't part of the picker.

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
hint: rerun with an explicit selection, e.g. `vp create @your-org:web`,
      or use a Vite+ built-in template like `vp create vite:application`.
```

### Intentionally out of scope

- **User-level default** at `~/.vite-plus/config.json`. Deferred to a future
  RFC to keep this one tight. Callers who want a personal default can commit
  the project config.
- **Multiple defaults** (e.g. a picker spanning `['@your-org', '@vercel']`).
  If that need surfaces later, it warrants a separate field
  (`defaultTemplates: string[]`) rather than overloading the singular form.

## Interactive UX

### Picker

When `@org/create`'s manifest is found, `vp create @org` displays a list
prompt over the **context-filtered** entries (see "Context-aware filtering"
below), followed by a trailing **Vite+ built-in templates** entry (see
"Keeping access to the Vite+ built-in templates" above). Sketch:

```
? Pick a template from @your-org
❯ web       Web app template (Vite + React)
  mobile    Mobile app (React Native) template
  server    Server template (Node + Fastify)
  library   TypeScript library template
  ──────────────────
  › Vite+ built-in templates   Use defaults (monorepo / application / library)
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

If filtering empties the list entirely, `vp create @org` prints an
`info:` note ("No templates from `@org/create` are applicable inside a
monorepo — showing Vite+ built-in templates instead.") and routes to the
built-in picker, so the user never sees an empty picker and isn't left
at a dead end.

### Direct-selection behavior

`vp create @org:<name>` bypasses the picker, so filtering does not apply —
but an explicit selection of a `monorepo: true` entry from _within_ a
monorepo is almost certainly a mistake. Vite+ already refuses
`vite:monorepo` in this situation at `packages/cli/src/create/bin.ts:468-472`.
The same error ("Cannot create a monorepo inside an existing monorepo")
extends to manifest entries with `monorepo: true`.

Keyword search: typing filters against `name`, `description`, and
`keywords`. Arrow keys + Enter select; Ctrl-C cancels.

**Decision**: reuse the `@voidzero-dev/vite-plus-prompts` `select` primitive
already wired into `vp create` (`packages/cli/src/create/bin.ts:5`, which
wraps `@clack/core`), with prefix filtering over `name` / `description` /
`keywords`. If real usage surfaces friction (e.g., orgs with many
templates), revisit with a fuzzy-search picker (e.g. based on
`@voidzero-dev/vite-plus-prompts`' `autocomplete`) in a follow-up.

### `--no-interactive`

When `@org` is passed without a name and interactive mode is disabled, the
command errors and prints the full manifest table — the same table a
dedicated `--list` flag would have produced. This keeps the surface small
(no extra flag), and, critically, gives AI agents reading the output enough
context (name, description, underlying template) to pick an appropriate
option and retry with `vp create @org:<name>`:

```
A template name is required when running `vp create @your-org` in non-interactive mode.

Available templates in @your-org/create:

  NAME     DESCRIPTION                          TEMPLATE
  web      Web app template (Vite + React)      @your-org/template-web
  mobile   Mobile app (React Native) template   @your-org/template-mobile
  server   Server template (Node + Fastify)     github:your-org/template-server
  library  TypeScript library template          @your-org/template-library
  demo     Bundled demo template                ./templates/demo

Examples:
  # Scaffold a specific template from the org
  vp create @your-org:web --no-interactive

  # Or use a Vite+ built-in template
  vp create vite:application --no-interactive
```

Shape matches the existing `vp create` missing-argument message
(`packages/cli/src/create/bin.ts:387-399`) — same opening sentence pattern,
same `Examples:` block — so users see a consistent shape for any
missing-template error across the command.

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
@your-org/create/
├── package.json              # "createConfig": { "templates": [{ "template": "./templates/demo" }, ...] }
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
@your-org/create/
├── package.json              # "createConfig": { "templates": [{ "template": "@org/template-web" }, ...] }
└── README.md
```

The two layouts can also be mixed — the example manifest higher up uses
external packages for most entries and `./templates/demo` for one.

No code is required if the manifest is your only surface. However, it is
strongly recommended that `@org/create` remains **runnable as a classic
`create-*` package** too, as a fallback for users on plain
`npm create` / `yarn create`. Typical layout adds a bin script:

```
@your-org/create/
├── package.json         # "bin": { "create": "./bin.js" }, and "createConfig.templates"
├── bin.js               # small launcher that runs the picker for npm users
├── templates/...        # (if using Layout 1)
└── README.md
```

This gives you:

- `npm create @your-org` / `yarn create @your-org` → runs your `bin.js` (legacy path).
- `vp create @your-org` → reads the manifest directly, no `bin.js` execution.

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
`@your-org/template-web@2.3.0`) inside the `template` field. We do not add a
separate `version` field on the manifest entry to avoid two competing knobs.

### Publishing checklist

1. Create `@org/create` (scoped npm package) if you don't already have one.
2. Add a `createConfig.templates` array to `package.json`.
3. (Optional) Provide a `bin` launcher for `npm create @org` compatibility.
4. Publish.
5. Verify with `vp create @org --no-interactive` (prints the available
   template names) or `vp create @org` (opens the picker).
6. (Optional) Commit `create: { defaultTemplate: '@org' }` in your
   internal template repos.

### Backwards compatibility

If you already publish `@org/create` as a single-template package, **adding
`createConfig.templates` is not a breaking change for `vp create` users** — the picker
replaces the direct execution, and each manifest entry can still point to
your existing template. Users on plain `npm create @org` are unaffected
either way; they continue to run your `bin` script.

## Error Handling

| Situation                                                                          | Behavior                                                                                                                                                                                                   |
| ---------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `@org/create` does not exist on npm                                                | Same "template not found" error as today.                                                                                                                                                                  |
| `@org/create` exists, no `createConfig.templates`                                  | Fall through to today's behavior: run `@org/create`. No error.                                                                                                                                             |
| `createConfig.templates` is not an array                                           | Schema error: `@org/create: createConfig.templates must be an array`.                                                                                                                                      |
| Manifest entry missing `name` / `description` / `template`                         | Schema error with the offending index and field.                                                                                                                                                           |
| Manifest entry has duplicate `name`                                                | Schema error listing the duplicate.                                                                                                                                                                        |
| Chosen template fails to resolve (404, bad URL)                                    | Downstream error with context: `selected 'web' from @your-org/create: <downstream error>`.                                                                                                                 |
| Network failure fetching manifest                                                  | Hard error. Never silently skip the picker when the user explicitly typed `@org`.                                                                                                                          |
| `--no-interactive` without `@org:<name>`                                           | Error listing valid names (see above).                                                                                                                                                                     |
| All manifest entries filtered (e.g. all `monorepo: true` inside a monorepo)        | Print an `info:` note (`"No templates from @org/create are applicable inside a monorepo — showing Vite+ built-in templates instead."`) and route to the built-in picker. Keeps the user out of a dead end. |
| `vp create @org:<name>` where `name` has `monorepo: true` and cwd is a monorepo    | Same error as the builtin: `Cannot create a monorepo inside an existing monorepo` (mirrors `bin.ts:468-472`).                                                                                              |
| `vp create @org:<name>` where `name` isn't in the manifest (or manifest is absent) | Hard error listing the available entries — no silent fall-through to the `@org/create-name` shorthand, which is reserved for the slash-form.                                                               |
| Bundled path (`./foo`) resolves outside `@org/create` root                         | Schema error at manifest-validation time: `createConfig.templates[i].template escapes the package root`.                                                                                                   |
| Bundled path points to a directory that does not exist in the tarball              | Scaffolding error: `selected 'demo' from @your-org/create: ./templates/demo not found in @your-org/create@1.0.0`.                                                                                          |
| Tarball download or extraction fails                                               | Hard error with the upstream cause. Cached partial extractions are cleaned up before retry.                                                                                                                |

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

**Rejected** because `package.json` `createConfig.templates` is readable via a single
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

## Implementation State

Shipped on branch `vp-create-support-org` (PR #1398). Concrete
landings:

| Module                                          | Role                                                                                                                 |
| ----------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `packages/cli/src/create/org-manifest.ts`       | `parseOrgScopedSpec`, `readOrgManifest`, schema validation (incl. `__vp_` reserved-prefix check).                    |
| `packages/cli/src/create/org-resolve.ts`        | `resolveOrgManifestForCreate`, `getConfiguredDefaultTemplate`, picker / `--no-interactive` table dispatch.           |
| `packages/cli/src/create/org-picker.ts`         | `pickOrgTemplate` interactive picker, escape-hatch entry, context-aware filtering.                                   |
| `packages/cli/src/create/org-tarball.ts`        | `ensureOrgPackageExtracted`, `resolveBundledPath`, `sanitizeHostForPath`, integrity verification, mode preservation. |
| `packages/cli/src/create/templates/bundled.ts`  | `executeBundledTemplate` (directory-copy scaffold for relative-path manifest entries).                               |
| `packages/cli/src/create/discovery.ts`          | `bundledLocalPath` + `skipShorthand` parameters threading manifest results into the existing template flow.          |
| `packages/cli/src/create/bin.ts`                | Unified monorepo branch (builtin + bundled), git-init prompt, `injectCreateDefaultTemplate` for `@org` monorepos.    |
| `packages/cli/src/create/utils.ts`              | `ensureGitignoreNodeModules` post-`git init` guarantee.                                                              |
| `packages/cli/src/define-config.ts`             | `create: { defaultTemplate?: string }` augmentation on `UserConfig`.                                                 |
| `packages/cli/src/migration/migrator.ts`        | `injectCreateDefaultTemplate` helper (called from `bin.ts`, gated on bundled monorepo).                              |
| `packages/cli/src/utils/npm-config.ts`          | `.npmrc` parser, `getNpmRegistry(scope?)`, `getNpmAuthHeader(url)`, `fetchNpmResource` (401/403 retry).              |
| `packages/cli/src/resolve-vite-config.ts`       | `findWorkspaceRoot` exported for the default-template walk-up.                                                       |
| `docs/guide/create.md`, `docs/config/create.md` | Authoring guide and `create.defaultTemplate` reference.                                                              |

## Testing

End-to-end snap-test fixtures under `packages/cli/snap-tests/` use a
shared local mock registry (`.shared/mock-npm-registry.mjs`) that
serves a per-fixture `mock-manifest.json` and any tarballs in
`<fixture>/tarballs/`. CI stays fast and offline.

| Fixture                                  | What it verifies                                                                                                                     |
| ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| `create-org-bundled`                     | `vp create @org:<entry>` extracts the tarball and scaffolds a single-project bundled subdirectory.                                   |
| `create-org-bundled-escape-check`        | `./../outside` paths rejected at schema validation before any tarball fetch.                                                         |
| `create-org-bundled-monorepo`            | Bundled `monorepo: true` entry: scaffold + `git init` + `create.defaultTemplate: '@org'` injection + `node_modules` in `.gitignore`. |
| `create-org-config-default`              | `vp create` in a repo with `create.defaultTemplate` uses the configured org.                                                         |
| `create-org-invalid-manifest`            | Invalid `createConfig.templates` produces a schema error.                                                                            |
| `create-org-monorepo-filter`             | `monorepo: true` entries hidden from picker / `--no-interactive` output when run inside a monorepo.                                  |
| `create-org-monorepo-direct-in-monorepo` | `vp create @org:<monorepo-entry>` inside a monorepo errors loudly.                                                                   |
| `create-org-no-interactive-error`        | `--no-interactive` without a name errors and prints the full manifest table (name + description + template).                         |
| `snap-tests-global/new-vite-monorepo`    | Builtin `vp create vite:monorepo` does NOT auto-inject `create.defaultTemplate` (negative case for the gating).                      |

Unit tests under `packages/cli/src/**/__tests__/`:

- `org-manifest.spec.ts` — `parseOrgScopedSpec` (incl. `@scope@version`,
  `@scope:name@version` forms), `filterManifestForContext`,
  `readOrgManifest` happy path + schema errors + version pinning + auth
  retry on 401/403.
- `org-tarball.spec.ts` — `parseEntryMode`, `normalizeEntryName`,
  `cleanupStaleStagingDirs`, `resolveBundledPath` path-escape,
  `sanitizeHostForPath` (Windows-illegal chars).
- `org-picker.spec.ts` — interactive picker filtering + escape-hatch
  routing + per-call UUID sentinel.
- `org-resolve.spec.ts` — `getConfiguredDefaultTemplate` walk-up via
  monorepo markers.
- `utils.spec.ts` — `ensureGitignoreNodeModules` (fresh / append /
  no-newline / no-op / trailing-slash / CRLF / `node_modules/sub` /
  `!node_modules` cases).
- `migrator.spec.ts` — `injectCreateDefaultTemplate` (injects when
  scope is set, skips when empty, preserves an existing `create:`).
- `npm-config.spec.ts` (`packages/cli/src/utils/__tests__/`) —
  `.npmrc` precedence (project > user), scoped registry resolution,
  `_authToken` extraction.

The snap-tests use stubbed `fetch` for unit-level scenarios and the
mock registry for end-to-end scenarios. We do **not** publish a
dedicated `@voidzero-dev/create-test-fixture` package; registry-surface
regressions are low-frequency and can be patched downstream.

## CLI Help Output

The relevant additions to `vp create --help`:

```
Usage: vp create [TEMPLATE] [OPTIONS] [-- TEMPLATE_OPTIONS]

Arguments:
  TEMPLATE           Template to scaffold from. May be:
                       - an org scope (e.g. @your-org) for org templates
                       - an org entry (e.g. @your-org:web) for a specific
                         manifest entry
                       - any value accepted today: create-*, github:*, vite:*,
                         @scope/package, local package name
                     When omitted, uses `create.defaultTemplate` from
                     vite.config.ts if set.

Options:
  ...existing flags...

Configuration (vite.config.ts):
  create.defaultTemplate   Default org/template used by bare `vp create`.
```

## Compatibility

- **Org packages that already exist as single-template `@org/create`**: keep
  working unchanged until they opt in by adding `createConfig.templates`.
- **Plain `npm create @org` / `yarn create @org`**: unaffected. Those
  consumers run the package's `bin` script, which is outside Vite+'s scope.
- **Existing `@org/name` shorthand**: untouched. `vp create @org/foo`
  still expands to `@org/create-foo` exactly as it did before this
  feature. Manifest lookup is only triggered by the `:` separator
  (`vp create @org:foo`), so there's no collision with real
  `@org/anything` npm packages.

## Real-World Usage Examples

### Org with a published `@org/create` manifest

```bash
# Discovery
vp create @your-org
# → picker with: web, mobile, server, library

# Direct
vp create @your-org:server

# Non-interactive (CI)
vp create @your-org:library --no-interactive --directory ./packages/new-lib
```

### Enterprise monorepo with a default

```ts
// vite.config.ts at the company's template-seed repo
export default defineConfig({
  create: { defaultTemplate: '@your-org' },
});
```

```bash
# Inside that repo: engineers just type `vp create`
vp create
# → picker from @your-org/create, plus a trailing
#   "Vite+ built-in templates" entry for users who need vite:library etc.

# Explicit builtin (bypasses the configured default)
vp create vite:library
```

### Mixed-specifier manifest

```json
{
  "createConfig": {
    "templates": [
      { "name": "web", "description": "Next.js app", "template": "@your-org/template-web" },
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

- **Picker implementation**: plain `@voidzero-dev/vite-plus-prompts`
  `select` with prefix filtering. Upgrade to a fuzzy-search picker (e.g.
  the wrapper's `autocomplete`) in a follow-up if real usage reports
  friction.
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
  per `<host>/<scope>/<version>` into
  `$VP_HOME/tmp/create-org/<host>/<scope>/create/<version>/`, then
  scaffolds by directory copy. This lets an org ship N templates in a
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

### Decisions added during implementation

- **`@scope:name` (colon) as the manifest-entry separator**: not
  `@scope/name` (which collides with real npm package specifiers and
  the existing `@scope/create-name` shorthand). Mirrors the existing
  `vite:monorepo` / `vite:library` syntax for builtin templates.
- **`createConfig.templates` (not `vp.templates`)**: tool-neutral key
  name mirroring the existing `publishConfig` precedent. Other
  scaffolders can adopt the same convention without an opinionated
  `vp` namespace.
- **Pinned versions are honored**: `@scope@1.2.3` and
  `@scope:name@next` resolve through `dist-tags[...]` first then
  `versions[...]`. Unknown versions are a hard error.
- **`.npmrc` registry + auth, retry on challenge**: the resolver layers
  user / project `.npmrc` with `npm_config_*` env vars and honors
  `@scope:registry=...` overrides. The first request goes anonymous;
  the resolver only sends the matching `_authToken` / `_auth` /
  username:\_password on a 401/403 challenge so public registries never
  see the token.
- **Reserved `__vp_` prefix on entry names**: schema validation rejects
  manifest names starting with `__vp_`. Internal sentinel values (e.g.
  the picker's escape-hatch UUID) live under that prefix and can never
  collide with a user-authored entry.
- **Registry-aware cache key**: cache path includes a
  `sanitizeHostForPath(<tarballUrl host>)` segment so two repos that
  resolve the same `<scope>@<version>` through different `.npmrc`
  scope mappings don't share a slot. Sanitization replaces
  Windows-illegal characters (`\ / : * ? " < > |` plus IPv6 brackets)
  with `_`.
- **Atomic extract with stale-staging cleanup**: tarballs extract into
  `<destDir>.tmp-<pid>-<timestamp>` and atomically rename into place;
  rename-races resolve the loser to a cache hit. Sibling staging dirs
  older than 24h are pruned at the start of each fresh extract.
- **Tar-entry mode preservation**: `gradlew`, `mvnw`, `bin/*`, and
  similar files keep their `0755` bits through the extract. `setuid`,
  `setgid`, and sticky bits are stripped — those have no place in a
  user-land scaffold.
- **`keywords` field dropped**: prototyped in early rounds but never
  consumed by the picker. Removed from the schema entirely (YAGNI)
  rather than left validated-but-unused.
- **`create.defaultTemplate` auto-injection is gated**: only fires
  when the user just scaffolded from `vp create @scope:<entry>` AND
  the entry is `monorepo: true`. Builtin `vp create vite:monorepo`
  with a scoped package name does NOT auto-inject — the scope there
  is just an npm-publish detail, not a template-org choice.
- **Git-init prompt unified across monorepo paths**: prompt + spawn
  live in `bin.ts`'s monorepo branch where `vite:monorepo` and
  bundled `@org` monorepos converge; both ask, both default to yes
  in non-interactive mode.
- **`.gitignore` always excludes `node_modules` after `git init`**:
  bundled `@org` templates may ship without a `.gitignore`. After
  `git init` succeeds, `ensureGitignoreNodeModules` either creates a
  fresh `node_modules\n` file or appends the line if missing, with
  CRLF/`node_modules/`/`!node_modules` edge cases handled. No-op when
  the line is already present.
- **`findWorkspaceRoot` stays monorepo-marker-only**: extending it to
  recognize `.git` was prototyped and reverted. Standalone repos with
  no monorepo markers don't get config walk-up — call sites either
  point at the right starting directory or accept the deferral.

## Conclusion

- `vp create @org` becomes a branded entry point backed by an org-owned
  manifest.
- Opt-in via a single `createConfig.templates` field in `@org/create`'s `package.json`.
- Adopt in a repo via `create: { defaultTemplate: '@org' }`.
- Zero-risk for existing `@org/create` publishers.
- Consistent with the `code-generator.md` dual-mode strategy.
