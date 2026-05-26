[CmdletBinding()]
param(
    [switch]$SkipBuild,
    [string]$OutputDirectory = ""
)

$ErrorActionPreference = "Stop"
$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$TargetDirectory = if ($OutputDirectory) {
    [System.IO.Path]::GetFullPath($OutputDirectory)
} else {
    Join-Path $RepoRoot "dist\windows\ActivateurRMS"
}

if (-not $SkipBuild) {
    Push-Location $RepoRoot
    try {
        cargo build --release --workspace
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build failed with exit code $LASTEXITCODE"
        }
    } finally {
        Pop-Location
    }
}

$DesktopExecutable = Join-Path $RepoRoot "target\release\ActivateurRMS.exe"
$ServiceExecutable = Join-Path $RepoRoot "target\release\KeyGenService.exe"
$NssmExecutable = Join-Path $PSScriptRoot "vendor\nssm\nssm.exe"
$EnvironmentTemplate = Join-Path $PSScriptRoot "client.env.example"

foreach ($path in @($DesktopExecutable, $ServiceExecutable, $NssmExecutable, $EnvironmentTemplate)) {
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
Copy-Item -LiteralPath $EnvironmentTemplate -Destination (Join-Path $TargetDirectory ".env.example") -Force

$PackagedFiles = [ordered]@{
    "ActivateurRMS.exe" = (Join-Path $TargetDirectory "ActivateurRMS.exe")
    "KeyGenService\KeyGenService.exe" = (Join-Path $ServiceDirectory "KeyGenService.exe")
    "nssm\nssm.exe" = (Join-Path $NssmDirectory "nssm.exe")
    ".env.example" = (Join-Path $TargetDirectory ".env.example")
}
$Checksums = foreach ($relativePath in $PackagedFiles.Keys) {
    $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $PackagedFiles[$relativePath]).Hash
    "$hash  $relativePath"
}
Set-Content -LiteralPath (Join-Path $TargetDirectory "SHA256SUMS.txt") -Value $Checksums -Encoding ascii

Write-Host "Package created: $TargetDirectory"
