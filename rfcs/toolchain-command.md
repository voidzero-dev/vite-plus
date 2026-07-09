# RFC: Vite+ Toolchain Inspection Command

- Status: Proposed
- Related: [why-package-command.md](./why-package-command.md),
  [packages/core/BUNDLING.md](../packages/core/BUNDLING.md),
  [packages/cli/BUNDLING.md](../packages/cli/BUNDLING.md),
  [docs/guide/upgrade.md](../docs/guide/upgrade.md)

## Summary

Add a top-level `vp toolchain` command that reports the exact tools and engines
in the active Vite+ release:

```bash
vp toolchain
vp toolchain vite
vp toolchain vite rolldown oxc
vp toolchain --json
vp toolchain --global
```

The `vite-plus` package ships a static toolchain manifest. The command reads
that file without invoking a package manager, executing dependency code, or
accessing the network.

`vp why` keeps its package-manager behavior. For human-readable queries that
match a manifest entry, it points users to `vp toolchain`.

## Motivation

Vite+ pins the tools that `vp build`, `vp test`, and `vp check` compose. Peer
dependency resolution must not change those versions between projects.

Package managers cannot inspect the full toolchain:

- `@voidzero-dev/vite-plus-core` bundles Vite, Rolldown, and tsdown.
- Vite+ compiles Rolldown's native binding into its native addon.
- Oxc and other Rust engines may have no installed npm package.
- `pnpm why`, `npm explain`, Yarn, and Bun describe the installed package graph.
- Resolving `vite/package.json` in a migrated project returns the Vite+ core
  alias. Its package version identifies Vite+, so it does not report the
  bundled upstream Vite version.

`vp --version` reports a flat summary. `vite-plus/versions` exposes the same
major versions to JavaScript. Neither surface shows ownership, composition,
Oxc, or Vite Task.

To check whether a project can use a new transform, a maintainer may need:

1. the Vite version that exposes it,
2. the Rolldown and Oxc versions behind that Vite release, and
3. the Vite+ release that ships those versions.

Ship this provenance with each Vite+ release.

## Goals

- Report the exact toolchain selected for the current directory.
- Show how packages, bundled tools, and compiled engines relate to each other.
- Support focused queries for one or more tools.
- Include hidden versions that package managers cannot report.
- Provide versioned, machine-readable JSON.
- Work offline and without running a managed Node.js runtime.
- Generate `vp toolchain`, `vp --version`, and public exports from one manifest.

## Non-goals

- Replace `vp why` or reproduce package-manager dependency resolution.
- List every npm transitive dependency, Rust crate, optional peer, or platform
  binding package.
- Determine whether an upstream feature exists in a particular version.
- Fetch changelogs, release notes, commits, or registry metadata.
- Allow projects to override Vite+'s bundled tool versions.
- Change Vite+ tools to peer dependencies.
- Produce a software bill of materials.

## Manifest Scope

The toolchain manifest includes components whose versions affect Vite+ behavior
or compatibility:

1. Vite+ distribution packages, including `vite-plus` and
   `@voidzero-dev/vite-plus-core`.
2. User-facing tools invoked or composed by Vite+, including Vite, Rolldown,
   Vitest, Oxlint, Oxfmt, oxlint-tsgolint, tsdown, and Vite Task.
3. Bundled or compiled engines whose versions determine available behavior.
   Version 1 includes Oxc and Oxc Resolver.

The manifest excludes ordinary implementation dependencies such as terminal
formatting libraries, file globbers, and HTTP clients. It also excludes
platform-specific binding wrapper packages when their version is identical to
the logical tool they deliver.

`vp toolchain` uses this bounded graph. `vp why` and `vp list` cover the
installed npm graph.

Maintainers must update the manifest topology when Vite+ adds a user-facing
tool or a hidden engine with its own compatibility surface.

## Command Interface

```text
Usage: vp toolchain [OPTIONS] [TOOLS]...

Show versions and relationships in the active Vite+ toolchain

Arguments:
  [TOOLS]...  Filter by tool or package name

Options:
      --json    Output the toolchain graph as JSON
      --global  Inspect the global Vite+ toolchain
  -h, --help    Print help
```

With no positional arguments, the command prints the complete manifest scope.
Positional arguments filter the graph to one or more components.

Examples:

```bash
vp toolchain                       # Active local-first toolchain
vp toolchain vite                  # Vite and its ownership/engine chain
vp toolchain rolldown oxc          # Union of both matching branches
vp toolchain @voidzero-dev/vite-plus-core
vp toolchain --global              # Ignore the project's local vite-plus
vp toolchain vite --json           # Stable machine-readable result
```

