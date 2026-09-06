# Vite+ CLI Installer for Windows
# https://vite.plus/ps1
#
# Usage:
#   irm https://vite.plus/ps1 | iex
#
# Environment variables:
#   VP_VERSION - Version to install (default: latest)
#   VP_HOME - Optional pin for the monolithic layout. If unset, Vite+ reuses an
#             existing %USERPROFILE%\.vite-plus install. Otherwise, the complete
#             VP_*_DIR group or Windows Local and Roaming folders select the roots.
#   VP_BIN_DIR / VP_DATA_DIR / VP_CACHE_DIR - Complete group of absolute
#                                             category overrides
#   NPM_CONFIG_REGISTRY - Custom npm registry URL (default: https://registry.npmjs.org)
#   VP_LOCAL_TGZ - Path to local vite-plus.tgz (for development/testing)
#   VP_PR_VERSION - PR number or commit SHA to install from the registry bridge
#                   (for temporary testing of unreleased builds, e.g. VP_PR_VERSION=1569).
#                   When set, overrides VP_VERSION and installs the clearly-defined
#                   0.0.0-commit.<sha> build through the bridge instead of npm.

# When dot-sourced, returns script-scoped InstallDir, ShimDir, CacheDir, ConfigDir, and StateDir.
# These are resolved paths, not VP_* overrides for subsequent commands.
$ErrorActionPreference = "Stop"

$ViteVersion = if ($env:VP_VERSION) { $env:VP_VERSION } else { "latest" }
# npm registry URL (strip trailing slash if present)
$NpmRegistry = if ($env:NPM_CONFIG_REGISTRY) { $env:NPM_CONFIG_REGISTRY.TrimEnd('/') } else { "https://registry.npmjs.org" }
# Local tarball for development/testing
$LocalTgz = $env:VP_LOCAL_TGZ
# Local binary path (set by install-global-cli.ts for local dev)
$LocalBinary = $env:VP_LOCAL_BINARY
# PR number or commit SHA to install as a test build (registry bridge mode)
$PrVersion = $env:VP_PR_VERSION
# Registry bridge that serves PR preview builds as clearly-versioned packages.
# The pkg.pr.new-style download URL (BridgeDownloadBase) 302-redirects to a
# canonical 0.0.0-commit.<sha> tarball; the registry (BridgeRegistry) resolves
# those commit versions (and proxies everything else to npmjs) so a full install
# pulls a coherent, clearly-defined test build.
$BridgeDownloadBase = "https://registry-bridge.viteplus.dev/voidzero-dev/vite-plus"
$BridgeRegistry = "https://registry-bridge.viteplus.dev/"

$script:InstallStopSignal = 'VP_INSTALL_STOP'
$script:PackageMetadata = $null
# Legacy is published beside this bootstrap; preview builds rewrite this origin.
$LegacyInstallerUrl = if ($env:VP_LEGACY_INSTALLER_URL) { $env:VP_LEGACY_INSTALLER_URL } else { 'https://viteplus.dev/install-legacy.ps1' }
$InstallerDirectory = $PSScriptRoot

function Write-Info {
    param([string]$Message)
    Write-Host "info: " -ForegroundColor Blue -NoNewline
    Write-Host $Message
}

function Test-IsInstallStopException {
    param(
        [System.Management.Automation.ErrorRecord]$ErrorRecord
    )
    return $ErrorRecord.Exception.Message -eq $script:InstallStopSignal
}

function Test-ShouldKeepShellOpenAfterFailure {
    # Only `irm ... | iex` typed in an already-open interactive shell should keep the
    # session alive. CI, script files, and `powershell -Command "..."` must exit non-zero.
    if ($env:CI -eq "true") {
        return $false
    }
    if ($PSCommandPath) {
        return $false
    }
    if (-not [Environment]::UserInteractive) {
        return $false
    }
    try {
        $commandLine = (Get-CimInstance Win32_Process -Filter "ProcessId=$PID").CommandLine
        if ($commandLine -match '(^|\s)-Command(\s|$)') {
            return $false
        }
    } catch {
        return $false
    }
    return $true
}

