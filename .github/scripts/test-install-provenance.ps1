# Run from the repository root with RUNNER_TEMP and TEST_VERSION set.
$ErrorActionPreference = "Stop"

function Invoke-ProvenanceCase {
    param(
        [string]$Mode,
        [bool]$ExpectRejection,
        [bool]$RawContentType = $false
    )

    $caseDir = Join-Path $env:RUNNER_TEMP "vite-plus-provenance-ps1-$Mode"
    $homeDir = Join-Path $caseDir "home"
    $vpHome = Join-Path $caseDir "vp-home"
    $portFile = Join-Path $caseDir "port"
    $logFile = Join-Path $caseDir "requests.jsonl"
    $stdoutFile = Join-Path $caseDir "registry.stdout.log"
    $stderrFile = Join-Path $caseDir "registry.stderr.log"
    $installerStdoutFile = Join-Path $caseDir "installer.stdout.log"
    $installerStderrFile = Join-Path $caseDir "installer.stderr.log"

    Remove-Item -Path $caseDir -Recurse -Force -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Path $homeDir, $vpHome -Force | Out-Null

    $fixture = Join-Path (Get-Location) "packages/cli/tests/fixtures/provenance-registry.mjs"
    $serverArgs = @(
        $fixture,
        "--port-file", $portFile,
        "--log-file", $logFile,
        "--mode", $Mode,
        "--version", $env:TEST_VERSION
    )
    if ($RawContentType) {
        $serverArgs += @("--raw-content-type", "true")
    }

    $server = Start-Process -FilePath "node" -ArgumentList $serverArgs -PassThru -RedirectStandardOutput $stdoutFile -RedirectStandardError $stderrFile

    try {
        for ($attempt = 0; $attempt -lt 100 -and -not (Test-Path $portFile); $attempt++) {
            Start-Sleep -Milliseconds 100
        }
        if (-not (Test-Path $portFile)) {
            throw "Mock registry did not start: $(Get-Content $stderrFile -Raw -ErrorAction SilentlyContinue)"
        }

        $registry = "http://127.0.0.1:$(Get-Content $portFile -Raw)"
        $env:CI = "true"
        $env:USERPROFILE = $homeDir
        $env:VP_HOME = $vpHome
        $env:VP_NODE_MANAGER = "no"
        $env:VP_VERSION = $env:TEST_VERSION
        $env:NPM_CONFIG_REGISTRY = $registry

        # Windows PowerShell 5.1 turns redirected native stderr into a
        # NativeCommandError. Capture each stream separately so the
        # expected tarball failure cannot stop this parent test script.
        $installerArgs = @(
            "-NoProfile",
            "-ExecutionPolicy", "Bypass",
            "-File", ".\packages\cli\install.ps1"
        )
        $installer = Start-Process -FilePath "powershell.exe" -ArgumentList $installerArgs -PassThru -Wait -RedirectStandardOutput $installerStdoutFile -RedirectStandardError $installerStderrFile
        $exitCode = $installer.ExitCode
        $text = @(
            Get-Content -Path $installerStdoutFile -Raw -ErrorAction SilentlyContinue
            Get-Content -Path $installerStderrFile -Raw -ErrorAction SilentlyContinue
        ) -join "`n"
    } finally {
        if (-not $server.HasExited) {
            Stop-Process -Id $server.Id -Force
            $server.WaitForExit()
        }
    }

    Write-Host $text
    if ($exitCode -eq 0) {
        throw "Expected the fixture tarball endpoint to prevent installation"
    }

    $requests = Get-Content -Path $logFile -Raw
    $tarballRequested = $requests.Contains('"path":"/platform.tgz"')
    $provenanceError = "does not contain supported npm provenance metadata"

    if ($ExpectRejection) {
        if (-not $text.Contains($provenanceError)) {
            throw "Expected provenance rejection for $Mode"
        }
        if (-not $text.Contains("@voidzero-dev/vite-plus-cli-") -or
                -not $text.Contains($env:TEST_VERSION)) {
            throw "Expected rejected package name and version in installer output"
        }
        if ($tarballRequested) {
            throw "Platform tarball was requested before provenance validation"
        }
        if ((Test-Path (Join-Path $vpHome "current")) -or
                (Test-Path (Join-Path $vpHome "$($env:TEST_VERSION)\bin\vp.exe"))) {
            throw "Rejected package left an active or executable installation"
        }
    } else {
        if ($text.Contains($provenanceError)) {
            throw "Supported provenance metadata was rejected"
        }
        if (-not $tarballRequested) {
            throw "Supported provenance metadata did not reach the tarball endpoint"
        }
    }

    # The child installer and the deliberate tarball failure are expected.
    $global:LASTEXITCODE = 0
}

Invoke-ProvenanceCase -Mode "missing" -ExpectRejection $true
Invoke-ProvenanceCase -Mode "malformed" -ExpectRejection $true
Invoke-ProvenanceCase -Mode "top-level-only" -ExpectRejection $true -RawContentType $true
Invoke-ProvenanceCase -Mode "dotted-top-level-key" -ExpectRejection $true
Invoke-ProvenanceCase -Mode "unsupported" -ExpectRejection $true
Invoke-ProvenanceCase -Mode "valid-v1" -ExpectRejection $false
Invoke-ProvenanceCase -Mode "valid-v0.2" -ExpectRejection $false

# Match the version policy in Rust and install.sh on a custom registry.
$env:TEST_VERSION = "0.0.0-commit.0123456789abcdef0123456789abcdef01234567"
Invoke-ProvenanceCase -Mode "missing" -ExpectRejection $false
Invoke-ProvenanceCase -Mode "unsupported" -ExpectRejection $false
$env:TEST_VERSION = "0.0.0-commit.0123456789ABCDEF0123456789ABCDEF01234567"
Invoke-ProvenanceCase -Mode "missing" -ExpectRejection $false
foreach ($version in @(
    "1.2.3-beta.1",
    "0.0.0",
    "0.0.0-beta.1",
    # PowerShell 5.1 strips trailing dots from URL paths. Use a bare
    # commit label to test a missing SHA through the registry fixture.
    "0.0.0-commit",
    "0.0.0-commit.abc1234",
    "0.0.0-commit.0123456789abcdef0123456789abcdef012345678",
    "0.0.0-commit.0123456789abcdef0123456789abcdef0123456g",
    "0.0.0-COMMIT.0123456789abcdef0123456789abcdef01234567",
    "0.0.0-commit.0123456789abcdef0123456789abcdef01234567.extra"
)) {
    $env:TEST_VERSION = $version
    Invoke-ProvenanceCase -Mode "missing" -ExpectRejection $true
}
