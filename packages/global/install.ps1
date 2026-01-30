# Vite+ CLI Installer for Windows
# https://viteplus.dev/install.ps1
#
# Usage:
#   irm https://viteplus.dev/install.ps1 | iex
#
# Environment variables:
#   VITE_PLUS_VERSION - Version to install (default: latest)
#   VITE_PLUS_INSTALL_DIR - Installation directory (default: $env:USERPROFILE\.vite-plus)
#   NPM_CONFIG_REGISTRY - Custom npm registry URL (default: https://registry.npmjs.org)

$ErrorActionPreference = "Stop"

$ViteVersion = if ($env:VITE_PLUS_VERSION) { $env:VITE_PLUS_VERSION } else { "latest" }
$InstallDir = if ($env:VITE_PLUS_INSTALL_DIR) { $env:VITE_PLUS_INSTALL_DIR } else { "$env:USERPROFILE\.vite-plus" }
# npm registry URL (strip trailing slash if present)
$NpmRegistry = if ($env:NPM_CONFIG_REGISTRY) { $env:NPM_CONFIG_REGISTRY.TrimEnd('/') } else { "https://registry.npmjs.org" }

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

function Get-LatestVersion {
    try {
        $response = Invoke-RestMethod "$NpmRegistry/vite-plus-cli/latest"
        return $response.version
    } catch {
        Write-Error-Exit "Failed to fetch latest version from npm registry: $_"
    }
}

function Download-AndExtract {
    param(
        [string]$Url,
        [string]$DestDir,
        [string]$Filter
    )

    Write-Info "Downloading from $Url"

    $tempFile = New-TemporaryFile
    try {
        Invoke-WebRequest -Uri $Url -OutFile $tempFile -UseBasicParsing

        # Create temp extraction directory
        $tempExtract = Join-Path $env:TEMP "vite-install-$(Get-Random)"
        New-Item -ItemType Directory -Force -Path $tempExtract | Out-Null

        # Extract using tar (available in Windows 10+)
        tar -xzf $tempFile -C $tempExtract

        # Copy the specified file/directory
        $sourcePath = Join-Path $tempExtract "package" $Filter
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
    $versions = Get-ChildItem -Path $InstallDir -Directory -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -ne "current" }

    if ($null -eq $versions -or $versions.Count -le $maxVersions) {
        return
    }

    # Sort by creation time (oldest first) and select excess
    $toDelete = $versions |
        Sort-Object CreationTime |
        Select-Object -First ($versions.Count - $maxVersions)

    foreach ($old in $toDelete) {
        Write-Info "Removing old version: $($old.Name)"
        Remove-Item -Path $old.FullName -Recurse -Force
    }
}

