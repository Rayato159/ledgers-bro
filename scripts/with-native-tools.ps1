# Configure libclang only for the invoking process; restore it in the caller.
function Initialize-LedgerNativeTools {
    if ($env:LIBCLANG_PATH -and (Test-Path -LiteralPath (Join-Path $env:LIBCLANG_PATH 'libclang.dll'))) { return }
    $llvmDirectory = 'C:\Program Files\LLVM\bin'
    if (Test-Path -LiteralPath (Join-Path $llvmDirectory 'libclang.dll')) { $env:LIBCLANG_PATH = $llvmDirectory; return }
    $ndkDirectory = Join-Path $env:LOCALAPPDATA 'Android\Sdk\ndk'
    if (Test-Path -LiteralPath $ndkDirectory) {
        $ndk = Get-ChildItem -LiteralPath $ndkDirectory -Directory | Sort-Object { [version]$_.Name } -Descending | Select-Object -First 1
        if ($ndk) {
            $llvmDirectory = Join-Path $ndk.FullName 'toolchains\llvm\prebuilt\windows-x86_64\bin'
            if (Test-Path -LiteralPath (Join-Path $llvmDirectory 'libclang.dll')) { $env:LIBCLANG_PATH = $llvmDirectory; return }
        }
    }
    throw 'Install LLVM or the Android NDK, or point LIBCLANG_PATH at the directory containing libclang.dll.'
}
