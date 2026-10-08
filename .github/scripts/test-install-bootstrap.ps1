# Run locally on Windows with ./.github/scripts/test-install-bootstrap.ps1; no registry or installed vp needed.
$ErrorActionPreference = 'Stop'
$source = Get-Content -LiteralPath (Join-Path $PSScriptRoot '../../packages/cli/install.ps1') -Raw
. ([scriptblock]::Create(($source -replace '(?m)^    Main\r?$', '')))
function Assert($Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

# Exercise the piped entry point without acquiring a payload.
function Test-InstallerEntryPoint {
    Assert ($ErrorActionPreference -eq 'Stop') 'Installer did not enable terminating errors'
    if ($entryPointFails) { throw 'Expected installer failure' }
}
$entryPointSource = $source -replace '(?m)^    Main\r?$', '    Test-InstallerEntryPoint'
try {
    foreach ($preference in @('Continue', 'Stop')) {
        foreach ($entryPointFails in @($false, $true)) {
            $ErrorActionPreference = $preference
            $caught = $false
            try {
                $entryPointSource | Invoke-Expression
            } catch {
                Assert ($_.Exception.Message -eq 'Expected installer failure') "Unexpected error: $_"
                $caught = $true
            }
            Assert ($caught -eq $entryPointFails) 'Installer failure was lost'
            Assert ($ErrorActionPreference -eq $preference) 'Installer changed the caller error preference'
        }
    }
} finally {
    $ErrorActionPreference = 'Stop'
}
function Exit-Installer { param([int]$Code = 1); $script:ExitCode = $Code; throw $script:InstallStopSignal }

$testRoot = Join-Path $env:TEMP "vite-bootstrap-test-$(Get-Random)"
$originalTemp = $env:TEMP
$originalCheck = $env:VP_SELF_SETUP_SUPPORT_CHECK
$originalPath = $env:Path
$originalRegistry = $env:NPM_CONFIG_REGISTRY
$originalVpShell = $env:VP_SHELL
$fixtureSha = '0123456789012345678901234567890123456789'
New-Item -ItemType Directory -Path "$testRoot/package", "$testRoot/tmp", "$testRoot/scripts" | Out-Null
Set-Content -LiteralPath "$testRoot/package/vp.exe" -Value 'Payload fixture'
@'
if ($args.Count -eq 0) {
    if (Test-Path Env:VP_SELF_SETUP_SUPPORT_CHECK) { exit 99 }
    New-Item -ItemType File -Path "$testRoot/binary-invoked" | Out-Null
    if ($scenario -eq 'failure') { exit 42 }
    if ($env:VP_SELF_SETUP_SHELL -ne 'powershell') { exit 98 }
    $expectedVpShell = if ($scenario -eq 'supported-pr') { 'fish' } else { 'powershell' }
    if ($env:VP_SHELL -ne $expectedVpShell) { exit 96 }
    if ($scenario -eq 'supported-pr' -and $env:NPM_CONFIG_REGISTRY -ne 'https://registry-bridge.viteplus.dev/') { exit 97 }
    Write-Output ("`$script:InstallDir = '{0}'" -f "$testRoot/data")
    Write-Output ("`$script:ShimDir = '{0}'" -f "$testRoot/installed bin")
    Write-Output ("`$script:CacheDir = '{0}'" -f "$testRoot/cache")
    Write-Output ("`$script:ConfigDir = '{0}'" -f "$testRoot/config")
    Write-Output ("`$script:StateDir = '{0}'" -f "$testRoot/state")
    exit 0
}
if ($env:VP_SELF_SETUP_SUPPORT_CHECK -ne '1') { exit 99 }
New-Item -ItemType File -Path "$testRoot/binary-probed" | Out-Null
if ($scenario -in @('legacy', 'legacy-remote', 'legacy-failure', 'pr')) { Write-Output 'Usage: vp [COMMAND]' }
else { Write-Output 'vite-plus-self-setup-v1' }
exit 0
'@ | Set-Content -LiteralPath "$testRoot/package/binary.ps1"
@'
param($BinarySource, $ResolvedVersion, $PreviewRef)
$script:InstallDir = "$testRoot/data"
$script:ShimDir = "$testRoot/installed bin"
$script:CacheDir = "$testRoot/cache"
$script:ConfigDir = "$testRoot/config"
$script:StateDir = "$testRoot/state"
if (-not [System.IO.Path]::IsPathRooted($BinarySource) -or -not (Test-Path -LiteralPath $BinarySource)) { throw 'Invalid payload path' }
@($ResolvedVersion, $PreviewRef) | Set-Content -LiteralPath "$testRoot/legacy"
if ($scenario -eq 'legacy-failure') { exit 42 }
'@ | Set-Content -LiteralPath "$testRoot/scripts/install-legacy.ps1"
& "$env:SystemRoot\System32\tar.exe" -czf "$testRoot/payload.tgz" -C $testRoot package
Assert ($LASTEXITCODE -eq 0) 'Could not create fixture'
$fixtureHasher = [Security.Cryptography.SHA512]::Create()
try {
    $fixtureBytes = [IO.File]::ReadAllBytes("$testRoot/payload.tgz")
    $fixtureDigest = $fixtureHasher.ComputeHash($fixtureBytes)
    $fixtureIntegrity = 'sha512-' + [Convert]::ToBase64String($fixtureDigest)
} finally {
    $fixtureHasher.Dispose()
}
Set-Content -LiteralPath "$testRoot/package/tampered" -Value 'Modified archive'
& "$env:SystemRoot\System32\tar.exe" -czf "$testRoot/tampered.tgz" -C $testRoot package
Assert ($LASTEXITCODE -eq 0) 'Could not create tampered fixture'
Remove-Item -LiteralPath "$testRoot/package/tampered"
$env:TEMP = "$testRoot/tmp"

# Record extraction directory creation: integrity failures must precede it.
$newItemCommand = Get-Command New-Item
function New-Item {
    param($ItemType, $Path, [switch]$Force)
    if ((Split-Path -Leaf $Path) -like 'vite-platform-*') { $script:ExtractionStarted = $true }
    & $newItemCommand @PSBoundParameters
}

function Invoke-RestMethod {
    param($Uri, $Headers)
    $script:Requests.Add("GET $Uri")
    if ($Uri -eq 'https://custom.example/vite-plus/latest') {
        return @{ version = '0.2.9' }
    }
    if ([System.Uri]::UnescapeDataString($Uri) -like 'https://*/@voidzero-dev/vite-plus-cli-*/*') {
        # Release payloads must pass the real provenance gate before handoff.
        $dist = @{
            tarball = 'https://custom.example/platform.tgz'
            integrity = $fixtureIntegrity
            attestations = @{
                provenance = @{ predicateType = 'https://slsa.dev/provenance/v1' }
            }
        }
        switch -Wildcard ($scenario) {
            'integrity-missing*' { $dist.Remove('integrity') }
            'integrity-malformed' { $dist.integrity = 'sha512-invalid' }
            'integrity-unsupported' { $dist.integrity = $fixtureIntegrity.Replace('sha512-', 'sha256-') }
            'integrity-noncanonical' { $dist.integrity = $fixtureIntegrity.Substring(0, $fixtureIntegrity.Length - 3) + 'B==' }
            'integrity-wrong-type' { $dist.integrity = @($fixtureIntegrity) }
            'integrity-dotted-key' {
                $dist.Remove('integrity')
                $dist['dist.integrity'] = $fixtureIntegrity
            }
        }
        if ($scenario -like '*pr') { $dist.Remove('attestations') }
        return @{ dist = $dist }
    }
    throw "Unexpected metadata request: $Uri"
}
function Invoke-WebRequest {
    param($Uri, $Method, $OutFile, [switch]$UseBasicParsing, $ErrorAction)
    $script:Requests.Add("$Method $Uri")
    if ($Method -eq 'Head') {
        return @{ Headers = @{ 'x-commit-key' = "voidzero-dev:vite-plus:$fixtureSha" } }
    }
    if (-not $OutFile) {
        $content = Get-Content -LiteralPath "$testRoot/scripts/install-legacy.ps1" -Raw
        return @{ Content = [Text.Encoding]::UTF8.GetBytes($content) }
    }
    $payload = if ($scenario -like 'integrity-mismatch*') { 'tampered' } else { 'payload' }
    Copy-Item -LiteralPath "$testRoot/$payload.tgz" -Destination $OutFile
}

# Use an executable script fixture so these checks need no native compiler.
$probe = ${function:Test-SelfSetupSupport}
function Test-SelfSetupSupport {
    param($BinarySource)
    & $probe -BinarySource (Join-Path (Split-Path $BinarySource) 'binary.ps1')
}
$handoff = ${function:Invoke-InstallHandoff}
function Invoke-InstallHandoff {
    param($BinarySource)
    & $handoff -BinarySource (Join-Path (Split-Path $BinarySource) 'binary.ps1')
}

try {
    foreach ($scenario in @(
        'supported', 'legacy', 'legacy-remote', 'legacy-failure', 'failure', 'pr', 'supported-pr',
        'integrity-missing', 'integrity-malformed', 'integrity-unsupported', 'integrity-noncanonical',
        'integrity-wrong-type', 'integrity-dotted-key', 'integrity-mismatch', 'integrity-missing-pr', 'integrity-mismatch-pr'
    )) {
        $env:Path = $originalPath
        $env:NPM_CONFIG_REGISTRY = 'https://custom.example'
        $initialVpShell = if ($scenario -eq 'supported-pr') { 'fish' } else { $null }
        $env:VP_SHELL = $initialVpShell
        $script:Requests = New-Object 'System.Collections.Generic.List[string]'
        $script:ExitCode = 0
        $script:ExtractionStarted = $false
        $script:PackageMetadata = $null
        Remove-Item -LiteralPath "$testRoot/legacy", "$testRoot/binary-invoked", "$testRoot/binary-probed" -ErrorAction SilentlyContinue
        $env:VP_SELF_SETUP_SUPPORT_CHECK = if ($scenario -eq 'supported') { 'original' } else { $null }
        $isIntegrityFailure = $scenario -like 'integrity-*'
        $expectedExit = if ($isIntegrityFailure) {
            1
        } elseif ($scenario -in @('failure', 'legacy-failure')) {
            42
        } else {
            0
        }
        try {
            & {
                $ViteVersion = 'latest'
                $LocalTgz = $LocalBinary = $PrVersion = $PrCommitVersion = $null
                $NpmRegistry = 'https://custom.example'
                $InstallerDirectory = if ($scenario -eq 'legacy-remote') { $null } else { "$testRoot/scripts" }
                if ($scenario -like '*pr') { $PrVersion = '2406' }
                Main
                Assert ($env:NPM_CONFIG_REGISTRY -eq 'https://custom.example') 'Setup changed the caller registry'
                Assert ($script:InstallDir -eq "$testRoot/data") 'InstallDir was lost'
                Assert ($script:ShimDir -eq "$testRoot/installed bin") 'ShimDir was lost'
                Assert ($script:CacheDir -eq "$testRoot/cache") 'CacheDir was lost'
                Assert ($script:ConfigDir -eq "$testRoot/config") 'ConfigDir was lost'
                Assert ($script:StateDir -eq "$testRoot/state") 'StateDir was lost'
            }
        } catch {
            if ($expectedExit -eq 0 -or -not (Test-IsInstallStopException $_)) { throw }
        }
        Assert ($script:ExitCode -eq $expectedExit) 'Binary exit code was lost'
        Assert (@(Get-ChildItem -LiteralPath "$testRoot/tmp" -Force).Count -eq 0) 'Temporary payload was not cleaned up'
        if ($isIntegrityFailure) {
            $tarballRequested = @($script:Requests | Where-Object { $_.Contains('platform.tgz') }).Count -gt 0
            Assert ($tarballRequested -eq ($scenario -like 'integrity-mismatch*')) 'Incorrect tarball request before integrity validation'
            Assert (-not $script:ExtractionStarted) 'Unverified archive reached extraction'
            Assert (-not (Test-Path -LiteralPath "$testRoot/binary-probed")) 'Unverified binary was probed'
            Assert (-not (Test-Path -LiteralPath "$testRoot/binary-invoked")) 'Unverified binary was invoked'
            Assert (-not (Test-Path -LiteralPath "$testRoot/legacy")) 'Unverified binary reached legacy setup'
            Write-Host "PASS: $scenario"
            continue
        }
        Assert ($env:VP_SHELL -eq $initialVpShell) 'Setup changed the caller shell'
        if ($scenario -eq 'supported') {
            Assert (($env:Path -split ';')[0] -eq "$testRoot/installed bin") 'Installed bin directory was not added to the current PATH'
        } elseif ($scenario -eq 'failure') {
            Assert ($env:Path -eq $originalPath) 'Failed setup changed the current PATH'
        }
        $usesBinary = $scenario -in @('supported', 'supported-pr', 'failure')
        Assert ((Test-Path -LiteralPath "$testRoot/binary-invoked") -eq $usesBinary) 'Incorrect binary invocation'
        Assert ((Test-Path -LiteralPath "$testRoot/legacy") -eq (-not $usesBinary)) 'Incorrect legacy invocation'
        if ($scenario -eq 'pr') {
            $record = @(Get-Content -LiteralPath "$testRoot/legacy")
            Assert ($record[0] -eq "0.0.0-commit.$fixtureSha" -and $record[1] -eq '2406') 'Resolved preview identity was lost'
            Assert ($script:Requests[1] -like "GET https://registry-bridge.viteplus.dev/*/0.0.0-commit.$fixtureSha") 'Preview metadata used a mutable ref or the wrong registry'
            Assert ($script:Requests.Count -eq 3) 'Preview was resolved or downloaded more than once'
        }
        Write-Host "PASS: $scenario"
    }
} finally {
    $env:VP_SELF_SETUP_SUPPORT_CHECK = $originalCheck
    $env:Path = $originalPath
    $env:NPM_CONFIG_REGISTRY = $originalRegistry
    $env:VP_SHELL = $originalVpShell
    $env:TEMP = $originalTemp
    Remove-Item -LiteralPath $testRoot -Recurse -Force
    $global:LASTEXITCODE = 0
}
