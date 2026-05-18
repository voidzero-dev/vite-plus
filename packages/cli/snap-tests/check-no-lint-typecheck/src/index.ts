// Includes a no-eval violation to verify that `--type-check-only` suppresses
// lint rules while still running type-check. The file itself is type-correct.
function run(code: string): string {
  eval(code);
  return code;
}

export { run };