The first version accepts exact names and declared aliases. It does not accept
globs.

## Source Resolution

By default, `vp toolchain` follows normal local-first routing:

1. Use the installed local `vite-plus` resolved for the current directory.
2. If routing finds no local package, use the Vite+ package paired with the
   running global `vp`.

The output names the selected source. `--global` skips local resolution.

The global binary delegates the full invocation to the selected local Vite+
package. It reads its own manifest only when routing finds no local package or
the user passes `--global`.

The project lockfile cannot describe code bundled into core or crates compiled
into the native addon. It may also contain unrelated copies of Vite, Rolldown,
or Oxc, so the command does not use it as release provenance.

## Human-readable Output

The command renders an ownership tree with relationship labels:

```text
Vite+ toolchain (local)

vite-plus@0.2.4
|-- depends on @voidzero-dev/vite-plus-core@0.2.4
|   |-- bundles vite@8.1.3
|   |   `-- uses rolldown@1.1.4
|   |-- bundles rolldown@1.1.4
|   |   |-- compiles oxc@0.138.0
|   |   `-- compiles oxc-resolver@11.22.0
|   `-- bundles tsdown@0.22.3
|-- depends on vitest@4.1.10
|-- depends on oxlint@1.72.0
|-- depends on oxlint-tsgolint@0.24.0
|-- depends on oxfmt@0.57.0
`-- compiles vite-task@<version> (<revision>)
```

These versions reflect the repository at the time of writing. They do not form
part of the command contract.

The human tree may repeat a shared node to show two relationships. JSON uses one
entry per node ID.

### Filtered output

For each filter, the command keeps:

- every ownership ancestor needed to explain how the matched component is
  provided, and
- downstream `uses` and `compiles` relationships needed to expose its engine
  chain.

For example:

```text
$ vp toolchain vite

Vite+ toolchain (local)

vite-plus@0.2.4
`-- depends on @voidzero-dev/vite-plus-core@0.2.4
    `-- bundles vite@8.1.3
        `-- uses rolldown@1.1.4
            |-- compiles oxc@0.138.0
            `-- compiles oxc-resolver@11.22.0
```

For multiple filters, the command returns the union of those nodes and edges.

### Name matching

Filters match a node's:

- stable ID,
- canonical package or tool name, or
- declared alias.

Initial aliases include:

| Query            | Node                           |
| ---------------- | ------------------------------ |
| `vite-plus-core` | `@voidzero-dev/vite-plus-core` |
| `tsgolint`       | `oxlint-tsgolint`              |
| `vite-task`      | Vite Task                      |
| `oxc-resolver`   | Oxc Resolver                   |

Package and tool names remain case-sensitive, matching npm and Cargo naming.

An unknown filter exits with status 1:

```text
error: `rollup` is not part of the Vite+ toolchain manifest
hint: run `vp why rollup` to inspect project dependencies
```

For close matches, the error may suggest a manifest name.

## JSON Output

With `--json`, the command omits the Vite+ header, styling, and hints. It writes
one JSON object:

```json
{
  "schemaVersion": 1,
  "source": {
    "scope": "local",
    "path": "/project/node_modules/vite-plus",
    "vitePlusVersion": "0.2.4"
  },
  "nodes": [
    {
      "id": "vite-plus",
      "name": "vite-plus",
      "version": "0.2.4",
      "kind": "package",
      "delivery": ["dependency"],
      "aliases": []
    },
    {
      "id": "vite-plus-core",
      "name": "@voidzero-dev/vite-plus-core",
      "version": "0.2.4",
      "kind": "package",
      "delivery": ["dependency"],
      "aliases": ["vite-plus-core"]
    },
    {
      "id": "vite",
      "name": "vite",
      "version": "8.1.3",
      "kind": "tool",
      "delivery": ["bundled"],
      "aliases": []
    },
    {
      "id": "rolldown",
      "name": "rolldown",
      "version": "1.1.4",
      "kind": "tool",
      "delivery": ["bundled", "compiled"],
      "aliases": []
    },
    {
      "id": "oxc",
      "name": "oxc",
      "version": "0.138.0",
      "kind": "engine",
      "delivery": ["compiled"],
      "aliases": []
    }
  ],
  "edges": [
    {
      "from": "vite-plus",
      "to": "vite-plus-core",
      "relationship": "depends-on"
    },
    {
      "from": "vite-plus-core",
      "to": "vite",
      "relationship": "bundles"
    },
    {
      "from": "vite",
      "to": "rolldown",
      "relationship": "uses"
    },
    {
      "from": "rolldown",
      "to": "oxc",
      "relationship": "compiles"
    }
  ]
}
```

