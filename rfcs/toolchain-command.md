# RFC: Vite+ Toolchain Inspection Command

- Status: Proposed
- Related: [why-package-command.md](./why-package-command.md),
  [packages/core/BUNDLING.md](../packages/core/BUNDLING.md),
  [packages/cli/BUNDLING.md](../packages/cli/BUNDLING.md),
  [docs/guide/upgrade.md](../docs/guide/upgrade.md)

## Summary

Add a top-level `vp toolchain` command. It shows the exact tools and engines in
the active Vite+ release:

```bash
vp toolchain
vp toolchain vite
vp toolchain vite rolldown oxc
vp toolchain --json
vp toolchain --global
```

The `vite-plus` package includes a static toolchain manifest. The command reads
this file. It does not run a package manager or dependency code. It does not use
the network.

`vp why` keeps its package-manager behavior. For readable output, it checks each
query against the manifest. If a query matches, it shows a `vp toolchain` hint.

## Motivation

Vite+ pins the tools that `vp build`, `vp test`, and `vp check` use. A project's
peer dependencies must not change these versions.

Package managers cannot show the full toolchain:

- `@voidzero-dev/vite-plus-core` bundles Vite, Rolldown, and tsdown.
- Vite+ compiles Rolldown's native binding into its native addon.
- Oxc and other Rust engines may have no installed npm package.
- `pnpm why`, `npm explain`, Yarn, and Bun describe the installed package graph.
- Resolving `vite/package.json` in a migrated project returns the Vite+ core
  alias. This package version identifies the Vite+ release. It does not identify
  the bundled Vite version.

`vp --version` shows a flat summary. `vite-plus/versions` gives the same major
versions to JavaScript. Neither output shows relationships, Oxc, or Vite Task.

To check whether a project can use a new transform, a maintainer may need:

1. the Vite version that exposes it,
2. the Rolldown and Oxc versions behind that Vite release, and
3. the Vite+ release that ships those versions.

Each Vite+ release must include this version information.

## Goals

- Show the exact toolchain selected for the current directory.
- Show how packages, bundled tools, and compiled engines relate to each other.
- Support focused queries for one or more tools.
- Include hidden versions that package managers cannot show.
- Provide JSON with a schema version.
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

The toolchain manifest includes components that affect Vite+ behavior or
compatibility:

1. Vite+ distribution packages, including `vite-plus` and
   `@voidzero-dev/vite-plus-core`.
2. User-facing tools invoked or composed by Vite+, including Vite, Rolldown,
   Vitest, Oxlint, Oxfmt, oxlint-tsgolint, tsdown, and Vite Task.
3. Bundled or compiled engines whose versions affect tool behavior.
   Version 1 includes Oxc and Oxc Resolver.

The manifest excludes ordinary implementation dependencies. Examples include
terminal-formatting libraries, file globbers, and HTTP clients. The manifest
also excludes platform binding packages when they have the same version as the
tool that they provide.

`vp toolchain` uses this limited graph. Use `vp why` and `vp list` for the
installed npm graph.

Maintainers must update the graph when Vite+ adds a user-facing tool. They must
also update it when Vite+ adds a hidden engine that affects compatibility.

## Command Interface

```text
Usage: vp toolchain [OPTIONS] [TOOLS]...

Show active Vite+ tools, versions, and relationships

Arguments:
  [TOOLS]...  Tool or package names to show

Options:
      --json    Print the graph as JSON
      --global  Use the global Vite+ toolchain
  -h, --help    Print help
```

With no tool names, the command prints the complete graph. Tool names select
one or more parts of the graph.

Examples:

```bash
vp toolchain                       # Active local-first toolchain
vp toolchain vite                  # Vite and its ownership/engine chain
vp toolchain rolldown oxc          # Union of both matching branches
vp toolchain @voidzero-dev/vite-plus-core
vp toolchain --global              # Ignore the project's local vite-plus
vp toolchain vite --json           # Stable JSON result
```

Version 1 accepts exact names and defined aliases. It does not accept globs.

## Source Resolution

By default, `vp toolchain` follows normal local-first routing:

1. Use the installed local `vite-plus` resolved for the current directory.
2. If routing finds no local package, use the Vite+ package paired with the
   running global `vp`.

The output identifies the selected source. `--global` skips local resolution.

The global binary sends the full command to the selected local Vite+ package.
It reads the global manifest only when no local package exists. It also reads
the global manifest when the user passes `--global`.

