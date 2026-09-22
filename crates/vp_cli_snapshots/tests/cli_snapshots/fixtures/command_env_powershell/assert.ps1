$ErrorActionPreference = "Stop"

$expectedBin = Join-Path $env:EXPECTED_VP_HOME "bin"
$expectedFallback = Join-Path $env:EXPECTED_VP_HOME "fallback-bin"
$externalBin = Join-Path $PWD "external-node"
New-Item -ItemType Directory -Path $externalBin | Out-Null
$node = Get-Command node -CommandType Application | Select-Object -First 1
Copy-Item -LiteralPath $node.Source -Destination $externalBin
$externalNode = Join-Path $externalBin (Split-Path $node.Source -Leaf)
$env:PATH = (@($expectedFallback.ToUpperInvariant(), $externalBin, $expectedBin.ToUpperInvariant(), $env:PATH, $expectedBin, $expectedFallback)) -join [IO.Path]::PathSeparator
if ((Get-Command node -CommandType Application | Select-Object -First 1).Source -ne $externalNode) {
    throw "Fixture did not create a Node PATH priority conflict"
}

# Repeated activation must put one shim entry first, ahead of the external Node.
. (Join-Path $env:EXPECTED_VP_HOME "env.ps1")
. (Join-Path $env:EXPECTED_VP_HOME "env.ps1")

if ($env:VP_HOME -ne $env:EXPECTED_VP_HOME) {
    throw "VP_HOME mismatch: expected $env:EXPECTED_VP_HOME, got $env:VP_HOME"
}

$binCount = @($env:PATH -split [IO.Path]::PathSeparator | Where-Object { $_ -ieq $expectedBin }).Count
if ($binCount -ne 1) {
    throw "PATH contains the Vite+ bin directory $binCount times"
}
if (($env:PATH -split [IO.Path]::PathSeparator)[0] -ne $expectedBin) {
    throw "Activation did not put the shim directory first on PATH"
}
$fallbackCount = @($env:PATH -split [IO.Path]::PathSeparator | Where-Object { $_ -ieq $expectedFallback }).Count
if ($fallbackCount -ne 1 -or ($env:PATH -split [IO.Path]::PathSeparator)[-1] -ne $expectedFallback) {
    throw "Activation did not put exactly one fallback directory last on PATH"
}
if ((Get-Command node -CommandType Application | Select-Object -First 1).Source -ne (Join-Path $expectedBin (Split-Path $node.Source -Leaf))) {
    throw "Activation did not give the Node shim priority"
}

if (-not (Get-Command vp -CommandType Function -ErrorAction SilentlyContinue)) {
    throw "env.ps1 did not define the vp wrapper"
}

$vpOutput = vp --version
if ($LASTEXITCODE -ne 0) {
    throw "vp --version failed through the PowerShell wrapper"
}
if ([string]::IsNullOrWhiteSpace(($vpOutput -join ""))) {
    throw "vp --version returned no output"
}

$env:VP_NODE_VERSION = "18.20.0"
vp env use --help *> $null
if ($LASTEXITCODE -ne 0) {
    throw "vp env use --help failed through the PowerShell wrapper"
}
if ($env:VP_NODE_VERSION -ne "18.20.0") {
    throw "vp env use --help changed VP_NODE_VERSION"
}

vp env use 20.18.0 --no-install
if ($LASTEXITCODE -ne 0) {
    throw "vp env use failed through the PowerShell wrapper"
}
if ($env:VP_NODE_VERSION -ne "20.18.0") {
    throw "VP_NODE_VERSION mismatch: expected 20.18.0, got $env:VP_NODE_VERSION"
}
if ($ErrorActionPreference -ne 'Stop' -or (Test-Path Env:VP_ENV_USE_EVAL_ENABLE) -or (Test-Path Env:VP_SHELL)) {
    throw "vp env use did not restore the caller's preferences and environment"
}

$env:VP_ENV_USE_EVAL_ENABLE = 'original'
$env:VP_SHELL = 'powershell'
vp env use invalid-version --no-install 6>$null
if ($LASTEXITCODE -eq 0 -or $env:VP_NODE_VERSION -ne '20.18.0') {
    throw "Failed vp env use changed the selected Node version or lost its exit code"
}
if ($ErrorActionPreference -ne 'Stop' -or $env:VP_ENV_USE_EVAL_ENABLE -ne 'original' -or $env:VP_SHELL -ne 'powershell') {
    throw "Failed vp env use did not restore the caller's preferences and environment"
}

# A terminating error while displaying native stderr must also restore the environment.
& {
    function Write-Host { throw 'Simulated host output failure' }
    $caught = $false
    try {
        vp env use 20.18.0 --no-install
    } catch {
        if ($_.Exception.Message -notlike '*Simulated host output failure*') { throw }
        $caught = $true
    }
    if (-not $caught) { throw 'The wrapper did not propagate the host output failure' }
}
if ($ErrorActionPreference -ne 'Stop' -or $env:VP_ENV_USE_EVAL_ENABLE -ne 'original' -or $env:VP_SHELL -ne 'powershell') {
    throw "Interrupted vp env use did not restore the caller's preferences and environment"
}
Remove-Item Env:VP_ENV_USE_EVAL_ENABLE, Env:VP_SHELL

$ErrorActionPreference = 'Continue'
vp env use --unset
if ($LASTEXITCODE -ne 0) {
    throw "vp env use --unset failed through the PowerShell wrapper"
}
if (Test-Path Env:VP_NODE_VERSION) {
    throw "vp env use --unset did not remove VP_NODE_VERSION"
}
if ($ErrorActionPreference -ne 'Continue' -or (Test-Path Env:VP_ENV_USE_EVAL_ENABLE) -or (Test-Path Env:VP_SHELL)) {
    throw "vp env use --unset did not restore the caller's preferences and environment"
}
$ErrorActionPreference = 'Stop'

vp env use --no-install
if ($LASTEXITCODE -ne 0) {
    throw "vp env use without a version failed through the PowerShell wrapper"
}
if ($env:VP_NODE_VERSION -ne "22.18.0") {
    throw "file-based VP_NODE_VERSION mismatch: expected 22.18.0, got $env:VP_NODE_VERSION"
}

Write-Output "PowerShell environment checks passed"