Node fields:

| Field      | Meaning                                                   |
| ---------- | --------------------------------------------------------- |
| `id`       | Stable identifier referenced by edges and filters         |
| `name`     | Canonical package, tool, or engine name                   |
| `version`  | Exact resolved version                                    |
| `revision` | Optional exact source revision for git-sourced components |
| `kind`     | `package`, `tool`, or `engine`                            |
| `delivery` | One or more of `dependency`, `bundled`, or `compiled`     |
| `aliases`  | Additional accepted filter names                          |

Schema version 1 defines these edge relationships:

- `depends-on`: shipped as a Vite+ package dependency,
- `bundles`: source or JavaScript output merged into another package,
- `uses`: runtime composition without ownership,
- `compiles`: linked into the Vite+ native addon.

The renderer emits nodes and edges in manifest order. Consumers must address
nodes by ID.

Breaking JSON changes increment `schemaVersion`. Adding optional fields, nodes,
edges, aliases, or enum values is non-breaking.

## Published Toolchain Manifest

The CLI package build writes:

```text
packages/cli/dist/toolchain.json
packages/cli/dist/toolchain.js
packages/cli/dist/toolchain.d.ts
```

`vite-plus` exports a typed JavaScript form:

```json
{
  "./toolchain": {
    "types": "./dist/toolchain.d.ts",
    "default": "./dist/toolchain.js"
  }
}
```

The exported object contains the release graph. The CLI adds the runtime
`source` object and installation path.

The build also derives the existing `vite-plus/versions` export from the
manifest and preserves its current keys. The build and both version commands
then share one version list.

### Version sources

The build resolves versions from:

| Component type                | Source                                                        |
| ----------------------------- | ------------------------------------------------------------- |
| `vite-plus` and core packages | Their generated `package.json` files                          |
| Bundled JS tools              | Core `bundledVersions` generated during the core build        |
| Managed npm tools             | Resolved dependency `package.json` files                      |
| Compiled Rust tools/engines   | `cargo metadata --locked --format-version 1` and `Cargo.lock` |
| Git-sourced Rust components   | Cargo package version plus the exact resolved revision        |

Maintainers define graph topology and aliases in a small source-controlled
descriptor. The generator fills version and revision fields from the sources
above.

Release builds fail when:

- the generator cannot resolve a required node,
- a required node has no exact version,
- an edge references an unknown node,
- node IDs or aliases conflict, or
- the generated flat `versions` export disagrees with the graph.

At runtime, `vp toolchain` reads the generated artifact. It does not parse
repository source files or run Cargo in an installed project.

## Older Local Vite+ Releases

Local-first routing sends `vp toolchain` to the selected local Vite+ package. A
local release that predates this command rejects it as an unknown command and
exits nonzero.

The global CLI does not reconstruct a partial graph from older package
metadata. Users can upgrade the local Vite+ release or run
`vp toolchain --global` to inspect the global release.

## Relationship to `vp --version`

`vp --version` keeps its concise environment summary:

- global `vp` version,
- local `vite-plus` version,
- major tool versions,
- package manager, and
- Node.js.

It reads tool rows from the manifest. `vp toolchain` handles filtering,
relationships, and engine details.

## Relationship to `vp why`

`vp why` delegates to the detected package manager with its existing arguments,
output, and exit status. It explains the installed package graph.

After a successful human-readable query, Vite+ checks each name against the
active toolchain manifest. A match adds one hint:

```text
Vite+ also provides vite@8.1.3 through its toolchain.
Run `vp toolchain vite` to inspect its bundled version and relationships.
```

The hint says "also provides" because a project may also install upstream Vite.
Vite+ leaves package-manager output unchanged and omits the hint for failed or
machine-readable queries. One hint includes all matching names.

## Implementation

### Manifest generation

Extend `packages/cli/build.ts` so the versions-export step generates the
toolchain graph, then derives `versions.js` and its type declaration from it.

Core generates `bundledVersions` while it builds Vite, Rolldown, and tsdown.
The CLI generator combines that metadata with resolved npm packages and Cargo
metadata.

### Command implementation

Shared Rust code handles parsing, filtering, and rendering for both the global
CLI and local NAPI-backed CLI.

Place the top-level command beside other Vite+ inspection and lifecycle
commands. `vite_pm_cli` owns commands that invoke a package manager.

