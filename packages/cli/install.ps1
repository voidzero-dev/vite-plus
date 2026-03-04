# Vite+ CLI Installer for Windows
# https://viteplus.dev/install.ps1
#
# Usage:
#   irm https://viteplus.dev/install.ps1 | iex
#
# Environment variables:
#   VITE_PLUS_VERSION - Version to install (default: latest)
#   VITE_PLUS_HOME - Installation directory (default: $env:USERPROFILE\.vite-plus)
#   NPM_CONFIG_REGISTRY - Custom npm registry URL (default: https://registry.npmjs.org)
#   VITE_PLUS_LOCAL_TGZ - Path to local vite-plus.tgz (for development/testing)

$ErrorActionPreference = "Stop"

$ViteVersion = if ($env:VITE_PLUS_VERSION) { $env:VITE_PLUS_VERSION } else { "latest" }
$InstallDir = if ($env:VITE_PLUS_HOME) { $env:VITE_PLUS_HOME } else { "$env:USERPROFILE\.vite-plus" }
# npm registry URL (strip trailing slash if present)
$NpmRegistry = if ($env:NPM_CONFIG_REGISTRY) { $env:NPM_CONFIG_REGISTRY.TrimEnd('/') } else { "https://registry.npmjs.org" }
# Local tarball for development/testing
$LocalTgz = $env:VITE_PLUS_LOCAL_TGZ
# Local binary path (set by install-global-cli.ts for local dev)
$LocalBinary = $env:VITE_PLUS_LOCAL_BINARY

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

