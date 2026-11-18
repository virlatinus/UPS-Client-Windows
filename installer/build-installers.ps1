<#
.SYNOPSIS
    Builds the UPS Client Windows release binary and generates Windows installers (.exe via NSIS and .msi via WiX Toolset).

.DESCRIPTION
    Can be run locally or in GitHub Actions CI/Release workflows.
    Outputs artifacts to the `dist/` directory:
      - ups-client-windows-v<Version>-x86_64.exe        (Standalone portable binary)
      - UPS-Client-Windows-v<Version>-x86_64-setup.exe  (NSIS .exe setup installer)
      - UPS-Client-Windows-v<Version>-x86_64.msi        (WiX .msi Windows Installer package)
      - SHA256SUMS.txt                                  (SHA-256 checksums for all artifacts)

.EXAMPLE
    .\installer\build-installers.ps1
    .\installer\build-installers.ps1 -Version "v1.0.0"
#>

[CmdletBinding()]
param(
    [string]$Version = "",
    [switch]$SkipCargoBuild,
    [switch]$SkipExeInstaller,
    [switch]$SkipMsiInstaller
)

$ErrorActionPreference = "Stop"

# Ensure ~/.cargo/bin is in PATH
if (Test-Path "$env:USERPROFILE\.cargo\bin") {
    $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
}

$RepoRoot = Split-Path -Parent $PSScriptRoot
Push-Location $RepoRoot

