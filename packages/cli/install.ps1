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

$ErrorActionPreference = "Stop"

$ViteVersion = if ($env:VP_VERSION) { $env:VP_VERSION } else { "latest" }
# After these helper definitions, the selected payload resolves category roots
# through VP_DUMP_DIRS. Pre-split payloads use the legacy layout.
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

function Write-Info {
    param([string]$Message)
    Write-Host "info: " -ForegroundColor Blue -NoNewline
    Write-Host $Message
}

function Write-Success {
    param([string]$Message)
    Write-Host "success: " -ForegroundColor Green -NoNewline
    Write-Host $Message
}

function Write-Warn {
    param([string]$Message)
    Write-Host "warn: " -ForegroundColor Yellow -NoNewline
    Write-Host $Message
}

# Exit code when a Windows native binary cannot load required DLLs (STATUS_DLL_NOT_FOUND).
$script:DllNotFoundExitCode = -1073741515

function Test-IsDllNotFoundExitCode {
    param([int]$ExitCode)
    if ($ExitCode -eq $script:DllNotFoundExitCode) {
        return $true
    }
    if ($ExitCode -eq 3221225781) {
        return $true
    }
    if ($ExitCode -lt 0) {
        $hex = '{0:X8}' -f ($ExitCode -band 0xFFFFFFFF)
        return $hex -eq 'C0000135'
    }
    return $false
}

function Get-DllNotFoundInstallMessage {
    $arch = if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") { "arm64" } else { "x64" }
    $vcUrl = if ($arch -eq "arm64") {
        "https://aka.ms/vs/17/release/vc_redist.arm64.exe"
    } else {
        "https://aka.ms/vs/17/release/vc_redist.x64.exe"
    }
    return @"
vp.exe could not start (exit code 0xC0000135).
This usually means Microsoft Visual C++ 2015-2022 Redistributable ($arch) is not installed.

Install: $vcUrl
Then re-run: irm https://vite.plus/ps1 | iex
"@
}

# Internal stop signal: halts install without re-printing an error we already wrote.
$script:InstallStopSignal = 'VP_INSTALL_STOP'

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

