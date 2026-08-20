$ErrorActionPreference = "Stop"

. (Join-Path $env:EXPECTED_VP_HOME "env.ps1")

if ($env:VP_HOME -ne $env:EXPECTED_VP_HOME) {
    throw "VP_HOME mismatch: expected $env:EXPECTED_VP_HOME, got $env:VP_HOME"
}

$expectedBin = Join-Path $env:EXPECTED_VP_HOME "bin"
$binCount = @($env:Path -split [IO.Path]::PathSeparator | Where-Object { $_ -ieq $expectedBin }).Count
if ($binCount -ne 1) {
    throw "PATH contains the Vite+ bin directory $binCount times"
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

vp env use --unset
if ($LASTEXITCODE -ne 0) {
    throw "vp env use --unset failed through the PowerShell wrapper"
}
if (Test-Path Env:VP_NODE_VERSION) {
    throw "vp env use --unset did not remove VP_NODE_VERSION"
}

vp env use --no-install
if ($LASTEXITCODE -ne 0) {
    throw "vp env use without a version failed through the PowerShell wrapper"
}
if ($env:VP_NODE_VERSION -ne "22.18.0") {
    throw "file-based VP_NODE_VERSION mismatch: expected 22.18.0, got $env:VP_NODE_VERSION"
}

Write-Output "PowerShell environment checks passed"