try {
    # 1. Resolve version from parameter or Cargo.toml (strip leading 'v' for numeric version)
    if ([string]::IsNullOrWhiteSpace($Version)) {
        $CargoToml = Get-Content -Path (Join-Path $RepoRoot "Cargo.toml") -Raw
        if ($CargoToml -match '(?m)^version\s*=\s*"([^"]+)"') {
            $NumericVersion = $Matches[1]
        } else {
            $NumericVersion = "1.0.0"
        }
    } else {
        $NumericVersion = $Version.TrimStart('v', 'V')
    }

    $TagVersion = "v$NumericVersion"
    Write-Host "==> Building UPS Client Windows $TagVersion (numeric: $NumericVersion)" -ForegroundColor Cyan

    # 2. Build Release Binary via Cargo
    $ReleaseExe = Join-Path $RepoRoot "target\release\ups-client-windows.exe"
    if (-not $SkipCargoBuild) {
        Write-Host "==> Running cargo build --release..." -ForegroundColor Cyan
        cargo build --release
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build --release failed with exit code $LASTEXITCODE"
        }
    }

    if (-not (Test-Path $ReleaseExe)) {
        throw "Release binary not found at $ReleaseExe"
    }

    # 3. Prepare dist/ output folder and copy standalone binary
    $DistDir = Join-Path $RepoRoot "dist"
    if (Test-Path $DistDir) {
        Remove-Item -Path "$DistDir\*" -Force -ErrorAction SilentlyContinue
    } else {
        New-Item -ItemType Directory -Path $DistDir | Out-Null
    }

    $StandaloneArtifact = Join-Path $DistDir "ups-client-windows-$TagVersion-x86_64.exe"
    Copy-Item -Path $ReleaseExe -Destination $StandaloneArtifact -Force
    Write-Host "==> Created standalone binary: $StandaloneArtifact" -ForegroundColor Green

    # 4. Build .exe Installer using NSIS (makensis.exe)
    if (-not $SkipExeInstaller) {
        $MakeNsis = $null
        $NsisCmd = Get-Command makensis -ErrorAction SilentlyContinue
        if ($NsisCmd) {
            $MakeNsis = $NsisCmd.Source
        } else {
            $CandidatePaths = @(
                "${env:ProgramFiles(x86)}\NSIS\makensis.exe",
                "${env:ProgramFiles}\NSIS\makensis.exe"
            )
            foreach ($Candidate in $CandidatePaths) {
                if (Test-Path $Candidate) {
                    $MakeNsis = $Candidate
                    break
                }
            }
        }

        if (-not $MakeNsis) {
            throw "NSIS (makensis.exe) was not found. Install it via: winget install NSIS.NSIS"
        }

        $SetupExeArtifact = Join-Path $DistDir "UPS-Client-Windows-$TagVersion-x86_64-setup.exe"
        $NsiScript = Join-Path $RepoRoot "installer\ups-client-windows.nsi"

        Write-Host "==> Building NSIS .exe installer using $MakeNsis..." -ForegroundColor Cyan
        & $MakeNsis "/DVERSION=$NumericVersion" "/DOUTFILE=$SetupExeArtifact" $NsiScript
        if ($LASTEXITCODE -ne 0) {
            throw "NSIS installer build failed with exit code $LASTEXITCODE"
        }
        Write-Host "==> Created .exe installer: $SetupExeArtifact" -ForegroundColor Green
    }

    # 5. Build .msi Installer using WiX Toolset (candle.exe + light.exe or cargo-wix)
    if (-not $SkipMsiInstaller) {
        $CandleExe = $null
        $LightExe = $null

        $CandleCmd = Get-Command candle -ErrorAction SilentlyContinue
        $LightCmd = Get-Command light -ErrorAction SilentlyContinue
        if ($CandleCmd -and $LightCmd) {
            $CandleExe = $CandleCmd.Source
            $LightExe = $LightCmd.Source
        } else {
            $WixDirs = @()
            if ($env:WIX) {
                $WixDirs += (Join-Path $env:WIX "bin")
            }
            $ProgFilesX86 = ${env:ProgramFiles(x86)}
            if ($ProgFilesX86 -and (Test-Path $ProgFilesX86)) {
                $FoundWixDirs = Get-ChildItem -Path $ProgFilesX86 -Filter "WiX Toolset*" -Directory -ErrorAction SilentlyContinue
                foreach ($Dir in $FoundWixDirs) {
                    $WixDirs += (Join-Path $Dir.FullName "bin")
                }
            }
            foreach ($BinDir in $WixDirs) {
                $CandidateCandle = Join-Path $BinDir "candle.exe"
                $CandidateLight = Join-Path $BinDir "light.exe"
                if ((Test-Path $CandidateCandle) -and (Test-Path $CandidateLight)) {
                    $CandleExe = $CandidateCandle
                    $LightExe = $CandidateLight
                    break
                }
            }
        }

        $MsiArtifact = Join-Path $DistDir "UPS-Client-Windows-$TagVersion-x86_64.msi"

        if ($CandleExe -and $LightExe) {
            Write-Host "==> Building WiX .msi installer using $CandleExe & $LightExe..." -ForegroundColor Cyan
            $WixObj = Join-Path $DistDir "main.wixobj"
            & $CandleExe "-dVersion=$NumericVersion" "-dTargetBin=$ReleaseExe" -arch x64 -out $WixObj "wix\main.wxs"
            if ($LASTEXITCODE -ne 0) {
                throw "WiX candle.exe failed with exit code $LASTEXITCODE"
            }

            & $LightExe -ext WixUIExtension -spdb -out $MsiArtifact $WixObj
            if ($LASTEXITCODE -ne 0) {
                throw "WiX light.exe failed with exit code $LASTEXITCODE"
            }

            Remove-Item -Path $WixObj -Force -ErrorAction SilentlyContinue
            Write-Host "==> Created .msi installer: $MsiArtifact" -ForegroundColor Green
        } elseif (Get-Command cargo-wix -ErrorAction SilentlyContinue) {
            Write-Host "==> Building WiX .msi installer using cargo wix..." -ForegroundColor Cyan
            cargo wix --nocapture --version $NumericVersion --output $MsiArtifact
            if ($LASTEXITCODE -ne 0) {
                throw "cargo wix failed with exit code $LASTEXITCODE"
            }
            Write-Host "==> Created .msi installer: $MsiArtifact" -ForegroundColor Green
        } else {
            throw "WiX Toolset (candle.exe/light.exe or cargo-wix) was not found. Install WiX v3 (winget install WiXToolset.WiXToolset) or cargo-wix (cargo install cargo-wix)."
        }
    }

    # 6. Generate SHA256SUMS.txt
    $ChecksumFile = Join-Path $DistDir "SHA256SUMS.txt"
    $Hashes = @()
    Get-ChildItem -Path $DistDir -File | Where-Object { $_.Name -ne "SHA256SUMS.txt" } | ForEach-Object {
        $Hash = (Get-FileHash -Path $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        $Hashes += "$Hash  $($_.Name)"
    }
    $Hashes | Set-Content -Path $ChecksumFile -Encoding UTF8
    Write-Host "==> Generated SHA256SUMS.txt:" -ForegroundColor Cyan
    Get-Content -Path $ChecksumFile | ForEach-Object { Write-Host "    $_" }

    Write-Host "`n==> All release artifacts built successfully in: $DistDir" -ForegroundColor Green
} finally {
    Pop-Location
}