The project lockfile cannot describe code bundled into core. It also cannot
describe crates compiled into the native addon. The lockfile can contain
unrelated copies of Vite, Rolldown, or Oxc. Thus, the command does not use the
lockfile as release version information.

## Readable Output

The command prints an ownership tree with relationship labels:

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
`-- compiles vite-task (built 2026-08-06T09:30:00Z, revision <revision>)
```

These versions show the repository state when this RFC was written. They are
not part of the command contract.

The readable tree can repeat a shared node to show two relationships. JSON has
one entry for each node ID.

### Filtered output

For each filter, the command keeps:

- each parent node that shows how Vite+ provides the matched component, and
- each downstream `uses` or `compiles` relationship in its engine chain.

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

Package and tool names are case-sensitive. This matches npm and Cargo names.

If a filter is unknown, the command exits with status 1:

```text
error: `rollup` is not in the Vite+ toolchain
hint: run `vp why rollup` to show project dependencies
```

For a close match, the error can suggest a name from the manifest.

## JSON Output

With `--json`, the command does not show the Vite+ header, styles, or hints. It
writes one JSON object:

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

| Field      | Meaning                                             |
| ---------- | --------------------------------------------------- |
| `id`       | Stable identifier used by edges and filters         |
| `name`     | Canonical package, tool, or engine name             |
| `version`  | Exact version, when available                       |
| `revision` | Exact source revision, when available               |
| `builtAt`  | UTC native build time when a version is not useful  |
| `kind`     | `package`, `tool`, or `engine`                      |
| `delivery` | One or more: `dependency`, `bundled`, or `compiled` |
| `aliases`  | Other filter names                                  |

Schema version 1 defines these edge relationships:

- `depends-on`: Vite+ ships the component as a package dependency.
- `bundles`: Vite+ merges the source or JavaScript output into another package.
- `uses`: A tool uses the component at runtime but does not own it.
- `compiles`: Vite+ links the component into the native addon.

The renderer writes nodes and edges in manifest order. Consumers must select
nodes by ID.

Increment `schemaVersion` for a breaking JSON change. Optional fields, nodes,
edges, aliases, and enum values do not require an increment.

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

The exported object contains the release graph. At runtime, the CLI adds the
`source` object and the installation path.

The build also creates the existing `vite-plus/versions` export from the
manifest. It keeps the current keys. The build and both version commands use
one version list.

### Version sources

The build reads versions from:

| Component type                | Source                                                           |
| ----------------------------- | ---------------------------------------------------------------- |
| `vite-plus` and core packages | Their generated `package.json` files                             |
| Bundled JS tools              | Core `bundledVersions` generated during the core build           |
| Managed npm tools             | Resolved dependency `package.json` files                         |
| Compiled Rust tools/engines   | `cargo metadata --locked --format-version 1` and `Cargo.lock`    |
| Git-sourced Rust components   | Exact revision and, after native compilation, its UTC build time |

Maintainers define the graph and aliases in a small source file. The native
build records its completion time. The manifest generator combines this time
with the versions and revisions above. A TypeScript-only build uses an existing
native timestamp. If no timestamp exists, the manifest shows only the revision.
`SOURCE_DATE_EPOCH` sets the timestamp for a reproducible build.

Release builds fail when:

- the generator cannot resolve a required node,
- a required node has no exact version, revision, or build time,
- an edge references an unknown node,
- node IDs or aliases conflict, or
- the generated flat `versions` export disagrees with the graph.

At runtime, `vp toolchain` reads the generated file. It does not read repository
source files. It does not run Cargo in an installed project.

## Older Local Vite+ Releases

Local-first routing sends `vp toolchain` to the selected local Vite+ package. An
old local release rejects the command and exits with a nonzero status.

The global CLI does not create a partial graph from old package data. Upgrade
the local Vite+ release to use the command. To show the global release, run
`vp toolchain --global`.

## Relationship to `vp --version`

`vp --version` keeps its concise environment summary:

- global `vp` version,
- local `vite-plus` version,
- major tool versions,
- package manager, and
- Node.js.

It reads tool rows from the manifest. Use `vp toolchain` to select tools and show
relationships or engine details.

## Relationship to `vp why`

`vp why` sends the command to the detected package manager. It keeps the
existing arguments, output, and exit status. It shows the installed package
graph.

After a successful query with readable output, Vite+ checks each name against
the active toolchain manifest. A match adds one hint:

