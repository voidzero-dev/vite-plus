# toolchain_json

## `vp toolchain vite --json --global`

JSON contains one node per ID and no human header

```
{
  "schemaVersion": 1,
  "source": {
    "scope": "global",
    "path": "<home>/.vite-plus/current/node_modules/vite-plus",
    "vitePlusVersion": "0.2.8"
  },
  "nodes": [
    {
      "id": "vite-plus",
      "name": "vite-plus",
      "version": "0.2.8",
      "kind": "package",
      "delivery": [
        "dependency"
      ],
      "aliases": []
    },
    {
      "id": "vite-plus-core",
      "name": "@voidzero-dev/vite-plus-core",
      "version": "0.2.8",
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
      "version": "8.2.0",
      "kind": "tool",
      "delivery": [
        "bundled"
      ],
      "aliases": []
    },
    {
      "id": "rolldown",
      "name": "rolldown",
      "version": "1.2.2",
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
      "version": "0.142.0",
      "kind": "engine",
      "delivery": [
        "compiled"
      ],
      "aliases": []
    },
    {
      "id": "oxc-resolver",
      "name": "oxc-resolver",
      "version": "11.24.2",
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