function Test-ReleaseAgeError {
    param([string]$LogPath)
    if (-not (Test-Path $LogPath)) {
        return $false
    }

    $content = Get-Content -Path $LogPath -Raw
    # This wrapper install path is pinned to pnpm via packageManager, so this
    # detection follows pnpm's resolver/reporter output rather than npm/yarn.
    #
    # pnpm's PnpmError prefixes internal codes with ERR_PNPM_, so
    # NO_MATURE_MATCHING_VERSION is normally printed as
    # ERR_PNPM_NO_MATURE_MATCHING_VERSION. npm-resolver emits that code with the
    # "does not meet the minimumReleaseAge constraint" message when
    # publishedBy/minimumReleaseAge rejects a matching version.
    # https://github.com/pnpm/pnpm/blob/16cfde66ec71125d692ea828eba2a5f9b3cc54fc/core/error/src/index.ts#L18-L20
    # https://github.com/pnpm/pnpm/blob/16cfde66ec71125d692ea828eba2a5f9b3cc54fc/resolving/npm-resolver/src/index.ts#L76-L84
    #
    # default-reporter may append guidance mentioning minimumReleaseAgeExclude
    # when the error has an immatureVersion, so that token is also a useful
    # release-age signal. minimum-release-age is pnpm's .npmrc key; npm's
    # min-release-age is intentionally not treated as a pnpm signal here.
    # https://github.com/pnpm/pnpm/blob/16cfde66ec71125d692ea828eba2a5f9b3cc54fc/cli/default-reporter/src/reportError.ts#L163-L164
    # https://github.com/pnpm/pnpm/blob/16cfde66ec71125d692ea828eba2a5f9b3cc54fc/config/reader/src/types.ts#L73-L74
    $hasReleaseAgeText = $content -match "does not meet the minimumReleaseAge constraint" `
        -or $content -match "minimumReleaseAge" `
        -or $content -match "minimumReleaseAgeExclude" `
        -or $content -match "minimum release age" `
        -or $content -match "minimum-release-age"

    # pnpm can also surface ERR_PNPM_NO_MATCHING_VERSION when minimumReleaseAge
    # filters out all candidates. That code is also used for real missing
    # versions, so require age-gate context before prompting for a bypass.
    # https://github.com/pnpm/pnpm/blob/16cfde66ec71125d692ea828eba2a5f9b3cc54fc/deps/inspection/outdated/src/createManifestGetter.ts#L66-L76
    return $content -match "ERR_PNPM_NO_MATURE_MATCHING_VERSION" `
        -or $content -match "NO_MATURE_MATCHING_VERSION" `
        -or (($content -match "ERR_PNPM_NO_MATCHING_VERSION") -and $hasReleaseAgeText) `
        -or $hasReleaseAgeText
}

function Confirm-ReleaseAgeOverride {
    if ($env:CI -eq "true") {
        return $false
    }
    if (-not [Environment]::UserInteractive) {
        return $false
    }

    Write-Host ""
    Write-Warn "Your minimumReleaseAge setting prevented installing vite-plus@$ViteVersion."
    Write-Host "This setting helps protect against newly published compromised packages."
    Write-Host "Proceeding will disable this protection for this Vite+ install only."
    $response = Read-Host "Do you want to proceed? (y/N)"
    return $response -match "^(?i:y|yes)$"
}

function Write-ReleaseAgeOverride {
    # Append idempotently so a bridge registry line written for PR builds survives.
    $npmrc = Join-Path $VersionDir ".npmrc"
    if ((-not (Test-Path $npmrc)) -or (-not (Select-String -Path $npmrc -Pattern '^minimum-release-age=' -Quiet))) {
        Add-Content -Path $npmrc -Value "minimum-release-age=0"
    }
}

function Test-AbsoluteOverridePath {
    param([string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path)) {
        return $false
    }
    return [System.IO.Path]::IsPathRooted($Path)
}

function Test-VpDirOverrides {
    $values = @($env:VP_BIN_DIR, $env:VP_DATA_DIR, $env:VP_CACHE_DIR) |
        Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
    if ($values.Count -ne 0 -and $values.Count -ne 3) {
        Write-Error-Exit "Set VP_BIN_DIR, VP_DATA_DIR, and VP_CACHE_DIR together, or leave all three unset."
    }
    if ($values.Count -eq 3) {
        foreach ($name in @("VP_BIN_DIR", "VP_DATA_DIR", "VP_CACHE_DIR")) {
            $value = [Environment]::GetEnvironmentVariable($name)
            if (-not (Test-AbsoluteOverridePath $value)) {
                Write-Error-Exit "$name must be an absolute path."
            }
        }
    }
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

# Monolithic mapping: every category on one root.
function New-MonolithicLayout {
    param([string]$Root)
    return [pscustomobject]@{
        Kind = "single-root"
        DataDir = $Root
        ShimDir = Join-Path $Root "bin"
        CacheDir = Join-Path $Root "cache"
        ConfigDir = $Root
        StateDir = $Root
    }
}

function Set-LayoutVars {
    $script:InstallDir = $script:Layout.DataDir
    $script:ShimDir = $script:Layout.ShimDir
    $script:CacheDir = $script:Layout.CacheDir
    $script:ConfigDir = $script:Layout.ConfigDir
    $script:StateDir = $script:Layout.StateDir
    $script:NodeManagerBinDisplay = $script:ShimDir -replace [regex]::Escape($env:USERPROFILE), '~'
}

# Pre-split releases resolve all paths from VP_HOME, which defaults to
# %USERPROFILE%\.vite-plus. Install them in this monolithic root. This keeps
# environment setup, shims, trampolines, and installer paths consistent.
function Use-LegacyLayout {
    $userHome = Get-UserHomeDir
    if ([string]::IsNullOrWhiteSpace($userHome)) {
        Write-Error-Exit "Vite+ could not resolve the user home directory."
    }

    $root = if (Test-AbsoluteOverridePath $env:VP_HOME) {
        $env:VP_HOME
    } else {
        Join-Path $userHome ".vite-plus"
    }
    $script:Layout = New-MonolithicLayout $root
    Set-LayoutVars
}

# Record the resolved layout next to each trampoline.
function Write-ShimPointer {
    param(
        [string]$BinDir,
        [string]$DataDir,
        [string]$CacheDir,
        [string]$LayoutKind,
        [string]$Name = "vp"
    )
    $path = Join-Path $BinDir "$Name.shim"
    $utf8 = New-Object System.Text.UTF8Encoding $false
    $contents = "vite-plus-shim-v1`nlayout=$LayoutKind`ndata=$($DataDir.TrimEnd('\', '/'))`ncache=$($CacheDir.TrimEnd('\', '/'))`n"
    [System.IO.File]::WriteAllText($path, $contents, $utf8)
}

function Get-ShimPointerData {
    param([string]$Path)
    try {
        $contents = [System.IO.File]::ReadAllText($Path).Trim()
    } catch {
        return $null
    }
    if ([string]::IsNullOrWhiteSpace($contents)) {
        return $null
    }
    $lines = $contents -split "`r?`n"
    if ($lines[0] -ne "vite-plus-shim-v1") {
        return $null
    }
    foreach ($line in $lines) {
        if ($line.StartsWith("data=")) {
            return $line.Substring(5)
        }
    }
    return $null
}

function Normalize-InstallDir {
    param([string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path)) {
        return $Path
    }

    try {
        if (Test-Path -LiteralPath $Path -PathType Container) {
            return (Resolve-Path -LiteralPath $Path).ProviderPath.TrimEnd([System.IO.Path]::DirectorySeparatorChar, [System.IO.Path]::AltDirectorySeparatorChar)
        }

        return [System.IO.Path]::GetFullPath($Path).TrimEnd([System.IO.Path]::DirectorySeparatorChar, [System.IO.Path]::AltDirectorySeparatorChar)
    } catch {
        return $Path.TrimEnd([System.IO.Path]::DirectorySeparatorChar, [System.IO.Path]::AltDirectorySeparatorChar)
    }
}

function Test-SafeInstallDirToRemove {
    param([string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path)) {
        return $false
    }

    $normalized = Normalize-InstallDir $Path
    $root = [System.IO.Path]::GetPathRoot($normalized)
    # Do not use $home: PowerShell is case-insensitive and $HOME is read-only on 5.1.
    $userHome = Normalize-InstallDir $env:USERPROFILE
    $programFilesX86 = [Environment]::GetEnvironmentVariable("ProgramFiles(x86)")
    $unsafeDirs = @(
        $root
        $userHome
        (Normalize-InstallDir $env:SystemRoot)
        (Normalize-InstallDir $env:ProgramFiles)
        (Normalize-InstallDir $programFilesX86)
    ) | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }

    return $unsafeDirs -notcontains $normalized
}

function Test-VitePlusInstallDir {
    param([string]$Path)
    if (-not (Test-Path -LiteralPath $Path -PathType Container)) {
        return $false
    }

    $binDir = Join-Path $Path "bin"
    if (-not (Test-Path -LiteralPath $binDir -PathType Container)) {
        return $false
    }
    if (-not (Test-Path -LiteralPath (Join-Path $Path "current"))) {
        return $false
    }

    return (Test-Path -LiteralPath (Join-Path $binDir "vp.exe")) `
        -or (Test-Path -LiteralPath (Join-Path $binDir "vp.cmd")) `
        -or (Test-Path -LiteralPath (Join-Path $binDir "vp"))
}

function Get-PreviousInstallDir {
    if (-not $env:VP_HOME) {
        return $null
    }

    $vpCommand = Get-Command vp -CommandType Application,ExternalScript -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($null -eq $vpCommand) {
        return $null
    }

    $vpPath = $vpCommand.Path
    if (-not $vpPath) {
        return $null
    }

    $vpFileName = [System.IO.Path]::GetFileName($vpPath)
    if ($vpFileName -notin @("vp", "vp.exe", "vp.cmd")) {
        return $null
    }

    $oldDir = Normalize-InstallDir (Split-Path -Parent (Split-Path -Parent $vpPath))
    $newDir = Normalize-InstallDir $InstallDir
    if ($oldDir -eq $newDir) {
        return $null
    }
    if (-not (Test-SafeInstallDirToRemove $oldDir)) {
        return $null
    }
    if (-not (Test-VitePlusInstallDir $oldDir)) {
        return $null
    }

    return $oldDir
}

function Test-NestedInstallDir {
    param(
        [string]$OldDir,
        [string]$NewDir
    )
    if ([string]::IsNullOrWhiteSpace($OldDir) -or [string]::IsNullOrWhiteSpace($NewDir)) {
        return $false
    }

    $oldDir = Normalize-InstallDir $OldDir
    $newDir = Normalize-InstallDir $NewDir
    if ([string]::IsNullOrWhiteSpace($oldDir) -or [string]::IsNullOrWhiteSpace($newDir) -or $oldDir -eq $newDir) {
        return $false
    }

    # Normalize-InstallDir already trimmed trailing separators
    $oldPrefix = $oldDir + [System.IO.Path]::DirectorySeparatorChar
    $newPrefix = $newDir + [System.IO.Path]::DirectorySeparatorChar
    return $oldPrefix.StartsWith($newPrefix, [System.StringComparison]::OrdinalIgnoreCase) `
        -or $newPrefix.StartsWith($oldPrefix, [System.StringComparison]::OrdinalIgnoreCase)
}

function Prompt-RemovePreviousInstallDir {
    param([string]$PreviousInstallDir)
    if (-not $PreviousInstallDir) {
        return
    }
    if ($env:CI -eq "true") {
        return
    }
    if (-not [Environment]::UserInteractive) {
        return
    }

    Write-Host ""
    Write-Warn "Found a previous Vite+ install at $PreviousInstallDir."
    Write-Host "The new VP_HOME is $InstallDir."
    $response = Read-Host "Remove the previous install directory? (y/N)"
    if ($response -match "^(?i:y|yes)$") {
        $vpBin = Join-Path $PreviousInstallDir "current\bin\vp.exe"
        if (-not (Test-Path -LiteralPath $vpBin)) {
            Write-Warn "Could not remove previous Vite+ install at ${PreviousInstallDir}: vp binary not found."
            return
        }

        $previousVpHome = $env:VP_HOME
        try {
            $env:VP_HOME = $PreviousInstallDir
            $output = & $vpBin implode --yes 2>&1
            $exitCode = $LASTEXITCODE
        } catch {
            $output = $_
            $exitCode = 1
        } finally {
            $env:VP_HOME = $previousVpHome
        }

        if ($exitCode -eq 0) {
            Write-Success "Removed previous Vite+ install at $PreviousInstallDir."
        } else {
            Write-Warn "Could not remove previous Vite+ install at ${PreviousInstallDir}: $output"
        }
    }
}

# Resolve a PR number or commit SHA to the registry bridge's immutable commit
# version (0.0.0-commit.<sha>). A full commit SHA maps directly to the bridge's
# deterministic version; a PR number (or short ref) is resolved via the bridge
# download URL's `x-commit-key: <owner>:<repo>:<sha>` header (HEAD).
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

function Write-InstallFailure {
    param(
        [string]$LogPath,
        [int]$ExitCode = 0
    )

    if (Test-IsDllNotFoundExitCode $ExitCode) {
        $message = Get-DllNotFoundInstallMessage
        if ($env:CI -eq "true") {
            Write-Host "error: " -ForegroundColor Red -NoNewline
            Write-Host $message
            Exit-Installer
        }
        Write-Error-Exit $message
    }

    if ($env:CI -eq "true") {
        Write-Host "error: " -ForegroundColor Red -NoNewline
        Write-Host "Failed to install dependencies. Log output:"
        Get-Content -Path $LogPath | ForEach-Object { Write-Host $_ }
        Exit-Installer
    } else {
        Write-Error-Exit "Failed to install dependencies. See log for details: $LogPath"
    }
}

function Write-ReleaseAgeFailure {
    param([string]$LogPath)
    if ($env:CI -eq "true") {
        Write-Host "error: " -ForegroundColor Red -NoNewline
        Write-Host "Install blocked by your minimumReleaseAge setting. Log output:"
        Get-Content -Path $LogPath | ForEach-Object { Write-Host $_ }
    } else {
        Write-Error-Exit "Install blocked by your minimumReleaseAge setting. Wait until the package is old enough or adjust your package manager configuration explicitly. See log for details: $LogPath"
    }
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

# Cached package metadata
$script:PackageMetadata = $null

function Get-PackageMetadata {
    if ($null -eq $script:PackageMetadata) {
        $versionPath = if ($ViteVersion -eq "latest") { "latest" } else { $ViteVersion }
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

function Download-AndExtract {
    param(
        [string]$Url,
        [string]$DestDir,
        [string]$Filter
    )

    $tempFile = New-TemporaryFile
    try {
        # Suppress progress bar for cleaner output
        $ProgressPreference = 'SilentlyContinue'
        Invoke-WebRequest -Uri $Url -OutFile $tempFile

        # Create temp extraction directory
        $tempExtract = Join-Path $env:TEMP "vite-install-$(Get-Random)"
        New-Item -ItemType Directory -Force -Path $tempExtract | Out-Null

        # Extract using tar (available in Windows 10+)
        & "$env:SystemRoot\System32\tar.exe" -xzf $tempFile -C $tempExtract

        # Copy the specified file/directory
        $sourcePath = Join-Path (Join-Path $tempExtract "package") $Filter
        if (Test-Path $sourcePath) {
            Copy-Item -Path $sourcePath -Destination $DestDir -Recurse -Force
        }

        Remove-Item -Recurse -Force $tempExtract
    } finally {
        Remove-Item $tempFile -ErrorAction SilentlyContinue
    }
}

function Cleanup-OldVersions {
    param([string]$InstallDir)

    $maxVersions = 3
    # Only cleanup semver format directories (0.1.0, 1.2.3-beta.1, etc.)
    # This excludes 'current' symlink and non-semver directories like 'local-dev'
    $semverPattern = '^\d+\.\d+\.\d+(-[a-zA-Z0-9.-]+)?$'
    $versions = Get-ChildItem -Path $InstallDir -Directory -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -match $semverPattern }

    if ($null -eq $versions -or $versions.Count -le $maxVersions) {
        return
    }

    # Sort by creation time (oldest first) and select excess
    $toDelete = $versions |
        Sort-Object CreationTime |
        Select-Object -First ($versions.Count - $maxVersions)

    foreach ($old in $toDelete) {
        # Remove silently
        Remove-Item -Path $old.FullName -Recurse -Force
    }
}

function Remove-CurrentLink {
    param([string]$Path)

    try {
        $item = Get-Item -LiteralPath $Path -Force -ErrorAction Stop
    } catch [System.Management.Automation.ItemNotFoundException] {
        return
    }

    $isReparsePoint = ($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0

    try {
        if ($isReparsePoint) {
            if ($item.PSIsContainer) {
                [System.IO.Directory]::Delete($item.FullName)
            } else {
                [System.IO.File]::Delete($item.FullName)
            }
            return
        }

        Remove-Item -LiteralPath $item.FullName -Recurse -Force -ErrorAction Stop
    } catch {
        Write-Error-Exit "Failed to remove existing current link at ${Path}: $_"
    }
}

# Configure user PATH for the resolved shim directory
# Returns: "true" = added, "already" = already configured
function Configure-UserPath {
    $binPath = $ShimDir
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")

    if ($userPath -like "*$binPath*") {
        return "already"
    }

    $newPath = "$binPath;$userPath"
    try {
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        $env:Path = "$binPath;$env:Path"
        return "true"
    } catch {
        Write-Warn "Could not update user PATH automatically."
        return "failed"
    }
}

function Get-NushellVendorAutoloadDir {
    $nushellCommand = Get-Command nu -ErrorAction SilentlyContinue
    if ($null -eq $nushellCommand) {
        return $null
    }

    try {
        $dirsOutput = & $nushellCommand.Source -c '$nu.vendor-autoload-dirs | reverse | each {|dir| $dir } | str join (char nl)' 2>$null
    } catch {
        return $null
    }

    foreach ($dir in ($dirsOutput -split "\r?\n")) {
        if (-not [string]::IsNullOrWhiteSpace($dir)) {
            return $dir
        }
    }

    return $null
}

function Configure-Nushell {
    $autoloadDir = Get-NushellVendorAutoloadDir
    if ($null -eq $autoloadDir) {
        if ($null -eq (Get-Command nu -ErrorAction SilentlyContinue)) {
            return [pscustomobject]@{
                Status = "skipped"
                Message = "skipped (not installed)"
            }
        }

        return [pscustomobject]@{
            Status = "failed"
            Message = "failed (could not determine vendor autoload dir)"
        }
    }

    $autoloadFile = Join-Path $autoloadDir "vite-plus.nu"
    $nuEnvRef= (Join-Path $ConfigDir "env.nu") -replace [regex]::Escape($env:USERPROFILE), '~'
    $content = "# Vite+ bin (https://viteplus.dev)`n" + ("source '"+ $nuEnvRef +"'") + "`n"

    try {
        New-Item -ItemType Directory -Force -Path $autoloadDir | Out-Null
        if (Test-Path $autoloadFile) {
            $existing = Get-Content -Path $autoloadFile -Raw
            if ($existing -eq $content) {
                return [pscustomobject]@{
                    Status = "already"
                    Message = "already configured $autoloadFile"
                }
            }
        }

        [System.IO.File]::WriteAllText($autoloadFile, $content)
        return [pscustomobject]@{
            Status = "true"
            Message = "updated $autoloadFile"
        }
    } catch {
        Write-Warn "Could not configure Nushell automatically."
        return [pscustomobject]@{
            Status = "failed"
            Message = "failed $autoloadFile"
        }
    }
}

# Run vp env setup --refresh, showing output only on failure
function Refresh-Shims {
    param([string]$BinDir)
    $setupOutput = & "$BinDir\vp.exe" env setup --refresh 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Warn "Failed to refresh shims:"
        Write-Host "$setupOutput"
    }
}

# Return true only if this Vite+ install owns the existing Node executable.
# $ShimDir can be shared. The existence of node.exe does not permit replacement.
function Test-VitePlusNodeShim {
    $nodePath = Join-Path $ShimDir "node.exe"
    $pointerPath = Join-Path $ShimDir "node.shim"
    $hasNode = Test-Path -LiteralPath $nodePath -PathType Leaf
    $hasPointer = Test-Path -LiteralPath $pointerPath -PathType Leaf
    if (-not $hasNode -or -not $hasPointer) {
        return $false
    }

    $pointer = Get-ShimPointerData $pointerPath
    if ([string]::IsNullOrWhiteSpace($pointer)) {
        return $false
    }

    return (Normalize-InstallDir $pointer) -eq (Normalize-InstallDir $InstallDir)
}

# Setup Vite+ environment shims
# Returns: "true" = enabled, "false" = not enabled, "already" = already configured
function Setup-NodeManager {
    param([string]$BinDir)

    $binPath = $ShimDir

    # Explicit override via environment variable
    if ($env:VP_NODE_MANAGER -eq "yes") {
        Refresh-Shims -BinDir $BinDir
        return "true"
    } elseif ($env:VP_NODE_MANAGER -eq "no") {
        return "false"
    }

    # A foreign Node executable in a custom bin directory prevents automatic
    # enablement. The explicit setting or interactive prompt can permit
    # replacement.
    $foreignNodeInBin = $false
    if (Test-Path -LiteralPath (Join-Path $binPath "node.exe")) {
        if (Test-VitePlusNodeShim) {
            Refresh-Shims -BinDir $BinDir
            return "already"
        }
        $foreignNodeInBin = $true
    }

    # Auto-enable on CI or devcontainer environments
    # CI: standard CI environment variable (GitHub Actions, Travis, CircleCI, etc.)
    # CODESPACES: set by GitHub Codespaces (https://docs.github.com/en/codespaces)
    # REMOTE_CONTAINERS: set by VS Code Dev Containers extension
    # DEVPOD: set by DevPod (https://devpod.sh)
    $isAutomaticEnvironment = $env:CI -or $env:CODESPACES -or $env:REMOTE_CONTAINERS -or $env:DEVPOD
    if (-not $foreignNodeInBin -and $isAutomaticEnvironment) {
        Refresh-Shims -BinDir $BinDir
        return "true"
    }

    # Check if node is available on the system
    $nodeAvailable = $null -ne (Get-Command node -ErrorAction SilentlyContinue)

    # Auto-enable if no node available on system
    if (-not $nodeAvailable -and -not $foreignNodeInBin) {
        Refresh-Shims -BinDir $BinDir
        return "true"
    }

    # Prompt user in interactive mode
    # CI requires unattended setup. Some hosted PowerShell runners report an
    # interactive host process, so do not use that report in CI.
    $isInteractive = [Environment]::UserInteractive -and -not $env:CI
    if ($isInteractive) {
        Write-Host ""
        Write-Host "Would you like Vite+ to manage your Node.js and package-manager versions?"
        Write-Host "Vite+ adds ``node``, ``npm``, ``npx``, ``pnpm``, ``pnpx``, ``yarn``, ``yarnpkg``, ``bun``, and ``bunx`` shims to $NodeManagerBinDisplay."
        Write-Host "It selects the required version automatically."
        Write-Host "Opt out anytime with ``vp env off``."
        $response = Read-Host "Press Enter to accept (Y/n)"

        if ($response -eq '' -or $response -eq 'y' -or $response -eq 'Y') {
            Refresh-Shims -BinDir $BinDir
            return "true"
        }
    }

    return "false"
}

function Main {
    Write-Host ""
    Write-Host "Setting up " -NoNewline
    Write-Host "VITE+" -ForegroundColor Blue -NoNewline
    Write-Host "..."

    if ($PrVersion -and $LocalTgz) {
        Write-Error-Exit "VP_PR_VERSION and VP_LOCAL_TGZ cannot be used together"
    }

    Test-VpDirOverrides
    Enable-SetupVpLegacyCompatibility
    $previousInstallDir = $null

    # Suppress progress bars for cleaner output
    $ProgressPreference = 'SilentlyContinue'

    $arch = Get-Architecture
    $platform = "win32-$arch"

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
        if (Apply-DirsFromVp $LocalBinary) {
            Set-LayoutVars
        } else {
            Use-LegacyLayout
            Write-Info "The local vite-plus binary does not support the split directory layout. Vite+ will install it in $InstallDir."
        }
    } elseif ($PrVersion) {
        # Registry bridge mode: resolve the requested PR/SHA to the bridge's
        # immutable commit version (0.0.0-commit.<sha>), the clearly-defined test
        # version we install. The directory label stays non-semver so it keeps
        # out of Cleanup-OldVersions and makes the PR build obvious in ~/.vite-plus.
        $PrCommitVersion = Resolve-BridgeCommitVersion -Ref $PrVersion
        if (-not $PrCommitVersion) {
            Write-Error-Exit "Could not resolve a registry bridge build for $PrVersion"
        }
        $ViteVersion = "pkg-pr-new-$PrVersion"
        Write-Info "Using registry bridge build: $PrCommitVersion"
    } else {
        # Fetch package metadata and resolve version from npm
        $ViteVersion = Get-VersionFromMetadata
    }

    $binaryName = "vp.exe"

    # Download the CLI platform tarball before Vite+ selects the final layout.
    # The downloaded binary reports the layout that it supports.
    $platformTempExtract = $null
    if (-not $LocalTgz) {
        # npm registry or registry bridge (when PrVersion is set)
        $platformSuffix = Get-PlatformSuffix -Platform $platform
        if ($PrVersion) {
            # The registry bridge redirects this URL to the platform tarball for
            # the matching commit build (0.0.0-commit.<sha>).
            $platformUrl = "$BridgeDownloadBase/@voidzero-dev/vite-plus-cli-$platformSuffix@$PrVersion"
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
        } finally {
            Remove-Item $platformTempFile -ErrorAction SilentlyContinue
        }

        # Ask the downloaded binary for its layout through VP_DUMP_DIRS. A
        # pre-split release cannot report a layout. Give that release the
        # monolithic root so the installed PATH commands work.
        $packageDir = Join-Path $platformTempExtract "package"
        $binarySource = Join-Path $packageDir $binaryName
        if (Test-Path $binarySource) {
            # Remove Zone.Identifier (Mark of the Web) so the probe can run.
            Unblock-File -LiteralPath $binarySource
        }
        if ((Test-Path $binarySource) -and (Apply-DirsFromVp $binarySource)) {
            Set-LayoutVars
        } else {
            Use-LegacyLayout
            Write-Info "vite-plus $ViteVersion does not support the split directory layout. Vite+ will install it in $InstallDir."
        }
    }

    # Run layout migration checks after the payload resolves the category roots.
    # A pre-split payload selects the legacy layout first.
    $previousInstallDir = Get-PreviousInstallDir
    if ($previousInstallDir -and (Test-NestedInstallDir -OldDir $previousInstallDir -NewDir $InstallDir)) {
        Write-Error-Exit "The previous Vite+ install at $previousInstallDir overlaps with VP_HOME $InstallDir. Set VP_HOME to a directory that does not overlap. Alternatively, remove the previous install."
    }

    # Set up version-specific directories
    $VersionDir = "$InstallDir\$ViteVersion"
    $BinDir = "$VersionDir\bin"
    $CurrentLink = "$InstallDir\current"

    # Create bin directory
    New-Item -ItemType Directory -Force -Path $BinDir | Out-Null

    if ($LocalTgz) {
        # Local development mode: only need the binary
        Write-Info "Vite+ uses the local tarball: $LocalTgz"

        # Copy binary from LOCAL_BINARY env var (set by install-global-cli.ts)
        Copy-Item -Path $LocalBinary -Destination (Join-Path $BinDir $binaryName) -Force
        # Also copy trampoline shim binary if available (sibling to vp.exe)
        $shimSource = Join-Path (Split-Path $LocalBinary) "vp-shim.exe"
        if (Test-Path $shimSource) {
            Copy-Item -Path $shimSource -Destination (Join-Path $BinDir "vp-shim.exe") -Force
        }
    } else {
        # Copy binary to BinDir
        if (Test-Path $binarySource) {
            Copy-Item -Path $binarySource -Destination $BinDir -Force
        }
        # Also copy trampoline shim binary if present in the package
        $shimSource = Join-Path $packageDir "vp-shim.exe"
        if (Test-Path $shimSource) {
            Copy-Item -Path $shimSource -Destination $BinDir -Force
        }

        Remove-Item -Recurse -Force $platformTempExtract
    }

    # Remove Zone.Identifier (Mark of the Web) from downloaded binaries so
    # Windows SmartScreen / Defender won't block execution.
    Get-ChildItem -Path $BinDir -Filter "*.exe" | Unblock-File

    # Generate wrapper package.json that declares vite-plus as a dependency.
    # pnpm will install vite-plus and all transitive deps via `vp install`.
    # The packageManager field pins pnpm to a known-good version.
    # In PR mode, pin vite-plus to the bridge's clearly-defined commit version and
    # resolve it (plus its platform binaries and transitive deps) through the
    # bridge registry written to .npmrc below. The bridge rewrites a preview
    # tarball's transitive deps to versions, not self-contained URLs, so a full
    # install must go through the registry rather than the bare download URL.
    $vitePlusSpec = if ($PrVersion) { $PrCommitVersion } else { $ViteVersion }
    if ($PrVersion) {
        # Bridge registry; drop any stale wrapper lockfile (see install.sh for why):
        # the reused pkg-pr-new-<ref> dir must re-resolve a lockfile matching the
        # spec we just wrote, not fail under CI's frozen-lockfile default.
        Set-Content -Path (Join-Path $VersionDir ".npmrc") -Value "registry=$BridgeRegistry"
        Remove-Item -Path (Join-Path $VersionDir "pnpm-lock.yaml") -ErrorAction SilentlyContinue
    }
    $wrapperJson = @{
        name = "vp-global"
        version = $ViteVersion
        private = $true
        packageManager = "pnpm@10.33.0"
        dependencies = @{
            "vite-plus" = $vitePlusSpec
        }
    } | ConvertTo-Json -Depth 10
    Set-Content -Path (Join-Path $VersionDir "package.json") -Value $wrapperJson

    # Install production dependencies (skip if VP_SKIP_DEPS_INSTALL is set,
    # e.g. during local dev where install-global-cli.ts handles deps separately)
    if (-not $env:VP_SKIP_DEPS_INSTALL) {
        $installLog = Join-Path $VersionDir "install.log"
        Push-Location $VersionDir
        try {
            # Use cmd /c so CI=true is scoped to the child process only,
            # avoiding leaking it into the user's shell session.
            # Do not pass --silent to the inner install: pnpm suppresses the
            # release-age error body in silent mode, which would leave
            # install.log empty and make the release-age gate impossible to
            # detect. Output is already captured to install.log here.
            $output = cmd /c "set CI=true && `"$BinDir\vp.exe`" install" 2>&1
            $installExitCode = $LASTEXITCODE
            $output | Out-File $installLog
            if ($installExitCode -ne 0) {
                if (Test-ReleaseAgeError $installLog) {
                    if (Confirm-ReleaseAgeOverride) {
                        # Write the override only after explicit consent, then retry once.
                        Write-ReleaseAgeOverride
                        $retryOutput = cmd /c "set CI=true && `"$BinDir\vp.exe`" install" 2>&1
                        $retryExitCode = $LASTEXITCODE
                        $retryOutput | Out-File $installLog
                        if ($retryExitCode -ne 0) {
                            Write-InstallFailure -LogPath $installLog -ExitCode $retryExitCode
                        }
                    } else {
                        Write-ReleaseAgeFailure $installLog
                        Exit-Installer
                    }
                } else {
                    Write-InstallFailure -LogPath $installLog -ExitCode $installExitCode
                }
            }
        } finally {
            Pop-Location
        }
    }

    # Create/update current junction (symlink)
    Remove-CurrentLink $CurrentLink
    # Create new junction pointing to the version directory
    cmd /c mklink /J "$CurrentLink" "$VersionDir" | Out-Null

    # Create user bin directory and vp wrapper (always done)
    New-Item -ItemType Directory -Force -Path $ShimDir | Out-Null
    $trampolineSrc = "$VersionDir\bin\vp-shim.exe"
    if (Test-Path $trampolineSrc) {
        # New versions: use trampoline exe to avoid "Terminate batch job (Y/N)?" on Ctrl+C
        Copy-Item -Path $trampolineSrc -Destination (Join-Path $ShimDir "vp.exe") -Force
        Write-ShimPointer -BinDir $ShimDir -DataDir $InstallDir -CacheDir $CacheDir -LayoutKind $Layout.Kind -Name "vp"
        # Remove legacy .cmd and shell script wrappers from previous versions
        foreach ($legacy in @((Join-Path $ShimDir "vp.cmd"), (Join-Path $ShimDir "vp"))) {
            if (Test-Path $legacy) {
                Remove-Item -Path $legacy -Force -ErrorAction SilentlyContinue
            }
        }
    } else {
        # Pre-trampoline versions: fall back to legacy .cmd and shell script wrappers.
        # Remove any stale trampoline .exe shims left by a newer install — .exe wins
        # over .cmd on Windows PATH, so leftover trampolines would bypass the wrappers.
        foreach ($stale in @("vp.exe", "node.exe", "npm.exe", "npx.exe", "corepack.exe", "vpx.exe", "vpr.exe")) {
            $stalePath = Join-Path $ShimDir $stale
            if (Test-Path $stalePath) {
                Remove-Item -Path $stalePath -Force -ErrorAction SilentlyContinue
            }
        }
        # Pin VP_HOME to the data root. In a split install, $ShimDir is not
        # `$InstallDir\bin`. Thus, `%~dp0..` would not find `<data>\current`.
        $wrapperContent = @"
@echo off
set VP_HOME=$InstallDir
"%VP_HOME%\current\bin\vp.exe" %*
exit /b %ERRORLEVEL%
"@
        Set-Content -Path (Join-Path $ShimDir "vp.cmd") -Value $wrapperContent -NoNewline

        # Also create shell script wrapper for Git Bash/MSYS
        $installDirUnix = $InstallDir -replace '\\', '/'
        $shContent = @"
#!/bin/sh
VP_HOME="$installDirUnix"
export VP_HOME
exec "`$VP_HOME/current/bin/vp.exe" "`$@"
"@
        Set-Content -Path (Join-Path $ShimDir "vp") -Value $shContent -NoNewline
    }

    # Cleanup old versions
    Cleanup-OldVersions -InstallDir $InstallDir

    # Create env files under the resolved config dir (matches install.sh).
    # Use current\bin\vp.exe directly instead of the trampoline so a Windows
    # refresh cannot overwrite the running wrapper.
    $vpBin = Join-Path $InstallDir "current\bin\vp.exe"
    if (Test-Path -LiteralPath $vpBin) {
        & $vpBin env setup --env-only | Out-Null
    }

    # Setup Node.js version manager (shims) - separate component
    $nodeManagerResult = Setup-NodeManager -BinDir $BinDir
    if ($nodeManagerResult -eq "true") {
        $previousErrorActionPreference = $ErrorActionPreference
        try {
            $ErrorActionPreference = "Continue"
            & $vpBin env on *> $null
            $preferenceExitCode = $LASTEXITCODE
        } finally {
            $ErrorActionPreference = $previousErrorActionPreference
        }
        if ($preferenceExitCode -ne 0) {
            Write-Warn "Failed to record environment management preference."
        }
        $global:LASTEXITCODE = 0
    }

    Prompt-RemovePreviousInstallDir -PreviousInstallDir $previousInstallDir

    # Configure shell access after the install is otherwise complete.
    $pathResult = Configure-UserPath
    $nushellResult = Configure-Nushell

    # Use ~ when an install location is under USERPROFILE. Otherwise, show the
    # full path.
    $displayDataDir = $InstallDir -replace [regex]::Escape($env:USERPROFILE), '~'
    $displayBinDir = $ShimDir -replace [regex]::Escape($env:USERPROFILE), '~'
    $displayConfigDir = $ConfigDir -replace [regex]::Escape($env:USERPROFILE), '~'

    # ANSI color codes for consistent output
    $e = [char]27
    $GREEN = "$e[32m"
    $YELLOW = "$e[33m"
    $BRIGHT_BLUE = "$e[94m"
    $BOLD = "$e[1m"
    $DIM = "$e[2m"
    $BOLD_BRIGHT_BLUE = "$e[1;94m"
    $NC = "$e[0m"
    $CHECKMARK = [char]0x2714

    # Print success message
    Write-Host ""
    Write-Host "${GREEN}${CHECKMARK}${NC} ${BOLD_BRIGHT_BLUE}VITE+${NC} successfully installed!"
    Write-Host ""
    Write-Host "  The Unified Toolchain for the Web."
    Write-Host ""
    Write-Host "  ${BOLD}Get started:${NC}"
    Write-Host "    ${BRIGHT_BLUE}vp create${NC}       Create a new project"
    Write-Host "    ${BRIGHT_BLUE}vp env${NC}          Manage Node.js and package managers"
    Write-Host "    ${BRIGHT_BLUE}vp install${NC}      Install dependencies"
    Write-Host "    ${BRIGHT_BLUE}vp migrate${NC}      Migrate to Vite+"

    # Show Node.js manager status
    if ($nodeManagerResult -eq "true" -or $nodeManagerResult -eq "already") {
        Write-Host ""
        Write-Host "  Vite+ is now managing Node.js and package managers via ${BRIGHT_BLUE}vp env${NC}."
        Write-Host "  Run ${BRIGHT_BLUE}vp env doctor${NC} to verify your setup, or ${BRIGHT_BLUE}vp env off${NC} to opt out."
    }

    Write-Host ""
    Write-Host "  Run ${BRIGHT_BLUE}vp help${NC} to see available commands."

    Write-Host ""
    Write-Host "  ${BOLD}Install locations:${NC}"
    Write-Host "    Data directory: $displayDataDir"
    Write-Host "    Bin directory:  $displayBinDir"

    Write-Host ""
    Write-Host "  Shell configuration:"
    switch ($pathResult) {
        "true" { Write-Host "    - Windows PATH: updated" }
        "already" { Write-Host "    - Windows PATH: already configured" }
        "failed" { Write-Host "    - Windows PATH: failed" }
        default { Write-Host "    - Windows PATH: skipped" }
    }
    if ($nushellResult.Status -ne "skipped") {
      Write-Host "    - Nushell: $($nushellResult.Message)"
    }

    # Show note if PATH or Nushell was updated
    if ($pathResult -eq "true" -or $nushellResult.Status -eq "true") {
        Write-Host ""
        Write-Host "  Note: Restart your terminal and IDE for changes to take effect."
    }

    # Show manual PATH/Nushell instructions if anything still needs manual setup
    if ($pathResult -eq "failed" -or $nushellResult.Status -eq "failed") {
        Write-Host ""
        Write-Host "  ${YELLOW}note${NC}: Some shells still need manual setup."
        Write-Host ""
        if ($pathResult -eq "failed") {
            Write-Host "  To use vp in Powershell/cmd, manually add it to your PATH:"
            Write-Host ""
            Write-Host "    [Environment]::SetEnvironmentVariable('Path', '$ShimDir;' + [Environment]::GetEnvironmentVariable('Path', 'User'), 'User')"
            Write-Host ""
        }
        if ($nushellResult.Status -eq "failed") {
            Write-Host "  To use vp in Nushell, create a vite-plus.nu file in your preferred vendor autoload directory with:"
            Write-Host ""
            Write-Host "    source '$displayConfigDir\env.nu'"
            Write-Host ""
        }
        Write-Host "  Or run vp directly:"
        Write-Host ""
        Write-Host "    & `"$(Join-Path $ShimDir 'vp.exe')`""
    }

    Write-Host ""
}

function Apply-DirsFromVp {
    param([string]$VpBinary)
    $previous = $env:VP_DUMP_DIRS
    $env:VP_DUMP_DIRS = "1"
    try {
        $out = & $VpBinary 2>$null
    } finally {
        if ($null -eq $previous) {
            Remove-Item Env:VP_DUMP_DIRS -ErrorAction SilentlyContinue
        } else {
            $env:VP_DUMP_DIRS = $previous
        }
    }
    $map = @{}
    foreach ($line in @($out)) {
        $text = "$line"
        $sep = $text.IndexOf("`t")
        if ($sep -lt 1) {
            continue
        }
        $map[$text.Substring(0, $sep)] = $text.Substring($sep + 1)
    }
    if (-not $map['data'] -or -not $map['bin'] -or -not $map['cache'] -or -not $map['config'] -or -not $map['state']) {
        return $false
    }
    $layoutKind = $map['layout']
    if ($layoutKind -ne 'single-root' -and $layoutKind -ne 'split') {
        $isSingleRoot = (Normalize-InstallDir $map['bin']) -eq (Normalize-InstallDir (Join-Path $map['data'] 'bin')) `
            -and (Normalize-InstallDir $map['cache']) -eq (Normalize-InstallDir (Join-Path $map['data'] 'cache')) `
            -and (Normalize-InstallDir $map['config']) -eq (Normalize-InstallDir $map['data']) `
            -and (Normalize-InstallDir $map['state']) -eq (Normalize-InstallDir $map['data'])
        $layoutKind = if ($isSingleRoot) { 'single-root' } else { 'split' }
    }
    $script:Layout = [pscustomobject]@{
        Kind = $layoutKind
        DataDir = $map['data']
        ShimDir = $map['bin']
        CacheDir = $map['cache']
        ConfigDir = $map['config']
        StateDir = $map['state']
    }
    return $true
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
