$ErrorActionPreference = 'Stop'
$workspace = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'with-native-tools.ps1')
$previousLibClang = $env:LIBCLANG_PATH
Push-Location $workspace
try {
    Initialize-LedgerNativeTools
    cargo fmt --all -- --check
    if ($LASTEXITCODE -ne 0) { throw 'Formatting check failed.' }
    cargo clippy --workspace --all-targets --locked -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw 'Clippy failed.' }
    cargo test --workspace --locked
    if ($LASTEXITCODE -ne 0) { throw 'Tests failed.' }
} finally { Pop-Location; $env:LIBCLANG_PATH = $previousLibClang }
