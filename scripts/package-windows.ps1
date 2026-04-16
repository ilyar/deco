param(
    [Parameter(Mandatory = $true)][string]$Version,
    [Parameter(Mandatory = $true)][string]$Target,
    [Parameter(Mandatory = $true)][string]$BinaryPath,
    [string]$OutputDir = "dist"
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $BinaryPath)) {
    throw "binary not found: $BinaryPath"
}

$packageName = "deco-v$Version-$Target"
$stagingDir = Join-Path $OutputDir $packageName
$archivePath = Join-Path $OutputDir "$packageName.zip"
$checksumPath = "$archivePath.sha256"

if (Test-Path $stagingDir) {
    Remove-Item -Recurse -Force $stagingDir
}

New-Item -ItemType Directory -Force -Path $stagingDir | Out-Null
Copy-Item $BinaryPath (Join-Path $stagingDir "deco.exe")
Copy-Item LICENSE (Join-Path $stagingDir "LICENSE")
Copy-Item README.md (Join-Path $stagingDir "README.md")

if (Test-Path $archivePath) {
    Remove-Item -Force $archivePath
}

Compress-Archive -Path $stagingDir -DestinationPath $archivePath
$hash = (Get-FileHash -Algorithm SHA256 $archivePath).Hash.ToLowerInvariant()
"$hash  $(Split-Path $archivePath -Leaf)" | Set-Content -NoNewline $checksumPath

Write-Output "archive=$archivePath"
Write-Output "checksum=$checksumPath"