function Exit-Installer {
    param([int]$Code = 1)
    $global:LASTEXITCODE = $Code
    if (-not (Test-ShouldKeepShellOpenAfterFailure)) {
        exit $Code
    }
    throw $script:InstallStopSignal
}

function Write-Error-Exit {
    param([string]$Message)
    Write-Host "error: " -ForegroundColor Red -NoNewline
    Write-Host $Message
    Exit-Installer
}

function Resolve-BridgeCommitVersion {
    param([string]$Ref)
    $sha = $Ref
    if ($Ref -notmatch '^[0-9a-fA-F]{40}$') {
        try {
            $resp = Invoke-WebRequest -Uri "$BridgeDownloadBase@$Ref" -Method Head -UseBasicParsing -ErrorAction Stop
        } catch {
            return $null
        }
        $commitKey = @($resp.Headers['x-commit-key'])[0]
        if (-not $commitKey) { return $null }
        $sha = ($commitKey -split ':')[-1]
    }
    if ($sha -notmatch '^[0-9a-fA-F]{40}$') { return $null }
    return "0.0.0-commit.$sha"
}

function Get-Architecture {
    if ([Environment]::Is64BitOperatingSystem) {
        if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") {
            return "arm64"
        } else {
            return "x64"
        }
    } else {
        Write-Error-Exit "32-bit Windows is not supported"
    }
}

function Get-PackageMetadata {
    if ($null -eq $script:PackageMetadata) {
        $versionPath = $ViteVersion
        $metadataUrl = "$NpmRegistry/vite-plus/$versionPath"
        try {
            $script:PackageMetadata = Invoke-RestMethod $metadataUrl
        } catch {
            if (Test-IsInstallStopException $_) { throw }
            # Try to extract npm error message from response
            $errorMsg = $_.ErrorDetails.Message
            if ($errorMsg) {
                try {
                    $errorJson = $errorMsg | ConvertFrom-Json
                    if ($errorJson.error) {
                        Write-Error-Exit "Failed to fetch version '${versionPath}': $($errorJson.error)`n  URL: $metadataUrl"
                    }
                } catch {
                    if (Test-IsInstallStopException $_) { throw }
                    # JSON parsing failed, fall through to generic error
                }
            }
            Write-Error-Exit "Failed to fetch package metadata from: $metadataUrl`nError: $_"
        }
        # Check for error in successful response
        # npm can return {"error":"..."} object or a plain string like "version not found: test"
        if ($script:PackageMetadata -is [string]) {
            # Some registries (e.g. JFrog) may return JSON with a non-JSON content type,
            # causing Invoke-RestMethod to return a raw string. Try parsing it as JSON first.
            try {
                $script:PackageMetadata = $script:PackageMetadata | ConvertFrom-Json
            } catch {
                if (Test-IsInstallStopException $_) { throw }
                # Not valid JSON - treat as plain string error
                Write-Error-Exit "Failed to fetch version '${versionPath}': $script:PackageMetadata`n  URL: $metadataUrl"
            }
        }
        if ($script:PackageMetadata.error) {
            Write-Error-Exit "Failed to fetch version '${versionPath}': $($script:PackageMetadata.error)`n  URL: $metadataUrl"
        }
    }
    return $script:PackageMetadata
}

function Get-VersionFromMetadata {
    $metadata = Get-PackageMetadata
    if (-not $metadata.version) {
        Write-Error-Exit "Failed to extract version from package metadata"
    }
    return $metadata.version
}

function Get-PlatformSuffix {
    param([string]$Platform)
    # Windows needs -msvc suffix, other platforms map directly
    if ($Platform.StartsWith("win32-")) { return "${Platform}-msvc" }
    return $Platform
}

function Get-UserHomeDir {
    if (-not [string]::IsNullOrWhiteSpace($env:USERPROFILE)) {
        return $env:USERPROFILE
    }
    if (-not [string]::IsNullOrWhiteSpace($env:HOME)) {
        return $env:HOME
    }
    return [Environment]::GetFolderPath('UserProfile')
}

