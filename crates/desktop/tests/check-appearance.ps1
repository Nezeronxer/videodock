param([Parameter(Mandatory = $true)][string]$OutputDirectory)
$ErrorActionPreference = 'Stop'
$project = (Resolve-Path (Join-Path $PSScriptRoot '../../..')).Path
$output = [IO.Path]::GetFullPath($OutputDirectory)
$build = Join-Path $env:CARGO_TARGET_DIR 'appearance'
$qt = Split-Path (Split-Path (Get-Command qmake).Source)
New-Item -ItemType Directory -Force $output | Out-Null
Push-Location $project
try {
    & cargo build -p videodock-core --example ui_engine --locked
    if ($LASTEXITCODE -ne 0) { throw 'UI Engine build failed' }
    & cmake -S $PSScriptRoot -B $build -G Ninja "-DCMAKE_PREFIX_PATH=$qt" -DCMAKE_BUILD_TYPE=Release
    if ($LASTEXITCODE -ne 0) { throw 'Appearance configuration failed' }
    & cmake --build $build --parallel 1
    if ($LASTEXITCODE -ne 0) { throw 'Appearance build failed' }
    & windeployqt --release --no-compiler-runtime --qmldir (Join-Path $project 'crates/desktop/qml') (Join-Path $build 'videodock-appearance.exe')
    if ($LASTEXITCODE -ne 0) { throw 'Appearance Qt deployment failed' }
    $package = Join-Path $env:CARGO_TARGET_DIR 'package/VideoDock'
    $env:PATH = "$package;$env:PATH"
    $sample = Join-Path $output 'sample.mp4'
    & ffmpeg -hide_banner -nostdin -loglevel error -y -f lavfi -i color=c=blue:s=160x90:r=10 -f lavfi -i sine=frequency=440:sample_rate=44100 -t 1 -c:v libx264 -pix_fmt yuv420p -c:a aac $sample
    if ($LASTEXITCODE -ne 0) { throw 'Appearance sample generation failed' }
    $results = [ordered]@{}
    $env:QT_FORCE_STDERR_LOGGING = '1'
    foreach ($language in @('en', 'ru')) {
        New-Item -ItemType Directory -Force "$output/$language" | Out-Null
        $arguments = @("`"$project/crates/desktop/qml/Main.qml`"", "`"$output/$language`"", $language,
                       "`"$env:CARGO_TARGET_DIR/debug/examples/ui_engine.exe`"", "`"$sample`"")
        $test = Start-Process -FilePath "$build/videodock-appearance.exe" -ArgumentList $arguments -Wait -PassThru `
            -RedirectStandardOutput "$output/$language-stdout.log" -RedirectStandardError "$output/$language.log"
        $results[$language] = $test.ExitCode
    }
    $results | ConvertTo-Json | Set-Content -Encoding utf8 "$output/result.json"
    if (@($results.Values | Where-Object { $_ -ne 0 }).Count) { throw 'Windows appearance/real Engine check failed' }
} finally {
    Pop-Location
}
