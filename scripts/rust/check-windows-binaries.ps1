param([Parameter(Mandatory = $true)][string]$Directory)
$ErrorActionPreference = 'Stop'
foreach ($file in Get-ChildItem -LiteralPath $Directory -File -Recurse | Where-Object Extension -In '.exe', '.dll') {
    $reader = [IO.BinaryReader]::new([IO.File]::OpenRead($file.FullName))
    try {
        $valid = $false
        if ($reader.BaseStream.Length -ge 64 -and $reader.ReadUInt16() -eq 0x5a4d) {
            $reader.BaseStream.Position = 0x3c
            $offset = $reader.ReadInt32()
            if ($offset -ge 64 -and $offset -le $reader.BaseStream.Length - 26) {
                $reader.BaseStream.Position = $offset
                $valid = $reader.ReadUInt32() -eq 0x4550 -and $reader.ReadUInt16() -eq 0x8664
                $reader.BaseStream.Position = $offset + 24
                $valid = $valid -and $reader.ReadUInt16() -eq 0x20b
            }
        }
        if (-not $valid) { throw "Invalid Windows x64 binary: $($file.FullName)" }
    } finally { $reader.Dispose() }
}
