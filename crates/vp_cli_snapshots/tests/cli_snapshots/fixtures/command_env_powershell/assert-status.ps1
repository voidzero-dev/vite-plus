# A native failure must survive the wrapper boundary without exiting this shell.
# Continue permits redirecting native stderr on Windows PowerShell 5.1.
$ErrorActionPreference = 'Continue'
vp --definitely-invalid-option *> $null
if ($? -or $LASTEXITCODE -ne 2) {
    throw "vp lost the native failure status or exit code"
}

# Success must clear a preceding failure, including when stdout is captured.
# -V must reach vp, rather than being consumed as PowerShell's -Verbose alias.
$versionOutput = vp -V
if (-not $? -or $LASTEXITCODE -ne 0 -or ($versionOutput -join "`n") -notmatch 'vp v') {
    throw "vp -V lost the native success status or output"
}

vp env use invalid-version --no-install --help *> $null
if (-not $? -or $LASTEXITCODE -ne 0) {
    throw "vp env use --help lost the native success status"
}
vp -C does-not-exist env use --help *> $null
if ($? -or $LASTEXITCODE -eq 0) {
    throw "vp env use --help lost a native directory error"
}

$projectDir = Join-Path $PWD 'project with spaces'
New-Item -ItemType Directory -Path $projectDir | Out-Null
Set-Content -LiteralPath (Join-Path $projectDir '.node-version') -Value '22.18.0'
vp -C $projectDir env use --no-install *> $null
if (-not $? -or $LASTEXITCODE -ne 0 -or $env:VP_NODE_VERSION -ne '22.18.0') {
    throw "vp -C did not forward a directory containing spaces"
}

# The script alias must retain dynamic completion.
$completion = TabExpansion2 'vp en' 5
if ('env' -notin $completion.CompletionMatches.CompletionText) {
    throw "vp lost its native argument completer"
}

# Evaluate strings so this fixture also parses under Windows PowerShell 5.1.
if ($PSVersionTable.PSVersion.Major -ge 7) {
    $andRan = $false
    Invoke-Expression 'vp --definitely-invalid-option 2>$null && ($andRan = $true)' | Out-Null
    if ($andRan) { throw '&& continued after a failed vp command' }
    $orRan = $false
    Invoke-Expression 'vp --definitely-invalid-option 2>$null || ($orRan = $true)' | Out-Null
    if (-not $orRan) { throw '|| did not continue after a failed vp command' }

    $andRan = $false
    Invoke-Expression 'vp env use invalid-version --no-install 6>$null && ($andRan = $true)' | Out-Null
    if ($andRan) { throw '&& continued after a failed vp env use' }
    $orRan = $false
    Invoke-Expression 'vp env use invalid-version --no-install 6>$null || ($orRan = $true)' | Out-Null
    if (-not $orRan) { throw '|| did not continue after a failed vp env use' }

    $andRan = $false
    Invoke-Expression 'vp --version >$null && ($andRan = $true)' | Out-Null
    if (-not $andRan) { throw '&& did not continue after a successful vp command' }
    $orRan = $false
    Invoke-Expression 'vp --version >$null || ($orRan = $true)' | Out-Null
    if ($orRan) { throw '|| continued after a successful vp command' }
}
$ErrorActionPreference = 'Stop'
