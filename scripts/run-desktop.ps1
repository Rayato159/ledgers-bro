param([switch]$Personal, [string]$ModelDirectory)
$ErrorActionPreference = 'Stop'
$workspace = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'with-native-tools.ps1')
$previousLibClang = $env:LIBCLANG_PATH
Push-Location $workspace
try {
    Initialize-LedgerNativeTools
    $appArguments = @('--ocr-dir', (Join-Path $workspace '.tools/ocr'))
    if (-not $Personal) { $appArguments += @('--data-dir', '.data/playground') }
    if ($ModelDirectory) { $appArguments += @('--model-dir', $ModelDirectory) }
    cargo run -p ledgers-bro --locked -- @appArguments
    if ($LASTEXITCODE -ne 0) { throw 'The desktop app could not start.' }
} finally { Pop-Location; $env:LIBCLANG_PATH = $previousLibClang }
