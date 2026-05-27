[CmdletBinding()]
param(
    [switch]$SkipBuild,
    [string]$OutputDirectory = "",
    [string]$ClientEnvironmentFile = ""
)

$ErrorActionPreference = "Stop"
$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$DefaultEnvironment = Join-Path $PSScriptRoot ".env"
$TargetDirectory = if ($OutputDirectory) {
    [System.IO.Path]::GetFullPath($OutputDirectory)
} else {
    Join-Path $RepoRoot "dist\windows\ActivateurRMS"
}

if (-not $SkipBuild) {
    $EmbeddedEnvironment = if ($ClientEnvironmentFile) {
        [System.IO.Path]::GetFullPath($ClientEnvironmentFile)
    } else {
        $DefaultEnvironment
    }
    if (-not (Test-Path -LiteralPath $EmbeddedEnvironment -PathType Leaf)) {
        throw "Client build configuration is missing: $EmbeddedEnvironment. Copy client.env.example to .env and configure it before packaging."
    }

    Push-Location $RepoRoot
    $PreviousBuildEnvironment = $env:KEYGEN_BUILD_ENV_FILE
    try {
        $env:KEYGEN_BUILD_ENV_FILE = $EmbeddedEnvironment
        cargo build --release --workspace
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build failed with exit code $LASTEXITCODE"
        }
    } finally {
        if ($null -eq $PreviousBuildEnvironment) {
            Remove-Item Env:KEYGEN_BUILD_ENV_FILE -ErrorAction SilentlyContinue
        } else {
            $env:KEYGEN_BUILD_ENV_FILE = $PreviousBuildEnvironment
        }
        Pop-Location
    }
}

$DesktopExecutable = Join-Path $RepoRoot "target\release\ActivateurRMS.exe"
$ServiceExecutable = Join-Path $RepoRoot "target\release\KeyGenService.exe"
$NssmExecutable = Join-Path $PSScriptRoot "vendor\nssm\nssm.exe"

foreach ($path in @($DesktopExecutable, $ServiceExecutable, $NssmExecutable)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Required package file is missing: $path"
    }
}

$ServiceDirectory = Join-Path $TargetDirectory "KeyGenService"
$NssmDirectory = Join-Path $TargetDirectory "nssm"
New-Item -ItemType Directory -Force -Path $TargetDirectory, $ServiceDirectory, $NssmDirectory | Out-Null

Copy-Item -LiteralPath $DesktopExecutable -Destination (Join-Path $TargetDirectory "ActivateurRMS.exe") -Force
Copy-Item -LiteralPath $ServiceExecutable -Destination (Join-Path $ServiceDirectory "KeyGenService.exe") -Force
Copy-Item -LiteralPath $NssmExecutable -Destination (Join-Path $NssmDirectory "nssm.exe") -Force
foreach ($runtimeConfiguration in @(".env", ".env.example")) {
    $packagedRuntimeConfiguration = Join-Path $TargetDirectory $runtimeConfiguration
    if (Test-Path -LiteralPath $packagedRuntimeConfiguration -PathType Leaf) {
        Remove-Item -LiteralPath $packagedRuntimeConfiguration -Force
    }
}

$PackagedFiles = [ordered]@{
    "ActivateurRMS.exe" = (Join-Path $TargetDirectory "ActivateurRMS.exe")
    "KeyGenService\KeyGenService.exe" = (Join-Path $ServiceDirectory "KeyGenService.exe")
    "nssm\nssm.exe" = (Join-Path $NssmDirectory "nssm.exe")
}
$Checksums = foreach ($relativePath in $PackagedFiles.Keys) {
    $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $PackagedFiles[$relativePath]).Hash
    "$hash  $relativePath"
}
Set-Content -LiteralPath (Join-Path $TargetDirectory "SHA256SUMS.txt") -Value $Checksums -Encoding ascii

Write-Host "Package created: $TargetDirectory"