Without `--global`, the global binary delegates to a selected local Vite+
package. The local package handles the command through its NAPI binding. With
`--global`, or when no local package exists, the global implementation reads
the global package's static manifest without starting Node.js.

The Rust `--version` implementation reads the shared manifest and removes its
hardcoded `TOOL_SPECS` table.

### Documentation

Add `vp toolchain` to:

- top-level CLI help,
- the interactive command picker,
- `README.md` and `packages/cli/README.md`,
- the guide command overview,
- upgrade and troubleshooting documentation, and
- generated project agent guidance when version inspection is discussed.

Documentation describes `vp why` as a package-manager operation.

## Testing

### Unit tests

- Manifest generation resolves all required npm and Cargo nodes.
- Invalid IDs, aliases, edges, versions, and revisions fail generation.
- The build derives `vite-plus/versions` from the graph and checks every key.
- Exact name and alias filters resolve the expected nodes.
- Filtering retains ownership ancestors and downstream engine edges.
- Multiple filters produce a stable union without duplicate JSON nodes.
- Human rendering uses stable ordering for shared nodes.
- Unknown filters return status 1 with the package-manager hint.

### CLI snapshot tests

New cases belong in `crates/vite_cli_snapshots/tests/cli_snapshots/`:

| Scenario                          | Expected coverage                                        |
| --------------------------------- | -------------------------------------------------------- |
| Full local manifest               | Complete tree and local source                           |
| `vp toolchain vite`               | Core, Vite, Rolldown, Oxc, and Oxc Resolver chain        |
| Multiple filters                  | Stable union of branches                                 |
| Alias filter                      | `vite-plus-core`, `vite-task`, and `tsgolint` resolution |
| `--json`                          | Valid JSON without header, styling, or trailing text     |
| No local package                  | Global source selection                                  |
| `--global` inside a local project | Global source forced                                     |
| Old local Vite+ package           | Unknown-command failure from the local CLI               |
| Unknown tool                      | Status 1 and `vp why` hint                               |
| `vp why vite`                     | Package-manager output followed by toolchain hint        |
| `vp why vite --json`              | Unmodified machine-readable package-manager output       |

Release artifact tests load the same manifest with each platform binding and
compare its native versions with the compiled release inputs.

## Performance and Security

- Runtime work resolves the selected `vite-plus` package, reads one JSON file,
  filters a small graph, and renders output.
- The command makes no network requests and executes no dependency code.
- The CLI reads the manifest from the selected `vite-plus` package. Manifest
  traversal does not accept arbitrary filesystem paths.
- The manifest contains public package versions and source revisions.

## Backward Compatibility

The new command does not change `vp why` flags or package-manager behavior.
Machine-readable output excludes the new human hint.

`vite-plus/versions` keeps its current flat shape. The release adds
`vite-plus/toolchain`.

## Alternatives Considered

### Extend `vp --version`

`vp --version` gives users a short environment summary. Graph filtering,
relationship labels, and JSON need their own command.

### Name the command `vp versions`

`versions` omits the ownership relationship and overlaps with `vp env list`,
which manages Node.js versions.

### Name the command `vp deps` or `vp tree`

Both names suggest the installed project graph. `toolchain` identifies
Vite+-owned release metadata.

### Change `vp why` to synthesize bundled nodes

`vp why` promises package-manager dependency analysis. Synthetic nodes would
change its normal and JSON contracts. A human-only pointer keeps those
contracts intact.

### Read package manifests at runtime

Runtime package reads recover Vite, Rolldown, tsdown, and managed npm tools.
They cannot recover compiled Oxc or Vite Task inputs, and they duplicate the
generated manifest logic.

### Query GitHub or the npm registry

Remote lookups fail offline and describe registry metadata. The installed
manifest describes the artifact on disk.

### Expose all Cargo and npm transitive dependencies

A full transitive graph would duplicate package-manager and SBOM tools. The
manifest includes components whose versions affect Vite+ behavior.

### Use peer dependencies for bundled tools

Peer dependencies would let project resolution change Vite+ runtime behavior.
The inspection command fixes the visibility problem without changing version
ownership.

## Rollout

1. Generate and publish the toolchain manifest and `vite-plus/toolchain` export.
2. Derive `vite-plus/versions` and `vp --version` tool rows from the manifest.
3. Add `vp toolchain`, filtering, and JSON output.
4. Add the human-readable `vp why` discovery hint.
5. Update product documentation and generated agent guidance.
