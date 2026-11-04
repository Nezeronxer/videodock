$ErrorActionPreference = 'Stop'
$project = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
if (-not $env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR = Join-Path $env:LOCALAPPDATA 'VideoDock\build' }
if (-not $env:CARGO_BUILD_JOBS) { $env:CARGO_BUILD_JOBS = '2' }
Get-Command cargo, git, ffmpeg, ffprobe -ErrorAction Stop | Out-Null
Push-Location $project
try {
    & cargo build --locked --release -p videodock-desktop
    if ($LASTEXITCODE -ne 0) { throw 'Rust build failed' }
    $destination = Join-Path $env:LOCALAPPDATA 'VideoDock\app'
    New-Item -ItemType Directory -Force $destination | Out-Null
    Copy-Item (Join-Path $env:CARGO_TARGET_DIR 'release\videodock-desktop.exe') (Join-Path $destination 'VideoDock.exe') -Force
    Copy-Item (Get-Command ffmpeg -ErrorAction Stop).Source (Join-Path $destination 'ffmpeg.exe') -Force
    Copy-Item (Get-Command ffprobe -ErrorAction Stop).Source (Join-Path $destination 'ffprobe.exe') -Force
    & windeployqt --release --qmldir (Join-Path $project 'crates\desktop\qml') (Join-Path $destination 'VideoDock.exe')
    if ($LASTEXITCODE -ne 0) { throw 'Qt deployment failed' }
    $shell = New-Object -ComObject WScript.Shell
    $shortcut = $shell.CreateShortcut((Join-Path ([Environment]::GetFolderPath('Programs')) 'VideoDock.lnk'))
    $shortcut.TargetPath = Join-Path $destination 'VideoDock.exe'
    $shortcut.WorkingDirectory = $destination
    $shortcut.Save()
    Write-Output "VideoDock: $destination"
} finally { Pop-Location }
