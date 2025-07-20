# Quetty Universal Installation Script for Windows
# Supports Windows x64 and ARM64
# Usage: Invoke-RestMethod -Uri "https://raw.githubusercontent.com/dawidpereira/quetty/main/install.ps1" | Invoke-Expression

param(
    [string]$Version = "",
    [string]$Channel = "stable",
    [string]$InstallDir = "",
    [switch]$System = $false,
    [switch]$DryRun = $false,
    [switch]$Force = $false,
    [switch]$Uninstall = $false,
    [switch]$Help = $false
)

# Configuration
$RepoOwner = "dawidpereira"
$RepoName = "quetty"
$GitHubApiUrl = "https://api.github.com/repos/$RepoOwner/$RepoName"

# Color output functions
function Write-Info {
    param([string]$Message)
    Write-Host "INFO: $Message" -ForegroundColor Blue
}

function Write-Success {
    param([string]$Message)
    Write-Host "SUCCESS: $Message" -ForegroundColor Green
}

function Write-Warning {
    param([string]$Message)
    Write-Host "WARNING: $Message" -ForegroundColor Yellow
}

function Write-Error {
    param([string]$Message)
    Write-Host "ERROR: $Message" -ForegroundColor Red
}

# Help message
function Show-Help {
    @"
Quetty Universal Installation Script for Windows

USAGE:
    # Download and run directly
    Invoke-RestMethod -Uri "https://raw.githubusercontent.com/dawidpereira/quetty/main/install.ps1" | Invoke-Expression

    # Download and run with parameters
    Invoke-RestMethod -Uri "https://raw.githubusercontent.com/dawidpereira/quetty/main/install.ps1" -OutFile install.ps1
    .\install.ps1 [OPTIONS]

OPTIONS:
    -Version VERSION       Install specific version (e.g., "v0.1.0-alpha.1")
    -Channel CHANNEL       Install from channel: "stable" (default), "nightly"
    -InstallDir DIR        Custom installation directory
    -System               Install system-wide to Program Files (requires elevation)
    -DryRun               Show what would be installed without executing
    -Force                Force reinstall even if already installed
    -Uninstall            Remove installed Quetty binary
    -Help                 Show this help message

EXAMPLES:
    # Install latest stable release
    Invoke-RestMethod -Uri "https://raw.githubusercontent.com/dawidpereira/quetty/main/install.ps1" | Invoke-Expression

    # Install specific version
    .\install.ps1 -Version "v0.1.0-alpha.1"

    # Install to custom directory
    .\install.ps1 -InstallDir "C:\Tools\bin"

    # Install system-wide (requires admin)
    .\install.ps1 -System

    # Install nightly build
    .\install.ps1 -Channel "nightly"

    # Dry run to see what would happen
    .\install.ps1 -DryRun

"@
}

# Detect platform and architecture
function Get-Platform {
    $arch = $env:PROCESSOR_ARCHITECTURE

    switch ($arch) {
        "AMD64" {
            $Platform = "windows-x64"
            $ArtifactName = "quetty-windows-x64.exe"
        }
        "ARM64" {
            $Platform = "windows-arm64"
            $ArtifactName = "quetty-windows-arm64.exe"
        }
        default {
            Write-Error "Unsupported architecture: $arch"
            Write-Info "Supported architectures: AMD64 (x64), ARM64"
            exit 1
        }
    }

    Write-Info "Detected platform: $Platform"

    return @{
        Platform = $Platform
        ArtifactName = $ArtifactName
        ArchiveExt = "zip"
    }
}

