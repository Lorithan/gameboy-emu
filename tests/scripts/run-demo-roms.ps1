param([string]$RomRoot)

# #88: verify the pinned demo ROMs and their licence files, then run the
# headless smoke tests. See tests/fixtures/demo-roms.json for sources.

$ErrorActionPreference = 'Stop'
$demoRepoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$demoManifest = Get-Content -LiteralPath (Join-Path $demoRepoRoot 'tests/fixtures/demo-roms.json') -Raw | ConvertFrom-Json
if (-not $RomRoot) { $RomRoot = Join-Path $demoRepoRoot 'target/test-roms' }
$demoRoot = (Resolve-Path -LiteralPath $RomRoot).Path

foreach ($demoRom in $demoManifest.roms) {
    foreach ($demoPart in @(@($demoRom.file, $demoRom.sha256), @($demoRom.license_file, $demoRom.license_sha256))) {
        $demoPath = Join-Path $demoRoot $demoPart[0]
        $demoActual = (Get-FileHash -LiteralPath $demoPath -Algorithm SHA256).Hash
        if ($demoActual -ne $demoPart[1]) {
            throw "Demo $($demoPart[0]) SHA-256 mismatch: expected $($demoPart[1]), got $demoActual ($demoPath)"
        }
    }
    Write-Output "Verified pinned demo $($demoRom.id)"
}

$previousDemoRoot = $env:DEMO_ROM_ROOT
Push-Location $demoRepoRoot
try {
    $env:DEMO_ROM_ROOT = $demoRoot
    & cargo test --release --test demo_roms -- --ignored --nocapture --test-threads=1
    $demoTestExit = $LASTEXITCODE
} finally {
    $env:DEMO_ROM_ROOT = $previousDemoRoot
    Pop-Location
}
exit $demoTestExit
