# Vite+ CLI Installer for Windows
# https://viteplus.dev/install.ps1
#
# Usage:
#   irm https://viteplus.dev/install.ps1 | iex
#
# Environment variables:
#   VITE_VERSION - Version to install (default: latest)
#   VITE_INSTALL_DIR - Installation directory (default: $env:USERPROFILE\.vite)
#   npm_config_registry - Custom npm registry URL (default: https://registry.npmjs.org)

$ErrorActionPreference = "Stop"

$ViteVersion = if ($env:VITE_VERSION) { $env:VITE_VERSION } else { "latest" }
$InstallDir = if ($env:VITE_INSTALL_DIR) { $env:VITE_INSTALL_DIR } else { "$env:USERPROFILE\.vite" }
$BinDir = "$InstallDir\bin"
$DistDir = "$InstallDir\dist"
# npm registry URL (strip trailing slash if present)
$NpmRegistry = if ($env:npm_config_registry) { $env:npm_config_registry.TrimEnd('/') } else { "https://registry.npmjs.org" }

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

    # Download and extract native binary
    $binaryUrl = "$NpmRegistry/$packageName/-/vite-plus-cli-$packageSuffix-$ViteVersion.tgz"
    Write-Info "Downloading native binary..."
    Download-AndExtract -Url $binaryUrl -DestDir $BinDir -Filter $binaryName

    # Create wrapper batch file
    $wrapperContent = @"
@echo off
set VITE_GLOBAL_CLI_JS_SCRIPTS_DIR=$DistDir
"$BinDir\$binaryName" %*
"@
    $wrapperPath = Join-Path $BinDir "vite.cmd"
    Set-Content -Path $wrapperPath -Value $wrapperContent -Encoding ASCII

    # Download and extract JS bundle
    $mainUrl = "$NpmRegistry/vite-plus-cli/-/vite-plus-cli-$ViteVersion.tgz"
    Write-Info "Downloading JS scripts..."
    Download-AndExtract -Url $mainUrl -DestDir $DistDir -Filter "dist\*"

    # Move files from dist subdirectory if needed
    $distSubdir = Join-Path $DistDir "dist"
    if (Test-Path $distSubdir) {
        Get-ChildItem -Path $distSubdir | Move-Item -Destination $DistDir -Force
        Remove-Item -Path $distSubdir -Force -ErrorAction SilentlyContinue
    }

    Write-Success "Vite+ CLI installed to $InstallDir"

    # Update PATH
    Write-Host ""
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($userPath -notlike "*$BinDir*") {
        $newPath = "$BinDir;$userPath"
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        $env:Path = "$BinDir;$env:Path"
        Write-Success "PATH has been updated"
        Write-Host ""
        Write-Host "  Restart your terminal to use vite, or run:"
        Write-Host ""
        Write-Host "    `$env:Path = `"$BinDir;`$env:Path`""
    } else {
        Write-Info "PATH already contains $BinDir"
    }

    Write-Host ""
    Write-Host "  Then run:"
    Write-Host ""
    Write-Host "    vite --version"
    Write-Host ""
}

Main
