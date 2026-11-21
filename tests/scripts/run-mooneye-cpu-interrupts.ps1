param([string]$RomRoot)

$ErrorActionPreference = 'Stop'
$romRepoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$romManifest = Get-Content -LiteralPath (Join-Path $romRepoRoot 'tests/fixtures/mooneye-cpu-interrupts.json') -Raw | ConvertFrom-Json
if (-not $RomRoot) { $RomRoot = Join-Path $romRepoRoot 'target/test-roms/mooneye' }
$romResolvedRoot = (Resolve-Path -LiteralPath $RomRoot).Path
foreach ($romFixture in $romManifest.fixtures) {
    foreach ($romPart in @(@('gb', $romFixture.sha256), @('sym', $romFixture.symbols_sha256))) {
        $romPath = Join-Path $romResolvedRoot "acceptance/$($romFixture.id).$($romPart[0])"
        $romActualHash = (Get-FileHash -LiteralPath $romPath -Algorithm SHA256).Hash
        if ($romActualHash -ne $romPart[1]) {
            throw "Mooneye $($romFixture.id) .$($romPart[0]) SHA-256 mismatch: expected $($romPart[1]), got $romActualHash ($romPath)"
        }
    }
    Write-Output "Verified pinned Mooneye $($romFixture.id)"
}
$previousRomRoot = $env:MOONEYE_CPU_ROM_ROOT
Push-Location $romRepoRoot
try {
    $env:MOONEYE_CPU_ROM_ROOT = $romResolvedRoot
    & cargo test --release --test cpu_rom_runner mooneye_interrupt_ -- --ignored --nocapture
    $romTestExit = $LASTEXITCODE
} finally {
    $env:MOONEYE_CPU_ROM_ROOT = $previousRomRoot
    Pop-Location
}
exit $romTestExit
