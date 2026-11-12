$ErrorActionPreference = 'Stop'
$project = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
if (-not $env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR = Join-Path $env:LOCALAPPDATA 'VideoDock\build' }
if (-not $env:CARGO_BUILD_JOBS) { $env:CARGO_BUILD_JOBS = '2' }
Get-Command cargo, git, ffmpeg, ffprobe -ErrorAction Stop | Out-Null
$staging = $null
$backup = $null
$published = Join-Path $env:CARGO_TARGET_DIR 'package\VideoDock'
Push-Location $project
try {
    & cargo build --locked --release -p videodock-desktop
    if ($LASTEXITCODE -ne 0) { throw 'Rust build failed' }
    $staging = Join-Path (Split-Path $published) ('VideoDock-stage-' + [guid]::NewGuid().ToString('N'))
    $destination = $staging
    New-Item -ItemType Directory -Force $destination | Out-Null
    Copy-Item (Join-Path $env:CARGO_TARGET_DIR 'release\videodock-desktop.exe') (Join-Path $destination 'VideoDock.exe') -Force
    $image = [IO.File]::ReadAllBytes((Join-Path $destination 'VideoDock.exe'))
    $pe = [BitConverter]::ToInt32($image, 0x3c)
    if ([BitConverter]::ToUInt16($image, $pe + 24 + 68) -ne 2) { throw 'Release executable must use the Windows GUI subsystem' }
    Copy-Item (Get-Command ffmpeg -ErrorAction Stop).Source (Join-Path $destination 'ffmpeg.exe') -Force
    Copy-Item (Get-Command ffprobe -ErrorAction Stop).Source (Join-Path $destination 'ffprobe.exe') -Force
    Copy-Item (Join-Path $project 'assets/videodock.ico') $destination -Force
    & windeployqt --release --no-compiler-runtime --include-plugins qoffscreen --qmldir (Join-Path $project 'crates\desktop\qml') (Join-Path $destination 'VideoDock.exe')
    if ($LASTEXITCODE -ne 0) { throw 'Qt deployment failed' }
    if (-not $env:VCToolsRedistDir) { throw 'Run packaging from the configured Visual Studio build environment' }
    $crt = Get-ChildItem (Join-Path $env:VCToolsRedistDir 'x64') -Directory -Filter 'Microsoft.VC*.CRT' | Sort-Object Name -Descending | Select-Object -First 1
    if (-not $crt) { throw 'Official app-local MSVC runtime absent' }
    Copy-Item (Join-Path $crt.FullName '*.dll') $destination -Force
    foreach ($dll in @('vcruntime140.dll', 'msvcp140.dll', 'platforms\qoffscreen.dll', 'platforms\qwindows.dll', 'QtWebEngineProcess.exe')) {
        if (-not (Test-Path (Join-Path $destination $dll) -PathType Leaf)) { throw "Incomplete portable runtime: $dll" }
    }
    & (Join-Path $PSScriptRoot 'check-windows-binaries.ps1') -Directory $destination
    $licenses = Join-Path $destination 'licenses'
    New-Item -ItemType Directory -Force $licenses | Out-Null
    Copy-Item (Join-Path $project 'LICENSES/*') $licenses -Recurse -Force
    Copy-Item (Join-Path $project 'THIRD-PARTY-NOTICES.md') $destination -Force
    Copy-Item (Join-Path $project 'crates/desktop/attestation/bgutils/LICENSE') (Join-Path $licenses 'BgUtils-MIT.txt') -Force
    Copy-Item (Join-Path $project 'crates/core/src/sabr/LICENSE.googlevideo') (Join-Path $licenses 'googlevideo-MIT.txt') -Force
    Copy-Item (Join-Path $project 'crates/core/src/sabr/LICENSE.sabr-rs') (Join-Path $licenses 'sabr-rs-MIT.txt') -Force
    $ffmpegRoot = Split-Path (Split-Path (Get-Command ffmpeg).Source)
    foreach ($name in @('LICENSE', 'README.txt')) {
        $notice = Join-Path $ffmpegRoot $name
        if (Test-Path $notice -PathType Leaf) {
            Copy-Item $notice (Join-Path $licenses "ffmpeg-upstream-$name") -Force
        }
    }
    # Windows PowerShell 5 treats FFmpeg's normal stderr banner as an ErrorRecord.
    $ErrorActionPreference = 'Continue'
    try {
        & ffmpeg -L 2>&1 | Out-File (Join-Path $licenses 'ffmpeg-license.txt') -Encoding utf8 -ErrorAction Stop
        if ($LASTEXITCODE -ne 0) { throw 'Cannot record FFmpeg license' }
        & ffmpeg -buildconf 2>&1 | Out-File (Join-Path $licenses 'ffmpeg-buildconf.txt') -Encoding utf8 -ErrorAction Stop
        if ($LASTEXITCODE -ne 0) { throw 'Cannot record FFmpeg configuration' }
    } finally { $ErrorActionPreference = 'Stop' }
    $qtBin = Split-Path (Get-Command windeployqt).Source
    & (Join-Path $qtBin 'qmake.exe') -query QT_VERSION | Out-File (Join-Path $licenses 'qt-version.txt') -Encoding utf8
    if ($LASTEXITCODE -ne 0) { throw 'Cannot record Qt version' }
    if (Test-Path $published) {
        $backup = Join-Path (Split-Path $published) ('VideoDock-backup-' + [guid]::NewGuid().ToString('N'))
        [IO.Directory]::Move($published, $backup)
    }
    try {
        [IO.Directory]::Move($staging, $published)
        $staging = $null
    } catch {
        if ($backup -and (Test-Path $backup) -and -not (Test-Path $published)) {
            [IO.Directory]::Move($backup, $published)
            $backup = $null
        }
        throw
    }
    if ($backup) {
        try { Remove-Item -LiteralPath $backup -Recurse -Force -ErrorAction Stop }
        catch { Write-Warning "Previous package retained at $backup; the new package is complete." }
    }
    Write-Output "VideoDock staged: $published"
} finally {
    if ($staging -and (Test-Path $staging)) {
        Remove-Item -LiteralPath $staging -Recurse -Force -ErrorAction SilentlyContinue
    }
    Pop-Location
}
