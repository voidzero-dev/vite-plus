# check_no_color

## `vp check --no-fmt`

Captured lint diagnostics and Vite+ messages honor NO_COLOR in a terminal

**Exit code:** 1

```
error: Lint issues found
x eslint(no-eval): eval can be harmful.
   ,-[src/index.js:2:3]
 1 | function hello() {
 2 |   eval(\"code\");
   :   ^^^^
 3 |   console.log(\"warning\");
   `----
  help: Avoid eval(). For JSON parsing use JSON.parse(); for dynamic property access use bracket notation (obj[key]); for other cases refactor to avoid evaluating strings as code.

  ! eslint(no-console): Unexpected console statement.
   ,-[src/index.js:3:3]
 2 |   eval(\"code\");
 3 |   console.log(\"warning\");
   :   ^^^^^^^^^^^
 4 |   return \"hello\";
   `----
  help: Delete this console statement.

Found 1 error and 1 warning in 2 files (<duration>, <n> threads)
```