function Write-Error-Exit {
    param([string]$Message)
    Write-Host "error: " -ForegroundColor Red -NoNewline
    Write-Host $Message
    exit 1
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
            # Try to extract npm error message from response
            $errorMsg = $_.ErrorDetails.Message
            if ($errorMsg) {
                try {
                    $errorJson = $errorMsg | ConvertFrom-Json
                    if ($errorJson.error) {
                        Write-Error-Exit "Failed to fetch version '${versionPath}': $($errorJson.error)"
                    }
                } catch {
                    # JSON parsing failed, fall through to generic error
                }
            }
            Write-Error-Exit "Failed to fetch package metadata from: $metadataUrl`nError: $_"
        }
        # Check for error in successful response
        # npm can return {"error":"..."} object or a plain string like "version not found: test"
        if ($script:PackageMetadata -is [string]) {
            # Plain string response means error
            Write-Error-Exit "Failed to fetch version '${versionPath}': $script:PackageMetadata"
        }
        if ($script:PackageMetadata.error) {
            Write-Error-Exit "Failed to fetch version '${versionPath}': $($script:PackageMetadata.error)"
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

    $maxVersions = 5
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

# Configure user PATH for ~/.vite-plus/bin
# Returns: "true" = added, "already" = already configured
function Configure-UserPath {
    $binPath = "$InstallDir\bin"
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")

    if ($userPath -like "*$binPath*") {
        return "already"
    }

    $newPath = "$binPath;$userPath"
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    $env:Path = "$binPath;$env:Path"
    return "true"
}

# Setup Node.js version manager (node/npm/npx shims)
# Returns: "true" = enabled, "false" = not enabled, "already" = already configured
function Setup-NodeManager {
    param([string]$BinDir)

    $binPath = "$InstallDir\bin"

    # Check if Vite+ is already managing Node.js (bin\node.exe exists)
    if (Test-Path "$binPath\node.exe") {
        # Already managing Node.js, just refresh shims
        & "$BinDir\vp.exe" env setup --refresh | Out-Null
        return "already"
    }

    # Auto-enable on CI environment
    if ($env:CI) {
        & "$BinDir\vp.exe" env setup --refresh | Out-Null
        return "true"
    }

    # Check if node is available on the system
    $nodeAvailable = $null -ne (Get-Command node -ErrorAction SilentlyContinue)

    # Auto-enable if no node available on system
    if (-not $nodeAvailable) {
        & "$BinDir\vp.exe" env setup --refresh | Out-Null
        return "true"
    }

    # Prompt user in interactive mode
    $isInteractive = [Environment]::UserInteractive
    if ($isInteractive) {
        Write-Host ""
        Write-Host "Would you want Vite+ to manage Node.js versions?"
        $response = Read-Host "Press Enter to accept (Y/n)"

        if ($response -eq '' -or $response -eq 'y' -or $response -eq 'Y') {
            & "$BinDir\vp.exe" env setup --refresh | Out-Null
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
    } else {
        # Fetch package metadata and resolve version from npm
        $ViteVersion = Get-VersionFromMetadata
    }

    # Set up version-specific directories
    $VersionDir = "$InstallDir\$ViteVersion"
    $BinDir = "$VersionDir\bin"
    $CurrentLink = "$InstallDir\current"

    $binaryName = "vp.exe"

    # Create bin directory
    New-Item -ItemType Directory -Force -Path $BinDir | Out-Null

    if ($LocalTgz) {
        # Local development mode: only need the binary
        Write-Info "Using local tarball: $LocalTgz"

        # Copy binary from LOCAL_BINARY env var (set by install-global-cli.ts)
        if ($LocalBinary -and (Test-Path $LocalBinary)) {
            Copy-Item -Path $LocalBinary -Destination (Join-Path $BinDir $binaryName) -Force
        } else {
            Write-Error-Exit "VITE_PLUS_LOCAL_BINARY must be set when using VITE_PLUS_LOCAL_TGZ"
        }
    } else {
        # Download from npm registry — extract only the vp binary from CLI platform package
        $platformSuffix = Get-PlatformSuffix -Platform $platform
        $packageName = "@voidzero-dev/vite-plus-cli-$platformSuffix"
        $platformUrl = "$NpmRegistry/$packageName/-/vite-plus-cli-$platformSuffix-$ViteVersion.tgz"

        $platformTempFile = New-TemporaryFile
        try {
            Invoke-WebRequest -Uri $platformUrl -OutFile $platformTempFile

            # Create temp extraction directory
            $platformTempExtract = Join-Path $env:TEMP "vite-platform-$(Get-Random)"
            New-Item -ItemType Directory -Force -Path $platformTempExtract | Out-Null

            # Extract the package
            & "$env:SystemRoot\System32\tar.exe" -xzf $platformTempFile -C $platformTempExtract

            # Copy binary to BinDir
            $binarySource = Join-Path (Join-Path $platformTempExtract "package") $binaryName
            if (Test-Path $binarySource) {
                Copy-Item -Path $binarySource -Destination $BinDir -Force
            }

            Remove-Item -Recurse -Force $platformTempExtract
        } finally {
            Remove-Item $platformTempFile -ErrorAction SilentlyContinue
        }
    }

    # Generate wrapper package.json that declares vite-plus as a dependency.
    # npm will install vite-plus and all transitive deps via `vp install`.
    $wrapperJson = @{
        name = "vp-global"
        version = $ViteVersion
        private = $true
        dependencies = @{
            "vite-plus" = $ViteVersion
        }
    } | ConvertTo-Json -Depth 10
    Set-Content -Path (Join-Path $VersionDir "package.json") -Value $wrapperJson

    # Install production dependencies (skip if VITE_PLUS_SKIP_DEPS_INSTALL is set,
    # e.g. during local dev where install-global-cli.ts handles deps separately)
    if (-not $env:VITE_PLUS_SKIP_DEPS_INSTALL) {
        Push-Location $VersionDir
        try {
            $env:CI = "true"
            & "$BinDir\vp.exe" install --silent
        } finally {
            Pop-Location
        }
    }

    # Create/update current junction (symlink)
    if (Test-Path $CurrentLink) {
        # Remove existing junction
        cmd /c rmdir "$CurrentLink" 2>$null
        Remove-Item -Path $CurrentLink -Force -ErrorAction SilentlyContinue
    }
    # Create new junction pointing to the version directory
    cmd /c mklink /J "$CurrentLink" "$VersionDir" | Out-Null

    # Create bin directory and vp.cmd wrapper (always done)
    # Set VITE_PLUS_HOME so the vp binary knows its home directory
    New-Item -ItemType Directory -Force -Path "$InstallDir\bin" | Out-Null
    $wrapperContent = @"
@echo off
set VITE_PLUS_HOME=%~dp0..
"%VITE_PLUS_HOME%\current\bin\vp.exe" %*
exit /b %ERRORLEVEL%
"@
    Set-Content -Path "$InstallDir\bin\vp.cmd" -Value $wrapperContent -NoNewline

    # Create shell script wrapper for Git Bash (vp without extension)
    # Note: We call vp.exe directly (not via symlink) because Windows symlinks
    # require admin privileges and Git Bash symlink support is unreliable
    $shContent = @"
#!/bin/sh
VITE_PLUS_HOME="`$(dirname "`$(dirname "`$(readlink -f "`$0" 2>/dev/null || echo "`$0")")")"
export VITE_PLUS_HOME
exec "`$VITE_PLUS_HOME/current/bin/vp.exe" "`$@"
"@
    Set-Content -Path "$InstallDir\bin\vp" -Value $shContent -NoNewline

    # Cleanup old versions
    Cleanup-OldVersions -InstallDir $InstallDir

    # Configure user PATH (always attempted)
    $pathResult = Configure-UserPath

    # Setup Node.js version manager (shims) - separate component
    $nodeManagerResult = Setup-NodeManager -BinDir $BinDir

    # Use ~ shorthand if install dir is under USERPROFILE, otherwise show full path
    $displayDir = $InstallDir -replace [regex]::Escape($env:USERPROFILE), '~'

    # ANSI color codes for consistent output
    $e = [char]27
    $GREEN = "$e[32m"
    $BRIGHT_BLUE = "$e[94m"
    $BOLD = "$e[1m"
    $DIM = "$e[2m"
    $BOLD_BRIGHT_BLUE = "$e[1;94m"
    $NC = "$e[0m"

    # Print success message
    Write-Host ""
    Write-Host "${GREEN}✔${NC} ${BOLD_BRIGHT_BLUE}VITE+${NC} successfully installed!"
    Write-Host ""
    Write-Host "  The Unified Toolchain for the Web."
    Write-Host ""
    Write-Host "  ${BOLD}Get started:${NC}"
    Write-Host "    ${BRIGHT_BLUE}vp create${NC}       Create a new project"
    Write-Host "    ${BRIGHT_BLUE}vp env${NC}          Manage Node.js versions"
    Write-Host "    ${BRIGHT_BLUE}vp install${NC}      Install dependencies"
    Write-Host "    ${BRIGHT_BLUE}vp dev${NC}          Start dev server"

    # Show Node.js manager status
    if ($nodeManagerResult -eq "true" -or $nodeManagerResult -eq "already") {
        Write-Host ""
        Write-Host "  Node.js is now managed by Vite+ (via ${BRIGHT_BLUE}vp env${NC})."
        Write-Host "  Run ${BRIGHT_BLUE}vp env doctor${NC} to verify your setup."
    }

    Write-Host ""
    Write-Host "  Run ${BRIGHT_BLUE}vp help${NC} for more information."

    # Show note if PATH was updated
    if ($pathResult -eq "true") {
        Write-Host ""
        Write-Host "  Note: Restart your terminal and IDE for changes to take effect."
    }

    Write-Host ""
}

Main
