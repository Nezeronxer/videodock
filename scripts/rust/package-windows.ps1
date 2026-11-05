param(
    [Parameter(Mandatory = $true)][string]$SourceDirectory,
    [Parameter(Mandatory = $true)][string]$OutputDirectory
)
$ErrorActionPreference = 'Stop'
$project = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$source = (Resolve-Path $SourceDirectory).Path
foreach ($file in @('VideoDock.exe', 'QtWebEngineProcess.exe', 'ffmpeg.exe', 'ffprobe.exe', 'msvcp140.dll', 'vcruntime140.dll', 'platforms/qwindows.dll', 'licenses/GPL-3.0.txt', 'licenses/rust/manifest.json', 'THIRD-PARTY-NOTICES.md', 'videodock.ico')) {
    if (-not (Test-Path (Join-Path $source $file) -PathType Leaf)) { throw "Incomplete application package: $file" }
}
$versionLine = Select-String -Path (Join-Path $project 'Cargo.toml') -Pattern '^version\s*=\s*"([0-9]+\.[0-9]+\.[0-9]+)"' | Select-Object -First 1
if (-not $versionLine) { throw 'Cannot determine the workspace release version' }
$version = $versionLine.Matches[0].Groups[1].Value
New-Item -ItemType Directory -Force $OutputDirectory | Out-Null
$output = (Resolve-Path $OutputDirectory).Path
$compiler = (Get-Command ISCC.exe -ErrorAction Stop).Source
& $compiler "/DAppSourceDir=$source" "/DPackageOutputDir=$output" "/DAppVersion=$version" (Join-Path $PSScriptRoot 'windows-installer.iss')
if ($LASTEXITCODE -ne 0) { throw 'Windows installer compilation failed' }
$installer = Join-Path $output "VideoDock-$version-windows-x64-Setup.exe"
if (-not (Test-Path $installer -PathType Leaf)) { throw 'Windows installer was not created' }
Write-Output $installer