```text
Vite+ also provides vite@8.1.3 through its toolchain.
Run `vp toolchain vite` to show this version and its relationships.
```

The hint says "also provides" because a project may also install upstream Vite.
Vite+ does not change the package-manager output. It does not show the hint for
a failed query. It also omits the hint for JSON or parseable output. One hint
includes all matching names.

## Implementation

### Manifest generation

Change `packages/cli/build.ts`. The versions-export step first generates the
toolchain graph. It then creates `versions.js` and its type declaration from the
graph.

Core generates `bundledVersions` while it builds Vite, Rolldown, and tsdown.
The CLI generator combines these versions with npm package data and Cargo data.

### Command implementation

Shared Rust code parses, filters, and renders the graph. The global CLI and the
local NAPI CLI use this code.

Place the top-level command with the other Vite+ version and lifecycle commands.
`vite_pm_cli` owns commands that run a package manager.

Without `--global`, the global binary sends the command to the selected local
Vite+ package. The local package runs the command through its NAPI binding. With
`--global`, the global implementation reads the static global manifest. It also
does this when no local package exists. It does not start Node.js.

The Rust `--version` implementation reads the shared manifest. It no longer
uses a hardcoded `TOOL_SPECS` table.

### Documentation

Add `vp toolchain` to:

- top-level CLI help,
- the interactive command picker,
- `README.md` and `packages/cli/README.md`,
- the guide command overview,
- upgrade and troubleshooting documentation, and
- generated project agent guidance that discusses tool versions.

The documentation states that `vp why` is a package-manager operation.

## Testing

### Unit tests

- Manifest generation resolves all required npm and Cargo nodes.
- Invalid IDs, aliases, edges, versions, and revisions fail generation.
- The build derives `vite-plus/versions` from the graph and checks every key.
- Exact name and alias filters resolve the expected nodes.
- Filtering retains ownership ancestors and downstream engine edges.
- Multiple filters produce a stable union without duplicate JSON nodes.
- Readable output uses a stable order for shared nodes.
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
| `vp why vite --json`              | Unmodified JSON package-manager output                   |

Release artifact tests load the same manifest with each platform binding. The
tests compare native versions with the compiled release inputs.

## Performance and Security

- The command resolves the selected `vite-plus` package.
- It reads one JSON file, filters a small graph, and writes the output.
- It does not use the network.
- It does not run dependency code.
- The CLI reads the manifest from the selected `vite-plus` package.
- The command does not use tool filters as filesystem paths.
- The manifest contains public package versions and source revisions.

## Backward Compatibility

The new command does not change `vp why` flags or package-manager behavior.
JSON output does not include the new hint.

`vite-plus/versions` keeps its current flat shape. The release adds
`vite-plus/toolchain`.

## Alternatives Considered

### Extend `vp --version`

`vp --version` gives users a short environment summary. It does not select
parts of the graph or show relationships. JSON output also needs a separate
command.

### Name the command `vp versions`

`versions` does not identify ownership. It also overlaps with `vp env list`,
which manages Node.js versions.

### Name the command `vp deps` or `vp tree`

Both names suggest the installed project graph. `toolchain` identifies release
data that Vite+ owns.

### Change `vp why` to synthesize bundled nodes

`vp why` shows package-manager dependency data. Synthetic nodes would change its
readable and JSON output. The package-manager output stays unchanged because
Vite+ prints the hint separately.

### Read package manifests at runtime

Runtime package reads can find Vite, Rolldown, tsdown, and managed npm tools.
They cannot find compiled Oxc or Vite Task inputs. They also duplicate the
manifest generator.

### Query GitHub or the npm registry

Remote lookups fail when the user is offline. They describe registry data, not
the installed files. The manifest describes the installed release.

### Expose all Cargo and npm transitive dependencies

A full transitive graph would duplicate package-manager and SBOM tools. The
manifest includes only components that affect Vite+ behavior.

### Use peer dependencies for bundled tools

Peer dependencies would let project resolution change Vite+ runtime behavior.
The command shows the versions without changing their ownership.

## Rollout

1. Generate and publish the toolchain manifest and `vite-plus/toolchain` export.
2. Derive `vite-plus/versions` and `vp --version` tool rows from the manifest.
3. Add `vp toolchain`, filtering, and JSON output.
4. Add the readable `vp why` hint.
5. Update product documentation and generated agent guidance.
