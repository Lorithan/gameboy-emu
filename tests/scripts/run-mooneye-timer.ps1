param(
    [ValidateSet('all', 'div_write', 'rapid_toggle', 'tim00', 'tim00_div_trigger', 'tim01', 'tim01_div_trigger', 'tim10', 'tim10_div_trigger', 'tim11', 'tim11_div_trigger', 'tima_reload', 'tima_write_reloading', 'tma_write_reloading')]
    [string]$Fixture = 'all',
    [string]$RomRoot
)

$ErrorActionPreference = 'Stop'
$romRepoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$romManifest = Get-Content -LiteralPath (Join-Path $romRepoRoot 'tests/fixtures/mooneye-timer.json') -Raw | ConvertFrom-Json
if (-not $RomRoot) { $RomRoot = Join-Path $romRepoRoot 'target/test-roms/mooneye' }
$romResolvedRoot = (Resolve-Path -LiteralPath $RomRoot).Path
$selectedFixtures = @($romManifest.fixtures | Where-Object { $Fixture -eq 'all' -or $_.id -eq $Fixture })
if ($selectedFixtures.Count -eq 0) { throw 'No fixtures selected' }
foreach ($romFixture in $selectedFixtures) {
    $romPath = Join-Path $romResolvedRoot $romFixture.rom_path
    $romActualHash = (Get-FileHash -LiteralPath $romPath -Algorithm SHA256).Hash
    if ($romActualHash -ne $romFixture.sha256) {
        throw "Mooneye $($romFixture.id) SHA-256 mismatch: expected $($romFixture.sha256), got $romActualHash ($romPath)"
    }
    Write-Output "Verified pinned Mooneye $($romFixture.id): $romActualHash"
}
$previousRomRoot = $env:MOONEYE_CPU_ROM_ROOT
Push-Location $romRepoRoot
try {
    $env:MOONEYE_CPU_ROM_ROOT = $romResolvedRoot
    if ($Fixture -eq 'all') {
        & cargo test --test cpu_rom_runner mooneye_timer_ -- --ignored --nocapture
    } else {
        & cargo test --test cpu_rom_runner "mooneye_timer_$Fixture" -- --ignored --exact --nocapture
    }
    $romTestExit = $LASTEXITCODE
} finally {
    $env:MOONEYE_CPU_ROM_ROOT = $previousRomRoot
    Pop-Location
}
exit $romTestExit
