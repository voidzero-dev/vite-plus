# The PowerShell script binder consumes a bare -- before populating $args.
# Exercise the generated wrapper with a real native child, without a registry.
function Assert-Arguments($actual, $expected) {
    if ($LASTEXITCODE -ne 0 -or ($actual -join "`n") -ne $expected) {
        throw "Expected arguments $expected, got $actual (exit $LASTEXITCODE)"
    }
}

Assert-Arguments (vp env exec node print-args.cjs -- --template react-ts) '["--","--template","react-ts"]'
Assert-Arguments (vp env exec node print-args.cjs '--' --template react-ts) '["--","--template","react-ts"]'
Assert-Arguments (vp env exec node print-args.cjs "--" --template react-ts) '["--","--template","react-ts"]'
Assert-Arguments (vp env exec node print-args.cjs '--' -- --template react-ts) '["--","--","--template","react-ts"]'
Assert-Arguments (vp env exec node print-args.cjs -- -- --template react-ts) '["--","--","--template","react-ts"]'
Assert-Arguments (vp env exec node print-args.cjs --) '["--"]'
Assert-Arguments (vp env exec node print-args.cjs 'value with spaces' -- --template 'react ts') '["value with spaces","--","--template","react ts"]'

$prefix = @('env', 'exec', 'node', 'print-args.cjs')
$suffix = @('--template', 'react-ts')
$allArguments = $prefix + @('--') + $suffix
Assert-Arguments (vp @prefix -- --template react-ts) '["--","--template","react-ts"]'
Assert-Arguments (vp env exec node print-args.cjs -- @suffix) '["--","--template","react-ts"]'
Assert-Arguments (vp @prefix -- @suffix) '["--","--template","react-ts"]'
Assert-Arguments (vp @allArguments) '["--","--template","react-ts"]'
$emptyArguments = @()
Assert-Arguments (vp @prefix @emptyArguments -- @emptyArguments @suffix) '["--","--template","react-ts"]'
Assert-Arguments (vp env exec node print-args.cjs @('before', 'separator') -- @('after', 'separator')) '["before","separator","--","after","separator"]'

# The caller AST must identify the invocation inside a function or on a later line.
function Invoke-ArgumentCheck {
    Assert-Arguments (vp @prefix -- @suffix) '["--","--template","react-ts"]'
}
Invoke-ArgumentCheck
Assert-Arguments (& vp env exec node print-args.cjs `
    -- `
    --template react-ts) '["--","--template","react-ts"]'
Assert-Arguments (vp env exec node print-args.cjs first -- second | ForEach-Object { $_ }) '["first","--","second"]'

# Recovering the separator must not evaluate argument expressions a second time.
$script:argumentEvaluations = 0
Assert-Arguments (vp env exec node print-args.cjs -- $($script:argumentEvaluations++; 'value with spaces')) '["--","value with spaces"]'
if ($script:argumentEvaluations -ne 1) { throw 'An argument expression ran more than once' }

Assert-Arguments (vp env exec node print-args.cjs 'text -- with spaces' --template react-ts) '["text -- with spaces","--template","react-ts"]'
Assert-Arguments (vp env exec node print-args.cjs `-- --template react-ts) '["--","--template","react-ts"]'
Assert-Arguments (vp env exec node print-args.cjs --template react-ts) '["--template","react-ts"]'
