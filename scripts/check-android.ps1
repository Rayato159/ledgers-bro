param(
    [string]$SdkRoot = $env:ANDROID_HOME,
    [string]$JdkRoot = $env:JAVA_HOME,
    [string]$NdkRoot = $env:NDK_HOME
)
# Read-only environment report. Never installs packages, accepts licenses,
# changes machine settings, starts a VM, or claims the desktop host is Android-ready.
$ErrorActionPreference = 'Stop'
if (-not $SdkRoot) { $SdkRoot = Join-Path $env:LOCALAPPDATA 'Android/Sdk' }
if (-not $JdkRoot) { $JdkRoot = 'C:/Program Files/Android/Android Studio/jbr' }
if (-not (Test-Path -LiteralPath (Join-Path $JdkRoot 'bin/javac.exe')) -and (Get-Command java -ErrorAction SilentlyContinue)) {
    $javaHomeLine = & java -XshowSettings:properties -version 2>&1 | Where-Object { $_ -match '^\s*java.home\s*=' } | Select-Object -First 1
    if ($javaHomeLine) {
        $detectedJdk = ($javaHomeLine.ToString() -split '=', 2)[1].Trim()
        if (Test-Path -LiteralPath (Join-Path $detectedJdk 'bin/javac.exe')) { $JdkRoot = $detectedJdk }
    }
}
if (-not $NdkRoot -and (Test-Path -LiteralPath (Join-Path $SdkRoot 'ndk'))) {
    $ndk = Get-ChildItem -LiteralPath (Join-Path $SdkRoot 'ndk') -Directory | Sort-Object Name -Descending | Select-Object -First 1
    if ($ndk) { $NdkRoot = $ndk.FullName }
}
$checks = @(
    @{Name='Dioxus CLI'; Ready= [bool](Get-Command dx -ErrorAction SilentlyContinue); Detail='Pinned project CLI: 0.7.2'},
    @{Name='JDK'; Ready= (Test-Path -LiteralPath (Join-Path $JdkRoot 'bin/javac.exe')); Detail=$JdkRoot},
    @{Name='Android SDK'; Ready= (Test-Path -LiteralPath (Join-Path $SdkRoot 'platforms')); Detail=$SdkRoot},
    @{Name='ADB'; Ready= (Test-Path -LiteralPath (Join-Path $SdkRoot 'platform-tools/adb.exe')); Detail='SDK platform-tools'},
    @{Name='Emulator'; Ready= (Test-Path -LiteralPath (Join-Path $SdkRoot 'emulator/emulator.exe')); Detail='SDK Android Emulator'},
    @{Name='Command-line tools'; Ready= (Test-Path -LiteralPath (Join-Path $SdkRoot 'cmdline-tools/latest/bin/sdkmanager.bat')); Detail='SDK command-line tools (latest)'},
    @{Name='NDK'; Ready= [bool]($NdkRoot -and (Test-Path -LiteralPath (Join-Path $NdkRoot 'toolchains/llvm/prebuilt/windows-x86_64/bin/clang.exe'))); Detail=$NdkRoot}
)
$installed = @()
if (Get-Command rustup -ErrorAction SilentlyContinue) { $installed = @(rustup target list --installed) }
foreach ($target in @('x86_64-linux-android','aarch64-linux-android')) {
    $checks += @{Name=$target; Ready=($target -in $installed); Detail='Rust target'}
}
$checks | ForEach-Object { [pscustomobject]@{Check=$_.Name;Status=$(if ($_.Ready) {'FOUND'} else {'MISSING'});Detail=$_.Detail} } | Format-Table -AutoSize
if (Test-Path -LiteralPath (Join-Path $SdkRoot 'emulator/emulator.exe')) {
    & (Join-Path $SdkRoot 'emulator/emulator.exe') -list-avds
}
$projectRoot = Split-Path $PSScriptRoot -Parent
if (Test-Path -LiteralPath (Join-Path $projectRoot 'apps/android/Cargo.toml')) {
    Write-Output 'Android test host present: scripts/build-android.ps1 builds the emulator APK; scripts/run-android.ps1 installs and opens it.'
    Write-Output 'Receipt OCR uses bundled Thai/English models and Android image decoding (Android 9+). This is not a production/Store release.'
    $apkDirectory = Join-Path $projectRoot 'target/android'
    if (Test-Path -LiteralPath $apkDirectory) {
        Get-ChildItem -LiteralPath $apkDirectory -Filter '*.apk' | Select-Object FullName, Length, LastWriteTime
    }
} else {
    Write-Output 'Android host is missing. See docs/android-emulator-th.md.'
}