# Released setup-vp versions add %USERPROFILE%\.vite-plus\bin to the GitHub
# Actions PATH. They do this after the installer exits. Use the monolithic
# layout until setup-vp declares support for VP_DUMP_DIRS.
function Enable-SetupVpLegacyCompatibility {
    if ($env:GITHUB_ACTION_REPOSITORY -cne "voidzero-dev/setup-vp") {
        return
    }
    if ($env:VP_VPDIRS_AWARE -eq "1") {
        return
    }
    if ($env:VP_HOME -or $env:VP_BIN_DIR -or $env:VP_DATA_DIR -or $env:VP_CACHE_DIR) {
        return
    }

    $userHome = Get-UserHomeDir
    if ([string]::IsNullOrWhiteSpace($userHome)) {
        Write-Error-Exit "Vite+ could not resolve the user home directory."
    }
    $env:VP_HOME = Join-Path $userHome ".vite-plus"
}

function Main {
    Enable-SetupVpLegacyCompatibility

    if ($PrVersion -and $LocalTgz) {
        Write-Error-Exit "VP_PR_VERSION and VP_LOCAL_TGZ cannot be used together"
    }

    # Suppress progress bars for cleaner output
    $ProgressPreference = 'SilentlyContinue'

    # Local development mode: use local tgz
    if ($LocalTgz) {
        # Validate local tgz
        if (-not (Test-Path $LocalTgz)) {
            Write-Error-Exit "Local tarball not found: $LocalTgz"
        }
        # Use version as-is (default to "local-dev")
        if ($ViteVersion -eq "latest" -or $ViteVersion -eq "test") {
            $ViteVersion = "local-dev"
        }
        if (-not $LocalBinary -or -not (Test-Path -LiteralPath $LocalBinary -PathType Leaf)) {
            Write-Error-Exit "Set VP_LOCAL_BINARY when you use VP_LOCAL_TGZ."
        }
    } elseif ($PrVersion) {
        # Registry bridge mode: resolve the requested PR/SHA to the bridge's
        # immutable commit version (0.0.0-commit.<sha>), the clearly-defined test
        # version we install. Legacy receives the full SHA as its preview ref.
        $PrCommitVersion = Resolve-BridgeCommitVersion -Ref $PrVersion
        if (-not $PrCommitVersion) {
            Write-Error-Exit "Could not resolve a registry bridge build for $PrVersion"
        }
        $ViteVersion = $PrCommitVersion
        Write-Info "Using registry bridge build: $PrCommitVersion"
    } else {
        # Fetch package metadata and resolve version from npm
        $ViteVersion = Get-VersionFromMetadata
    }

    Get-PayloadAndHandoff
}