# Check if running with administrator privileges
function Test-Administrator {
    $currentUser = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($currentUser)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

# Get latest release version
function Get-LatestVersion {
    param([string]$Channel)

    if ($Channel -eq "nightly") {
        return "nightly-latest"
    } else {
        $latestUrl = "$GitHubApiUrl/releases/latest"

        try {
            $response = Invoke-RestMethod -Uri $latestUrl -UseBasicParsing
            return $response.tag_name
        } catch {
            Write-Error "Could not determine latest version: $_"
            exit 1
        }
    }
}

# Get download URL for version
function Get-DownloadUrl {
    param(
        [string]$Version,
        [string]$Filename
    )

    if ($Version -eq "nightly-latest") {
        return "https://github.com/$RepoOwner/$RepoName/releases/download/$Version/$Filename"
    } else {
        $releaseUrl = "$GitHubApiUrl/releases/tags/$Version"

        try {
            $response = Invoke-RestMethod -Uri $releaseUrl -UseBasicParsing
            $asset = $response.assets | Where-Object { $_.name -eq $Filename }

            if (-not $asset) {
                Write-Error "Could not find download URL for $Filename in $Version"
                Write-Info "Available assets:"
                $response.assets | ForEach-Object { Write-Info "  $($_.name)" }
                exit 1
            }

            return $asset.browser_download_url
        } catch {
            Write-Error "Could not get release information for $Version: $_"
            exit 1
        }
    }
}

# Get checksum URL
function Get-ChecksumUrl {
    param(
        [string]$Version,
        [string]$ChecksumFilename
    )

    if ($Version -eq "nightly-latest") {
        return "https://github.com/$RepoOwner/$RepoName/releases/download/$Version/$ChecksumFilename"
    } else {
        $releaseUrl = "$GitHubApiUrl/releases/tags/$Version"

        try {
            $response = Invoke-RestMethod -Uri $releaseUrl -UseBasicParsing
            $asset = $response.assets | Where-Object { $_.name -eq $ChecksumFilename }

            if ($asset) {
                return $asset.browser_download_url
            } else {
                return $null
            }
        } catch {
            return $null
        }
    }
}

# Set installation directory
function Set-InstallDir {
    param([string]$CustomDir, [bool]$SystemInstall)

    if ($CustomDir) {
        return $CustomDir
    } elseif ($SystemInstall) {
        return "${env:ProgramFiles}\Quetty"
    } else {
        return "${env:LOCALAPPDATA}\Programs\Quetty"
    }
}

# Check if Quetty is already installed
function Test-ExistingInstallation {
    param([string]$InstallDir, [bool]$Force)

    $quettyPath = Join-Path $InstallDir "quetty.exe"

    if ((Test-Path $quettyPath) -and (-not $Force)) {
        Write-Warning "Quetty is already installed at $quettyPath"

        try {
            $currentVersion = & $quettyPath --version 2>$null | Select-Object -First 1
            Write-Info "Current version: $currentVersion"
        } catch {
            Write-Info "Current version: unknown"
        }

        Write-Info "Use -Force to reinstall or -Uninstall to remove"
        exit 1
    }
}

# Create installation directory
function New-InstallDir {
    param([string]$InstallDir, [bool]$DryRun)

    if (-not (Test-Path $InstallDir)) {
        Write-Info "Creating installation directory: $InstallDir"

        if (-not $DryRun) {
            try {
                New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
            } catch {
                Write-Error "Failed to create installation directory: $InstallDir"
                Write-Info "You may need to run as administrator or choose a different directory"
                exit 1
            }
        }
    }
}

# Download and verify file
function Get-FileWithVerification {
    param(
        [string]$Url,
        [string]$OutputPath,
        [string]$ChecksumUrl,
        [bool]$DryRun
    )

    Write-Info "Downloading: $(Split-Path $OutputPath -Leaf)"

    if (-not $DryRun) {
        try {
            $progressPreference = 'SilentlyContinue'
            Invoke-WebRequest -Uri $Url -OutFile $OutputPath -UseBasicParsing
            $progressPreference = 'Continue'
        } catch {
            Write-Error "Failed to download $Url : $_"
            exit 1
        }
    }

    # Verify checksum if available
    if ($ChecksumUrl -and (-not $DryRun)) {
        Write-Info "Verifying checksum..."

        try {
            $progressPreference = 'SilentlyContinue'
            $checksumContent = Invoke-WebRequest -Uri $ChecksumUrl -UseBasicParsing | Select-Object -ExpandProperty Content
            $progressPreference = 'Continue'

            if ($checksumContent) {
                $expectedChecksum = ($checksumContent -split '\s+')[0]
                $actualChecksum = (Get-FileHash -Path $OutputPath -Algorithm SHA256).Hash.ToLower()

                if ($expectedChecksum -eq $actualChecksum) {
                    Write-Success "Checksum verification passed"
                } else {
                    Write-Error "Checksum verification failed"
                    Write-Error "Expected: $expectedChecksum"
                    Write-Error "Actual:   $actualChecksum"
                    exit 1
                }
            } else {
                Write-Warning "Could not retrieve checksum for verification"
            }
        } catch {
            Write-Warning "Could not verify checksum: $_"
        }
    } elseif (-not $ChecksumUrl) {
        Write-Warning "No checksum available for verification"
    }
}

# Extract and install
function Install-FromArchive {
    param(
        [string]$ArchivePath,
        [string]$InstallDir,
        [string]$ExpectedBinaryName,
        [bool]$DryRun
    )

    Write-Info "Extracting archive..."

    if (-not $DryRun) {
        $tempExtractDir = Join-Path $env:TEMP "quetty_extract_$(Get-Random)"

        try {
            # Extract ZIP file
            Expand-Archive -Path $ArchivePath -DestinationPath $tempExtractDir -Force

            # Find the binary
            $binaryPath = Join-Path $tempExtractDir $ExpectedBinaryName

            if (-not (Test-Path $binaryPath)) {
                Write-Error "Binary not found in archive: $ExpectedBinaryName"
                Write-Info "Archive contents:"
                Get-ChildItem $tempExtractDir | ForEach-Object { Write-Info "  $($_.Name)" }
                exit 1
            }

            # Install binary
            $finalPath = Join-Path $InstallDir "quetty.exe"
            Write-Info "Installing to: $finalPath"

            Copy-Item $binaryPath $finalPath -Force

        } catch {
            Write-Error "Failed to extract and install: $_"
            exit 1
        } finally {
            # Clean up temp directory
            if (Test-Path $tempExtractDir) {
                Remove-Item $tempExtractDir -Recurse -Force -ErrorAction SilentlyContinue
            }
        }
    }
}

# Check and update PATH
function Update-Path {
    param([string]$InstallDir)

    $currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")

    if ($currentPath -notlike "*$InstallDir*") {
        Write-Warning "Installation directory is not in PATH: $InstallDir"
        Write-Info "To add it to your PATH, run the following command:"
        Write-Info '[Environment]::SetEnvironmentVariable("PATH", $env:PATH + ";' + $InstallDir + '", "User")'
        Write-Info ""
        Write-Info "Or add it manually through System Properties > Environment Variables"
        Write-Info "Then restart your PowerShell session"
    }
}

# Uninstall function
function Uninstall-Quetty {
    param([string]$InstallDir, [bool]$DryRun)

    $quettyPath = Join-Path $InstallDir "quetty.exe"

    if (Test-Path $quettyPath) {
        Write-Info "Removing Quetty from: $quettyPath"

        if (-not $DryRun) {
            try {
                Remove-Item $quettyPath -Force

                # Remove directory if empty
                if ((Get-ChildItem $InstallDir -ErrorAction SilentlyContinue).Count -eq 0) {
                    Remove-Item $InstallDir -Force
                }
            } catch {
                Write-Error "Failed to remove $quettyPath : $_"
                exit 1
            }
        }

        Write-Success "Quetty has been uninstalled"
    } else {
        Write-Warning "Quetty not found at: $quettyPath"
    }

    exit 0
}

# Main function
function Main {
    Write-Info "Quetty Universal Installation Script for Windows"
    Write-Info "=============================================="

    # Show help if requested
    if ($Help) {
        Show-Help
        exit 0
    }

    # Set installation directory
    $InstallDir = Set-InstallDir -CustomDir $InstallDir -SystemInstall $System
    Write-Info "Installation directory: $InstallDir"

    # Check for admin privileges if system install
    if ($System -and (-not (Test-Administrator))) {
        Write-Error "System installation requires administrator privileges"
        Write-Info "Please run PowerShell as Administrator or use user installation"
        exit 1
    }

    # Handle uninstall
    if ($Uninstall) {
        Uninstall-Quetty -InstallDir $InstallDir -DryRun $DryRun
    }

    # Detect platform
    $platformInfo = Get-Platform

    # Check existing installation
    Test-ExistingInstallation -InstallDir $InstallDir -Force $Force

    # Determine version
    if (-not $Version) {
        Write-Info "Determining latest $Channel version..."
        $Version = Get-LatestVersion -Channel $Channel
    }

    Write-Info "Installing Quetty $Version for $($platformInfo.Platform)"

    # Get download URLs
    $filename = "$($platformInfo.ArtifactName.Replace('.exe', ''))-$($Version.TrimStart('v')).$($platformInfo.ArchiveExt)"
    $checksumFilename = "$filename.sha256"

    $downloadUrl = Get-DownloadUrl -Version $Version -Filename $filename
    $checksumUrl = Get-ChecksumUrl -Version $Version -ChecksumFilename $checksumFilename

    Write-Info "Download URL: $downloadUrl"

    if ($DryRun) {
        Write-Info "DRY RUN - The following actions would be performed:"
        Write-Info "1. Create directory: $InstallDir"
        Write-Info "2. Download: $downloadUrl"
        if ($checksumUrl) {
            Write-Info "3. Verify checksum from: $checksumUrl"
        }
        Write-Info "4. Extract and install binary to: $InstallDir\quetty.exe"
        Update-Path -InstallDir $InstallDir
        exit 0
    }

    # Create installation directory
    New-InstallDir -InstallDir $InstallDir -DryRun $DryRun

    # Download and verify
    $archiveFile = Join-Path $env:TEMP $filename
    Get-FileWithVerification -Url $downloadUrl -OutputPath $archiveFile -ChecksumUrl $checksumUrl -DryRun $DryRun

    # Extract and install
    Install-FromArchive -ArchivePath $archiveFile -InstallDir $InstallDir -ExpectedBinaryName $platformInfo.ArtifactName -DryRun $DryRun

    # Clean up downloaded archive
    if (Test-Path $archiveFile) {
        Remove-Item $archiveFile -Force -ErrorAction SilentlyContinue
    }

    # Check installation
    $finalBinary = Join-Path $InstallDir "quetty.exe"
    if (Test-Path $finalBinary) {
        try {
            $installedVersion = & $finalBinary --version 2>$null | Select-Object -First 1
            Write-Success "Quetty installed successfully!"
            Write-Info "Version: $installedVersion"
            Write-Info "Location: $finalBinary"
        } catch {
            Write-Success "Quetty installed successfully!"
            Write-Info "Location: $finalBinary"
        }
    } else {
        Write-Error "Installation verification failed"
        exit 1
    }

    # Check PATH
    Update-Path -InstallDir $InstallDir

    Write-Info ""
    Write-Success "Installation complete! You can now run 'quetty' to get started."
    Write-Info "Run 'quetty --help' for usage information."
    Write-Info "You may need to restart your PowerShell session if you added the directory to PATH."
}

# Run main function
Main
