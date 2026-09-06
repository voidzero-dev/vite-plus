# Run locally on Windows with ./.github/scripts/test-install-bootstrap.ps1; no registry or installed vp needed.
$ErrorActionPreference = 'Stop'
$source = Get-Content -LiteralPath (Join-Path $PSScriptRoot '../../packages/cli/install.ps1') -Raw
. ([scriptblock]::Create(($source -replace '(?m)^    Main\r?$', '')))
function Exit-Installer { param([int]$Code = 1); $script:ExitCode = $Code; throw $script:InstallStopSignal }
function Assert($Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

$testRoot = Join-Path $env:TEMP "vite-bootstrap-test-$(Get-Random)"
$originalTemp = $env:TEMP
$originalCheck = $env:VP_SELF_SETUP_SUPPORT_CHECK
$originalPath = $env:Path
$originalRegistry = $env:NPM_CONFIG_REGISTRY
$fixtureSha = '0123456789012345678901234567890123456789'
New-Item -ItemType Directory -Path "$testRoot/package", "$testRoot/tmp", "$testRoot/scripts" | Out-Null
Set-Content -LiteralPath "$testRoot/package/vp.exe" -Value 'Payload fixture'
@'
if ($args.Count -eq 0) {
    if (Test-Path Env:VP_SELF_SETUP_SUPPORT_CHECK) { exit 99 }
    New-Item -ItemType File -Path "$testRoot/binary-invoked" | Out-Null
    if ($scenario -eq 'failure') { exit 42 }
    if ($env:VP_SELF_SETUP_SHELL -ne 'powershell') { exit 98 }
    if ($scenario -eq 'supported-pr' -and $env:NPM_CONFIG_REGISTRY -ne 'https://registry-bridge.viteplus.dev/') { exit 97 }
    Write-Output ("`$script:InstallDir = '{0}'" -f "$testRoot/data")
    Write-Output ("`$script:ShimDir = '{0}'" -f "$testRoot/installed bin")
    Write-Output ("`$script:CacheDir = '{0}'" -f "$testRoot/cache")
    Write-Output ("`$script:ConfigDir = '{0}'" -f "$testRoot/config")
    Write-Output ("`$script:StateDir = '{0}'" -f "$testRoot/state")
    exit 0
}
if ($env:VP_SELF_SETUP_SUPPORT_CHECK -ne '1') { exit 99 }
if ($scenario -in @('legacy', 'legacy-failure', 'pr')) { Write-Output 'Usage: vp [COMMAND]' }
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
$env:TEMP = "$testRoot/tmp"

function Invoke-RestMethod {
    param($Uri)
    $script:Requests.Add("GET $Uri")
    return @{ version = '0.2.9' }
}
function Invoke-WebRequest {
    param($Uri, $Method, $OutFile, [switch]$UseBasicParsing, $ErrorAction)
    $script:Requests.Add("$Method $Uri")
    if ($Method -eq 'Head') {
        return @{ Headers = @{ 'x-commit-key' = "voidzero-dev:vite-plus:$fixtureSha" } }
    }
    Copy-Item -LiteralPath "$testRoot/payload.tgz" -Destination $OutFile
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
    foreach ($scenario in @('supported', 'legacy', 'legacy-failure', 'failure', 'pr', 'supported-pr')) {
        $env:Path = $originalPath
        $env:NPM_CONFIG_REGISTRY = 'https://custom.example'
        $script:Requests = New-Object 'System.Collections.Generic.List[string]'
        $script:ExitCode = 0
        $script:PackageMetadata = $null
        Remove-Item -LiteralPath "$testRoot/legacy", "$testRoot/binary-invoked" -ErrorAction SilentlyContinue
        $env:VP_SELF_SETUP_SUPPORT_CHECK = if ($scenario -eq 'supported') { 'original' } else { $null }
        try {
            & {
                $ViteVersion = 'latest'
                $LocalTgz = $LocalBinary = $PrVersion = $PrCommitVersion = $null
                $NpmRegistry = 'https://custom.example'
                $InstallerDirectory = "$testRoot/scripts"
                if ($scenario -in @('pr', 'supported-pr')) { $PrVersion = '2406' }
                Main
                Assert ($env:NPM_CONFIG_REGISTRY -eq 'https://custom.example') 'Setup changed the caller registry'
                Assert ($script:InstallDir -eq "$testRoot/data") 'InstallDir was lost'
                Assert ($script:ShimDir -eq "$testRoot/installed bin") 'ShimDir was lost'
                Assert ($script:CacheDir -eq "$testRoot/cache") 'CacheDir was lost'
                Assert ($script:ConfigDir -eq "$testRoot/config") 'ConfigDir was lost'
                Assert ($script:StateDir -eq "$testRoot/state") 'StateDir was lost'
            }
        } catch {
            if ($scenario -notin @('failure', 'legacy-failure') -or -not (Test-IsInstallStopException $_)) { throw }
        }
        $expectedExit = if ($scenario -in @('failure', 'legacy-failure')) { 42 } else { 0 }
        Assert ($script:ExitCode -eq $expectedExit) 'Binary exit code was lost'
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
            Assert ($script:Requests[-1].EndsWith("@$fixtureSha")) 'Payload used a mutable ref'
            Assert ($script:Requests.Count -eq 2) 'Preview was resolved or downloaded more than once'
        }
        Assert (@(Get-ChildItem -LiteralPath "$testRoot/tmp" -Force).Count -eq 0) 'Temporary payload was not cleaned up'
        Write-Host "PASS: $scenario"
    }
} finally {
    $env:VP_SELF_SETUP_SUPPORT_CHECK = $originalCheck
    $env:Path = $originalPath
    $env:NPM_CONFIG_REGISTRY = $originalRegistry
    $env:TEMP = $originalTemp
    Remove-Item -LiteralPath $testRoot -Recurse -Force
    $global:LASTEXITCODE = 0
}
