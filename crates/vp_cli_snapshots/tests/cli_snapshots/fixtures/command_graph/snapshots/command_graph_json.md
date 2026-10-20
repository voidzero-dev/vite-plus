# command_graph_json

Prints the same graph as JSON for scripts and agents.

## `vp graph --json`

```
{
  "packages": [
    {
      "name": "monorepo",
      "path": ".",
      "dependencies": []
    },
    {
      "name": "@acme/web",
      "path": "apps/web",
      "dependencies": [
        {
          "name": "@acme/ui",
          "type": "normal"
        },
        {
          "name": "@acme/utils",
          "type": "dev"
        }
      ]
    },
    {
      "name": "@acme/ui",
      "path": "packages/ui",
      "dependencies": [
        {
          "name": "@acme/utils",
          "type": "normal"
        }
      ]
    },
    {
      "name": "@acme/utils",
      "path": "packages/utils",
      "dependencies": []
    }
  ]
}
```
