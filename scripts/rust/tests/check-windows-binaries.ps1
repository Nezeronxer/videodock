$ErrorActionPreference = 'Stop'
$check = Join-Path $PSScriptRoot '../check-windows-binaries.ps1'
$root = Join-Path ([IO.Path]::GetTempPath()) ('videodock-pe-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory $root | Out-Null
try {
    $valid = New-Object byte[] 128
    $valid[0] = 0x4d; $valid[1] = 0x5a; $valid[60] = 64
    $valid[64] = 0x50; $valid[65] = 0x45
    $valid[68] = 0x64; $valid[69] = 0x86
    $valid[88] = 0x0b; $valid[89] = 0x02
    $file = Join-Path $root 'runtime.dll'
    [IO.File]::WriteAllBytes($file, $valid)
    & $check -Directory $root
    foreach ($case in @('zeroed', 'truncated', 'offset', 'signature', 'architecture', 'optional-header')) {
        $image = $valid.Clone()
        switch ($case) {
            'zeroed' { $image = New-Object byte[] 128 }
            'truncated' { $image = New-Object byte[] 32 }
            'offset' { $image[60] = 127 }
            'signature' { $image[64] = 0 }
            'architecture' { $image[68] = 0x4c; $image[69] = 0x01 }
            'optional-header' { $image[89] = 0x01 }
        }
        [IO.File]::WriteAllBytes($file, $image)
        $rejected = $false
        try { & $check -Directory $root } catch {
            if ($_.Exception.Message -notlike 'Invalid Windows x64 binary:*') { throw }
            $rejected = $true
        }
        if (-not $rejected) { throw "Invalid $case binary was accepted" }
    }
    Write-Output 'Windows binary headers: valid x64 accepted; six corrupt headers rejected.'
} finally {
    Remove-Item -LiteralPath $root -Recurse -Force
}
