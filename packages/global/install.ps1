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
#   VITE_PLUS_LOCAL_BINARY - Path to locally built binary (for development/testing)
#   VITE_PLUS_LOCAL_PACKAGE - Path to local vite-plus-cli package dir (for development/testing)

$ErrorActionPreference = "Stop"

$ViteVersion = if ($env:VITE_PLUS_VERSION) { $env:VITE_PLUS_VERSION } else { "latest" }
$InstallDir = if ($env:VITE_PLUS_HOME) { $env:VITE_PLUS_HOME } else { "$env:USERPROFILE\.vite-plus" }
# npm registry URL (strip trailing slash if present)
$NpmRegistry = if ($env:NPM_CONFIG_REGISTRY) { $env:NPM_CONFIG_REGISTRY.TrimEnd('/') } else { "https://registry.npmjs.org" }
# Local paths for development/testing
$LocalBinary = $env:VITE_PLUS_LOCAL_BINARY
$LocalPackage = $env:VITE_PLUS_LOCAL_PACKAGE

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
        $metadataUrl = "$NpmRegistry/vite-plus-cli/$versionPath"
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

function Get-PackageSuffix {
    param([string]$Platform)

    $metadata = Get-PackageMetadata
    $optionalDeps = $metadata.optionalDependencies

    if ($null -eq $optionalDeps) {
        Write-Error-Exit "No optionalDependencies found in package metadata"
    }

    # Find matching package for platform
    $prefix = "@voidzero-dev/vite-plus-cli-"
    $matchingPackage = $null

    foreach ($dep in $optionalDeps.PSObject.Properties.Name) {
        if ($dep.StartsWith("$prefix$Platform")) {
            $matchingPackage = $dep
            break
        }
    }

    if ($null -eq $matchingPackage) {
        # List available platforms for helpful error message
        $availablePlatforms = $optionalDeps.PSObject.Properties.Name |
            ForEach-Object { $_.Replace($prefix, "") } |
            Join-String -Separator ", "
        Write-Error-Exit "Unsupported platform: $Platform. Available platforms: $availablePlatforms"
    }

    # Extract suffix by removing the package prefix
    return $matchingPackage.Replace($prefix, "")
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
    Write-Host "Setting up VITE+(⚡︎)..."
    Write-Host ""

    # Suppress progress bars for cleaner output
    $ProgressPreference = 'SilentlyContinue'

    $arch = Get-Architecture
    $platform = "win32-$arch"

    # Local development mode: skip npm entirely
    if ($LocalBinary -and $LocalPackage) {
        # Validate local paths
        if (-not (Test-Path $LocalBinary)) {
            Write-Error-Exit "Local binary not found: $LocalBinary"
        }
        if (-not (Test-Path $LocalPackage)) {
            Write-Error-Exit "Local package directory not found: $LocalPackage"
        }
        # Use version as-is (default to "local-dev")
        if ($ViteVersion -eq "latest") {
            $ViteVersion = "local-dev"
        }
    } else {
        # Fetch package metadata and resolve version from npm
        $ViteVersion = Get-VersionFromMetadata
    }

    # Set up version-specific directories
    $VersionDir = "$InstallDir\$ViteVersion"
    $BinDir = "$VersionDir\bin"
    $DistDir = "$VersionDir\dist"
    $CurrentLink = "$InstallDir\current"

    $binaryName = "vp.exe"

    # Create directories
    New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
    New-Item -ItemType Directory -Force -Path $DistDir | Out-Null

    # Download and extract native binary and .node files from platform package
    if ($LocalBinary) {
        # Use local binary for development/testing
        Write-Info "Using local binary: $LocalBinary"
        Copy-Item -Path $LocalBinary -Destination "$BinDir\$binaryName" -Force
        # Note: .node files won't be available when using local binary
    } else {
        # Get package suffix from optionalDependencies (dynamic lookup)
        $packageSuffix = Get-PackageSuffix -Platform $platform
        $packageName = "@voidzero-dev/vite-plus-cli-$packageSuffix"
        $platformUrl = "$NpmRegistry/$packageName/-/vite-plus-cli-$packageSuffix-$ViteVersion.tgz"

        $platformTempFile = New-TemporaryFile
        try {
            Invoke-WebRequest -Uri $platformUrl -OutFile $platformTempFile

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
    }

    # Copy JS bundle and assets from local package or download from npm
    $itemsToCopy = @("dist", "templates", "rules", "AGENTS.md", "package.json")
    if ($LocalPackage) {
        # Use local package for development/testing
        Write-Info "Using local package: $LocalPackage"
        foreach ($item in $itemsToCopy) {
            $itemSource = Join-Path $LocalPackage $item
            if (Test-Path $itemSource) {
                Copy-Item -Path $itemSource -Destination $VersionDir -Recurse -Force
            }
        }
    } else {
        # Download and extract JS bundle from npm
        $mainUrl = "$NpmRegistry/vite-plus-cli/-/vite-plus-cli-$ViteVersion.tgz"

        $mainTempFile = New-TemporaryFile
        try {
            Invoke-WebRequest -Uri $mainUrl -OutFile $mainTempFile

            # Create temp extraction directory
            $mainTempExtract = Join-Path $env:TEMP "vite-main-$(Get-Random)"
            New-Item -ItemType Directory -Force -Path $mainTempExtract | Out-Null

            # Extract the package
            tar -xzf $mainTempFile -C $mainTempExtract

            # Copy directories and files to VersionDir
            foreach ($item in $itemsToCopy) {
                $itemSource = Join-Path $mainTempExtract "package" $item
                if (Test-Path $itemSource) {
                    Copy-Item -Path $itemSource -Destination $VersionDir -Recurse -Force
                }
            }

            Remove-Item -Recurse -Force $mainTempExtract
        } finally {
            Remove-Item $mainTempFile -ErrorAction SilentlyContinue
        }
    }

    # Skip dependency installation for local package (deps already bundled or available)
    if (-not $LocalPackage) {
        # Remove devDependencies and optionalDependencies from package.json
        # (temporary solution until deps are fully bundled)
        $pkgFile = Join-Path $VersionDir "package.json"
        $pkg = Get-Content $pkgFile -Raw | ConvertFrom-Json
        $pkg.PSObject.Properties.Remove("devDependencies")
        $pkg.PSObject.Properties.Remove("optionalDependencies")
        $pkg | ConvertTo-Json -Depth 10 | Set-Content $pkgFile

        # Install production dependencies
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
    New-Item -ItemType Directory -Force -Path "$InstallDir\bin" | Out-Null
    $wrapperContent = @"
@echo off
"%~dp0..\current\bin\vp.exe" %*
exit /b %ERRORLEVEL%
"@
    Set-Content -Path "$InstallDir\bin\vp.cmd" -Value $wrapperContent -NoNewline

    # Cleanup old versions
    Cleanup-OldVersions -InstallDir $InstallDir

    # Configure user PATH (always attempted)
    $pathResult = Configure-UserPath

    # Setup Node.js version manager (shims) - separate component
    $nodeManagerResult = Setup-NodeManager -BinDir $BinDir

    # Use ~ shorthand if install dir is under USERPROFILE, otherwise show full path
    $displayDir = $InstallDir -replace [regex]::Escape($env:USERPROFILE), '~'

    # Print success message
    Write-Host ""
    Write-Host "✔ " -ForegroundColor Green -NoNewline
    Write-Host "VITE+(⚡︎) successfully installed!"
    Write-Host ""
    Write-Host "  Version: $ViteVersion"
    Write-Host ""
    Write-Host "  Location: $displayDir\bin"

    # Show Node.js manager status
    if ($nodeManagerResult -eq "true" -or $nodeManagerResult -eq "already") {
        Write-Host ""
        Write-Host "  Node.js manager: on"
    }

    Write-Host ""
    Write-Host "  Next: Run ``vp help`` to get started"

    # Show note if PATH was updated
    if ($pathResult -eq "true") {
        Write-Host ""
        Write-Host "  Note: Restart your terminal and IDE for changes to take effect."
    }

    Write-Host ""
}

Main
