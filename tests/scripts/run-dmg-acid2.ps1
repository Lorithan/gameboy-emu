param([string]$RomRoot)

# #84: verify the pinned dmg-acid2 v1.0 ROM and reference image, convert the
# reference PNG to a shade text file (no Cargo dependency), verify that too,
# and run the ignored comparison test. See tests/fixtures/dmg-acid2.json.

$ErrorActionPreference = 'Stop'
$acidRepoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$acidManifest = Get-Content -LiteralPath (Join-Path $acidRepoRoot 'tests/fixtures/dmg-acid2.json') -Raw | ConvertFrom-Json
if (-not $RomRoot) { $RomRoot = Join-Path $acidRepoRoot 'target/test-roms/dmg-acid2' }
$acidRoot = (Resolve-Path -LiteralPath $RomRoot).Path

function Assert-AcidHash([string]$Name, [string]$Expected) {
    $path = Join-Path $acidRoot $Name
    $actual = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash
    if ($actual -ne $Expected) {
        throw "dmg-acid2 $Name SHA-256 mismatch: expected $Expected, got $actual ($path)"
    }
    Write-Output "Verified pinned dmg-acid2 $Name"
}

Assert-AcidHash 'dmg-acid2.gb' $acidManifest.rom.sha256
Assert-AcidHash 'reference-dmg.png' $acidManifest.reference.sha256

# Greyscale FF, AA, 55, 00 are shades 0-3 (White to Black).
Add-Type -AssemblyName System.Drawing
$acidShadeOf = @{ 'FFFFFFFF' = '0'; 'FFAAAAAA' = '1'; 'FF555555' = '2'; 'FF000000' = '3' }
$acidBitmap = [System.Drawing.Bitmap]::FromFile((Join-Path $acidRoot 'reference-dmg.png'))
try {
    if ($acidBitmap.Width -ne 160 -or $acidBitmap.Height -ne 144) {
        throw "dmg-acid2 reference is $($acidBitmap.Width)x$($acidBitmap.Height), expected 160x144"
    }
    $acidText = New-Object System.Text.StringBuilder
    for ($y = 0; $y -lt 144; $y++) {
        for ($x = 0; $x -lt 160; $x++) {
            $c = $acidBitmap.GetPixel($x, $y)
            $key = '{0:X2}{1:X2}{2:X2}{3:X2}' -f $c.A, $c.R, $c.G, $c.B
            if (-not $acidShadeOf.ContainsKey($key)) { throw "dmg-acid2 reference colour $key at $x, $y" }
            [void]$acidText.Append($acidShadeOf[$key])
        }
        [void]$acidText.Append("`n")
    }
} finally {
    $acidBitmap.Dispose()
}
[IO.File]::WriteAllText((Join-Path $acidRoot 'reference-dmg.shades'), $acidText.ToString(), (New-Object System.Text.UTF8Encoding($false)))
Assert-AcidHash 'reference-dmg.shades' $acidManifest.reference.shades_sha256

$previousAcidRoot = $env:DMG_ACID2_ROOT
Push-Location $acidRepoRoot
try {
    $env:DMG_ACID2_ROOT = $acidRoot
    & cargo test --release --test dmg_acid2 -- --ignored --nocapture
    $acidTestExit = $LASTEXITCODE
} finally {
    $env:DMG_ACID2_ROOT = $previousAcidRoot
    Pop-Location
}
exit $acidTestExit