function Get-PayloadAndHandoff {
    $arch = Get-Architecture
    $platform = "win32-$arch"
    $binaryName = "vp.exe"

    # Keep acquisition separate from permanent installation. The bootstrap owns cleanup.
    $platformTempExtract = $null
    try {
        if (-not $LocalTgz) {
            # npm registry or registry bridge (when PrVersion is set)
            $platformSuffix = Get-PlatformSuffix -Platform $platform
            if ($PrVersion) {
                # The registry bridge redirects this URL to the platform tarball for
                # the matching commit build (0.0.0-commit.<sha>).
                $platformUrl = "$BridgeDownloadBase/@voidzero-dev/vite-plus-cli-$platformSuffix@$($PrCommitVersion.Substring(13))"
            } else {
                $packageName = "@voidzero-dev/vite-plus-cli-$platformSuffix"
                $platformUrl = "$NpmRegistry/$packageName/-/vite-plus-cli-$platformSuffix-$ViteVersion.tgz"
            }

            $platformTempFile = New-TemporaryFile
            try {
                Invoke-WebRequest -Uri $platformUrl -OutFile $platformTempFile

                # Create temp extraction directory
                $platformTempExtract = Join-Path $env:TEMP "vite-platform-$(Get-Random)"
                New-Item -ItemType Directory -Force -Path $platformTempExtract | Out-Null

                # Extract the package
                & "$env:SystemRoot\System32\tar.exe" -xzf $platformTempFile -C $platformTempExtract
                if ($LASTEXITCODE -ne 0) {
                    Write-Error-Exit "Failed to extract platform package from: $platformUrl"
                }
            } finally {
                Remove-Item $platformTempFile -ErrorAction SilentlyContinue
            }

            $binarySource = Join-Path (Join-Path $platformTempExtract "package") $binaryName
            if (-not (Test-Path -LiteralPath $binarySource -PathType Leaf)) {
                Write-Error-Exit "Downloaded package does not contain $binaryName"
            }
            Unblock-File -LiteralPath $binarySource
        } else {
            $binarySource = $LocalBinary
        }
        $binarySource = (Resolve-Path -LiteralPath $binarySource).Path
        if (Test-SelfSetupSupport -BinarySource $binarySource) {
            Invoke-InstallHandoff -BinarySource $binarySource
        } else {
            Invoke-LegacyInstaller -BinarySource $binarySource
        }
    } finally {
        if ($platformTempExtract) {
            Remove-Item -LiteralPath $platformTempExtract -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}

function Test-SelfSetupSupport {
    param([string]$BinarySource)
    $previous = $env:VP_SELF_SETUP_SUPPORT_CHECK
    try {
        $env:VP_SELF_SETUP_SUPPORT_CHECK = '1'
        # Old binaries must exit with help rather than opening an interactive picker.
        $response = & $BinarySource --help 2>$null
        return $LASTEXITCODE -eq 0 -and @($response).Count -eq 1 -and $response -ceq 'vite-plus-self-setup-v1'
    } catch {
        return $false
    } finally {
        $env:VP_SELF_SETUP_SUPPORT_CHECK = $previous
    }
}

function Invoke-LegacyInstaller {
    param([string]$BinarySource)
    $global:LASTEXITCODE = 0
    $legacyScript = if ($InstallerDirectory) { Join-Path $InstallerDirectory 'install-legacy.ps1' }
    if ($legacyScript -and (Test-Path -LiteralPath $legacyScript -PathType Leaf)) {
        . $legacyScript -BinarySource $BinarySource -ResolvedVersion $ViteVersion -PreviewRef $PrVersion
    } else {
        $response = Invoke-WebRequest -Uri $LegacyInstallerUrl -UseBasicParsing
        . ([scriptblock]::Create($response.Content)) -BinarySource $BinarySource -ResolvedVersion $ViteVersion -PreviewRef $PrVersion
    }
    # A child script's exit only returns to this bootstrap, so forward its failure.
    if ($LASTEXITCODE -ne 0) {
        Exit-Installer -Code $LASTEXITCODE
    }
}

function Invoke-InstallHandoff {
    param([string]$BinarySource)
    $previous = $env:VP_SELF_SETUP_SUPPORT_CHECK
    $previousShell = $env:VP_SELF_SETUP_SHELL
    $previousRegistry = $env:NPM_CONFIG_REGISTRY
    try {
        Remove-Item Env:VP_SELF_SETUP_SUPPORT_CHECK -ErrorAction SilentlyContinue
        $env:VP_SELF_SETUP_SHELL = 'powershell'
        # Preview dependencies must use the same registry as the downloaded binary.
        if ($PrVersion) {
            $env:NPM_CONFIG_REGISTRY = $BridgeRegistry
        }
        $result = & $BinarySource
        if ($LASTEXITCODE -ne 0) {
            Exit-Installer -Code $LASTEXITCODE
        }
        Invoke-Expression ($result -join "`n")
        # A child can update the user PATH, but this session needs the resolved bin directory too.
        if (($env:Path -split ';') -notcontains $script:ShimDir) {
            $env:Path = "$script:ShimDir;$env:Path"
        }
    } finally {
        $env:VP_SELF_SETUP_SHELL = $previousShell
        $env:NPM_CONFIG_REGISTRY = $previousRegistry
        $env:VP_SELF_SETUP_SUPPORT_CHECK = $previous
    }
}

try {
    Main
} catch {
    if (Test-IsInstallStopException $_) {
        if (Test-ShouldKeepShellOpenAfterFailure) {
            return
        }
        exit $global:LASTEXITCODE
    }
    throw
}