function Main {
    Write-Host ""
    Write-Host "  Vite+ CLI Installer"
    Write-Host ""

    $arch = Get-Architecture
    Write-Info "Detected architecture: win32-$arch"

    # Get version
    if ($ViteVersion -eq "latest") {
        Write-Info "Fetching latest version..."
        $ViteVersion = Get-LatestVersion
    }
    Write-Info "Installing vite-plus-cli v$ViteVersion"

    # Set up version-specific directories
    $VersionDir = "$InstallDir\$ViteVersion"
    $BinDir = "$VersionDir\bin"
    $DistDir = "$VersionDir\dist"
    $CurrentLink = "$InstallDir\current"

    # Package name (follows napi-rs convention)
    if ($arch -eq "arm64") {
        Write-Error-Exit "win32-arm64 is not currently supported. Only win32-x64 is supported."
    }
    $packageSuffix = "win32-$arch-msvc"
    $packageName = "@voidzero-dev/vite-plus-cli-$packageSuffix"
    $binaryName = "vp.exe"

    # Create directories
    Write-Info "Creating directories..."
    New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
    New-Item -ItemType Directory -Force -Path $DistDir | Out-Null

    # Download and extract native binary and .node files from platform package
    $platformUrl = "$NpmRegistry/$packageName/-/vite-plus-cli-$packageSuffix-$ViteVersion.tgz"
    Write-Info "Downloading platform package..."

    $platformTempFile = New-TemporaryFile
    try {
        Invoke-WebRequest -Uri $platformUrl -OutFile $platformTempFile -UseBasicParsing

        # Create temp extraction directory
        $platformTempExtract = Join-Path $env:TEMP "vite-platform-$(Get-Random)"
        New-Item -ItemType Directory -Force -Path $platformTempExtract | Out-Null

        # Extract the package
        tar -xzf $platformTempFile -C $platformTempExtract

        # Copy binary to BinDir
        $binarySource = Join-Path $platformTempExtract "package" $binaryName
        if (Test-Path $binarySource) {
            Copy-Item -Path $binarySource -Destination $BinDir -Force
        }

        # Copy .node files to DistDir (delete existing first to avoid system cache issues)
        $nodeFilesPath = Join-Path $platformTempExtract "package"
        Get-ChildItem -Path $nodeFilesPath -Filter "*.node" -ErrorAction SilentlyContinue | ForEach-Object {
            $destFile = Join-Path $DistDir $_.Name
            if (Test-Path $destFile) {
                Remove-Item -Path $destFile -Force
            }
            Copy-Item -Path $_.FullName -Destination $DistDir -Force
        }

        Remove-Item -Recurse -Force $platformTempExtract
    } finally {
        Remove-Item $platformTempFile -ErrorAction SilentlyContinue
    }

    # Download and extract JS bundle
    $mainUrl = "$NpmRegistry/vite-plus-cli/-/vite-plus-cli-$ViteVersion.tgz"
    Write-Info "Downloading JS scripts..."

    $mainTempFile = New-TemporaryFile
    try {
        Invoke-WebRequest -Uri $mainUrl -OutFile $mainTempFile -UseBasicParsing

        # Create temp extraction directory
        $mainTempExtract = Join-Path $env:TEMP "vite-main-$(Get-Random)"
        New-Item -ItemType Directory -Force -Path $mainTempExtract | Out-Null

        # Extract the package
        tar -xzf $mainTempFile -C $mainTempExtract

        # Copy dist contents to DistDir
        $distSource = Join-Path $mainTempExtract "package" "dist" "*"
        if (Test-Path $distSource) {
            Copy-Item -Path $distSource -Destination $DistDir -Recurse -Force
        }

        # Copy package.json to VersionDir for devEngines.runtime configuration
        $packageJsonSource = Join-Path $mainTempExtract "package" "package.json"
        if (Test-Path $packageJsonSource) {
            Copy-Item -Path $packageJsonSource -Destination $VersionDir -Force
        }

        Remove-Item -Recurse -Force $mainTempExtract
    } finally {
        Remove-Item $mainTempFile -ErrorAction SilentlyContinue
    }

    # Create/update current junction (symlink)
    if (Test-Path $CurrentLink) {
        # Remove existing junction
        cmd /c rmdir "$CurrentLink" 2>$null
        Remove-Item -Path $CurrentLink -Force -ErrorAction SilentlyContinue
    }
    # Create new junction pointing to the version directory
    cmd /c mklink /J "$CurrentLink" "$VersionDir" | Out-Null

    # Cleanup old versions
    Cleanup-OldVersions -InstallDir $InstallDir

    Write-Success "Vite+ CLI installed to $VersionDir"

    # Update PATH
    Write-Host ""
    $pathToAdd = "$InstallDir\current\bin"
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")

    # Check if we need to update PATH
    $needsPathUpdate = $true
    if ($userPath -like "*$pathToAdd*") {
        $needsPathUpdate = $false
    }

    if ($needsPathUpdate) {
        $newPath = "$pathToAdd;$userPath"
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        $env:Path = "$pathToAdd;$env:Path"
        Write-Success "PATH has been updated"
        Write-Host ""
        Write-Host "  Restart your terminal to use vp, or run:"
        Write-Host ""
        Write-Host "    `$env:Path = `"$pathToAdd;`$env:Path`""
    } else {
        Write-Info "PATH already contains $pathToAdd"
    }

    Write-Host ""
    Write-Host "  Then run:"
    Write-Host ""
    Write-Host "    vp --version"
    Write-Host ""
}

Main
