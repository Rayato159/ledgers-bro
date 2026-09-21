# Local Windows development package, not a signed Store artifact.
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'with-native-tools.ps1')
$previousLibClang = $env:LIBCLANG_PATH
$runtimeRoot = Join-Path $projectRoot '.tools/ocr'
if (-not (Test-Path -LiteralPath (Join-Path $runtimeRoot 'tesseract.exe'))) { throw 'Run scripts/setup-ocr.ps1 first.' }
if (-not (Test-Path -LiteralPath (Join-Path $runtimeRoot 'image-decoder/magick.exe'))) { throw 'Run scripts/setup-receipt-images.ps1 first.' }
Push-Location $projectRoot
try {
    Initialize-LedgerNativeTools
    cargo build -p ledgers-bro --release --locked --offline
    if ($LASTEXITCODE -ne 0) { throw 'Release build failed.' }
    $fontNotices = Join-Path $projectRoot 'target/release/licenses/THSarabunNew'
    New-Item -ItemType Directory -Force -Path $fontNotices | Out-Null
    Get-ChildItem -LiteralPath (Join-Path $projectRoot 'crates/ui/assets/fonts') -File | Copy-Item -Destination $fontNotices -Force
    $outputRoot = Join-Path $projectRoot 'target/release/ocr'
    New-Item -ItemType Directory -Force -Path $outputRoot | Out-Null
    Copy-Item -LiteralPath (Join-Path $runtimeRoot 'tesseract.exe') -Destination $outputRoot
    Get-ChildItem -LiteralPath $runtimeRoot -Filter '*.dll' -File | Copy-Item -Destination $outputRoot
    foreach ($subdirectory in @('doc', 'tessdata', 'image-decoder')) {
        $destination = Join-Path $outputRoot $subdirectory
        New-Item -ItemType Directory -Force -Path $destination | Out-Null
        Get-ChildItem -LiteralPath (Join-Path $runtimeRoot $subdirectory) | Copy-Item -Destination $destination -Recurse -Force
    }
    Write-Output 'Local package: target/release/ledgers-bro.exe with sibling ocr directory. Keep the ocr directory with the executable.'
} finally { Pop-Location; $env:LIBCLANG_PATH = $previousLibClang }
