# Runs the real packager with isolated fixture executables, without building Rust or changing an installed app.
$ErrorActionPreference = 'Stop'
$project = (Resolve-Path (Join-Path $PSScriptRoot '../../..')).Path
$root = Join-Path ([IO.Path]::GetTempPath()) ('videodock-staging-' + [guid]::NewGuid().ToString('N'))
$keys = @('PATH', 'CARGO_TARGET_DIR', 'VCToolsRedistDir', 'VIDEODOCK_STAGING_FIXTURE_FAIL')
$previous = @{}
foreach ($key in $keys) { $previous[$key] = [Environment]::GetEnvironmentVariable($key) }
try {
    $tools = Join-Path $root 'tools'
    New-Item -ItemType Directory -Force $tools | Out-Null
    $env:CARGO_TARGET_DIR = Join-Path $root 'target'
    $env:VCToolsRedistDir = Join-Path $root 'redist'
    $env:VIDEODOCK_STAGING_FIXTURE_FAIL = $null
    $crt = Join-Path $env:VCToolsRedistDir 'x64/Microsoft.VC143.CRT'
    New-Item -ItemType Directory -Force $crt | Out-Null
    $source = Join-Path $tools 'fixture.cs'
    @'
using System;
using System.IO;
using System.Reflection;
class Fixture {
    static int Main(string[] args) {
        string name = Path.GetFileNameWithoutExtension(Assembly.GetExecutingAssembly().Location).ToLowerInvariant();
        string fail = Environment.GetEnvironmentVariable("VIDEODOCK_STAGING_FIXTURE_FAIL");
        if (name == "cargo") {
            string target = Path.Combine(Environment.GetEnvironmentVariable("CARGO_TARGET_DIR"), "release");
            Directory.CreateDirectory(target);
            byte[] image = File.ReadAllBytes(Assembly.GetExecutingAssembly().Location);
            int pe = BitConverter.ToInt32(image, 0x3c);
            image[pe + 24 + 68] = 2; image[pe + 24 + 69] = 0;
            File.WriteAllBytes(Path.Combine(target, "videodock-desktop.exe"), image);
        } else if (name == "windeployqt") {
            string stage = Path.GetDirectoryName(args[args.Length - 1]);
            Directory.CreateDirectory(Path.Combine(stage, "platforms"));
            foreach (string file in new [] {"QtWebEngineProcess.exe", "platforms/qoffscreen.dll", "platforms/qwindows.dll"})
                File.Copy(Assembly.GetExecutingAssembly().Location, Path.Combine(stage, file), true);
            if (fail == "binary") File.WriteAllBytes(Path.Combine(stage, "platforms/qwindows.dll"), new byte[128]);
            if (fail == "deploy") return 17;
        } else if (name == "ffmpeg" && args.Length > 0 && args[0] == "-L" && fail == "license") {
            return 18;
        } else if (name == "qmake") {
            Console.WriteLine("6.8.3");
        } else {
            Console.WriteLine("fixture notice");
        }
        return 0;
    }
}
'@ | Set-Content -Encoding UTF8 $source
    $compiler = Join-Path $env:WINDIR 'Microsoft.NET/Framework64/v4.0.30319/csc.exe'
    if (-not (Test-Path $compiler -PathType Leaf)) { throw 'Windows .NET Framework C# fixture compiler absent' }
    $fixture = Join-Path $tools 'fixture.exe'
    & $compiler /nologo /target:exe /platform:x64 "/out:$fixture" $source
    if ($LASTEXITCODE -ne 0) { throw 'Fixture compilation failed' }
    foreach ($name in @('vcruntime140.dll', 'msvcp140.dll')) { Copy-Item $fixture (Join-Path $crt $name) }
    foreach ($name in @('cargo', 'git', 'ffmpeg', 'ffprobe', 'windeployqt', 'qmake')) {
        Copy-Item $fixture (Join-Path $tools "$name.exe")
    }
    $env:PATH = "$tools;$env:PATH"
    $published = Join-Path $env:CARGO_TARGET_DIR 'package/VideoDock'
    New-Item -ItemType Directory -Force $published | Out-Null
    Set-Content (Join-Path $published 'obsolete-plugin.dll') 'previous package'
    $script = Join-Path $project 'scripts/rust/build-windows.ps1'
    & $script
    if (Test-Path (Join-Path $published 'obsolete-plugin.dll')) { throw 'Stale file leaked into the new package' }
    if (-not (Test-Path (Join-Path $published 'VideoDock.exe') -PathType Leaf)) { throw 'New package is missing' }
    function PackageManifest {
        Get-ChildItem -LiteralPath $published -File -Recurse | Sort-Object FullName | ForEach-Object {
            $_.FullName.Substring($published.Length) + ':' + (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash
        }
    }
    $manifest = @(PackageManifest)
    foreach ($failure in @('deploy', 'license', 'binary')) {
        $env:VIDEODOCK_STAGING_FIXTURE_FAIL = $failure
        $failed = $false
        try { & $script } catch {
            if ($failure -eq 'binary' -and $_.Exception.Message -notlike 'Invalid Windows x64 binary:*') { throw }
            $failed = $true
        }
        if (-not $failed) { throw "Expected $failure to fail" }
        if (@(Compare-Object $manifest @(PackageManifest)).Count) { throw "$failure changed the previous package" }
        if (@(Get-ChildItem (Split-Path $published) -Directory | Where-Object Name -Match '^VideoDock-(stage|backup)-').Count) {
            throw "$failure left a staging or backup directory"
        }
    }
    Write-Output 'Windows package staging: stale files excluded; deployment/license/binary failure preserves the validated package.'
} finally {
    foreach ($key in $keys) { [Environment]::SetEnvironmentVariable($key, $previous[$key]) }
    if (Test-Path $root) { Remove-Item -LiteralPath $root -Recurse -Force }
}
