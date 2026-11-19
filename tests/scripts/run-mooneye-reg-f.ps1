param([string]$RomPath)

$ErrorActionPreference = 'Stop'
$romRepoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$romManifest = Get-Content -LiteralPath (Join-Path $romRepoRoot 'tests/fixtures/mooneye-reg-f.json') -Raw | ConvertFrom-Json
if (-not $RomPath) {
    $RomPath = Join-Path $romRepoRoot $romManifest.rom.cache_path
}
$romResolvedPath = (Resolve-Path -LiteralPath $RomPath).Path
$romActualHash = (Get-FileHash -LiteralPath $romResolvedPath -Algorithm SHA256).Hash
if ($romActualHash -ne $romManifest.rom.sha256) {
    throw "Mooneye reg_f SHA-256 mismatch: expected $($romManifest.rom.sha256), got $romActualHash ($romResolvedPath)"
}
Write-Output "Verified pinned Mooneye reg_f: $romActualHash"
$previousRomPath = $env:MOONEYE_REG_F
Push-Location $romRepoRoot
try {
    $env:MOONEYE_REG_F = $romResolvedPath
    & cargo test --test cpu_rom_runner mooneye_reg_f_external -- --ignored --exact --nocapture
    $romTestExit = $LASTEXITCODE
} finally {
    $env:MOONEYE_REG_F = $previousRomPath
    Pop-Location
}
exit $romTestExit
