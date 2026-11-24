# json

## `node verify-json.mjs`

```
Lint exited with status 1 and valid JSON:
[
  {
    "message": "eval can be harmful.",
    "code": "eslint(no-eval)",
    "severity": "error",
    "url": "https://oxc.rs/docs/guide/usage/linter/rules/eslint/no-eval.html",
    "help": "Avoid eval(). For JSON parsing use JSON.parse(); for dynamic property access use bracket notation (obj[key]); for other cases refactor to avoid evaluating strings as code.",
    "filename": "error.js",
    "labels": [
      {
        "span": {
          "offset": 0,
          "length": 4,
          "line": 1,
          "column": 1
        }
      }
    ]
  }
]
```
