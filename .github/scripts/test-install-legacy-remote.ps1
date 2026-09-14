# Reproduce setup-vp's remote install of 0.2.6 in cloudflare/vinext#3251.
# Serve this checkout's scripts so the test does not depend on the deployed installer.
$ErrorActionPreference = 'Stop'
$root = Join-Path $env:RUNNER_TEMP 'vp-remote-legacy'
New-Item -ItemType Directory -Force -Path $root | Out-Null
Copy-Item (Join-Path $PSScriptRoot '../../packages/cli/install.ps1') "$root/install.txt"
Copy-Item (Join-Path $PSScriptRoot '../../packages/cli/install-legacy.ps1') "$root/install-legacy.bin"
$stdout = Join-Path $root 'server.stdout.txt'
$stderr = Join-Path $root 'server.stderr.txt'

# Python serves .txt as text/plain and .bin as application/octet-stream.
# Port 0 selects an available port. Unbuffered output exposes it before any request.
$server = Start-Process -FilePath (Get-Command python).Source `
    -ArgumentList @('-u', '-m', 'http.server', '0', '--bind', '127.0.0.1', '--directory', "`"$root`"") `
    -RedirectStandardOutput $stdout -RedirectStandardError $stderr -PassThru
try {
    $url = $null
    $deadline = (Get-Date).AddSeconds(30)
    while ((Get-Date) -lt $deadline) {
        if ($server.HasExited) {
            throw "Installer HTTP server exited: $(Get-Content -LiteralPath $stderr -Raw)"
        }
        $serverOutput = Get-Content -LiteralPath $stdout -Raw
        if ($serverOutput -match 'Serving HTTP on 127\.0\.0\.1 port (\d+)') {
            $url = "http://127.0.0.1:$($Matches[1])"
            break
        }
        Start-Sleep -Milliseconds 100
    }
    if (-not $url) { throw 'Installer HTTP server did not start' }

    $env:VP_HOME = Join-Path $root 'install'
    $env:VP_TEST_INSTALLER_URL = "$url/install.txt"
    $env:VP_LEGACY_INSTALLER_URL = "$url/install-legacy.bin"
    $response = Invoke-WebRequest -Uri $env:VP_LEGACY_INSTALLER_URL -UseBasicParsing
    if ($response.Content -isnot [byte[]]) {
        throw "Expected a byte-array legacy response, got $($response.Content.GetType().FullName)"
    }

    # setup-vp evaluates downloaded text in a script block, with no sibling legacy file.
    # A child process also keeps installer exit calls from bypassing server cleanup.
    $powershell = (Get-Process -Id $PID).Path
    & $powershell -NoProfile -Command '& ([scriptblock]::Create((irm -TimeoutSec 15 $env:VP_TEST_INSTALLER_URL)))'
    if ($LASTEXITCODE -ne 0) { throw "Remote install failed with exit code $LASTEXITCODE" }

    # One request above checks the response type; another must come from the installer.
    $requests = @(Select-String -LiteralPath $stderr -Pattern 'GET /install-legacy.bin ' -SimpleMatch)
    if ($requests.Count -ne 2) { throw 'The installer did not fetch the remote legacy script exactly once' }

    $vp = Join-Path $env:VP_HOME 'bin/vp.exe'
    $version = & $vp --version
    if ($LASTEXITCODE -ne 0 -or $version -notcontains "vp v$env:VP_VERSION") {
        throw "Expected the installed vp to report version $env:VP_VERSION, got: $version"
    }
    Write-Host ($version -join [Environment]::NewLine)
    & $vp env run --node 24 -- node --version
    if ($LASTEXITCODE -ne 0) { throw 'The installed vp could not run Node.js 24' }
} finally {
    Stop-Process -Id $server.Id -ErrorAction SilentlyContinue
}
