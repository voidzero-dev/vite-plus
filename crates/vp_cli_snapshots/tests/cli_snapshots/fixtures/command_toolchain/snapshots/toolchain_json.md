# toolchain_json

## `vp toolchain vite --json --global`

JSON contains one node per ID and no CLI header

```
{
  "schemaVersion": 1,
  "source": {
    "scope": "global",
    "path": "<home>/.vite-plus/current/node_modules/vite-plus",
    "vitePlusVersion": "<version>"
  },
  "nodes": [
    {
      "id": "vite-plus",
      "name": "vite-plus",
      "version": "<version>",
      "kind": "package",
      "delivery": [
        "dependency"
      ],
      "aliases": []
    },
    {
      "id": "vite-plus-core",
      "name": "@voidzero-dev/vite-plus-core",
      "version": "<version>",
      "kind": "package",
      "delivery": [
        "dependency"
      ],
      "aliases": [
        "vite-plus-core"
      ]
    },
    {
      "id": "vite",
      "name": "vite",
      "version": "<version>",
      "kind": "tool",
      "delivery": [
        "bundled"
      ],
      "aliases": []
    },
    {
      "id": "rolldown",
      "name": "rolldown",
      "version": "<version>",
      "kind": "tool",
      "delivery": [
        "bundled",
        "compiled"
      ],
      "aliases": []
    },
    {
      "id": "oxc",
      "name": "oxc",
      "version": "<version>",
      "kind": "engine",
      "delivery": [
        "compiled"
      ],
      "aliases": []
    },
    {
      "id": "oxc-resolver",
      "name": "oxc-resolver",
      "version": "<version>",
      "kind": "engine",
      "delivery": [
        "compiled"
      ],
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
    },
    {
      "from": "rolldown",
      "to": "oxc-resolver",
      "relationship": "compiles"
    }
  ]
}
```
