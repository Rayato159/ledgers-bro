param(
    [ValidateSet('x86_64-linux-android', 'aarch64-linux-android')]
    [string]$Target = 'x86_64-linux-android',
    [string]$SdkRoot = $env:ANDROID_HOME,
    [string]$NdkRoot = $env:NDK_HOME,
    [string]$JdkRoot = $env:JAVA_HOME
)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
& (Join-Path $PSScriptRoot 'setup-android-ocr.ps1')
if (-not $SdkRoot) { $SdkRoot = Join-Path $env:LOCALAPPDATA 'Android\Sdk' }
if (-not $JdkRoot) {
    # JDK 21 is supported by the pinned Android Gradle/Kotlin toolchain.
    $JdkRoot = 'C:\Program Files\Java\jdk-21'
    if (-not (Test-Path -LiteralPath $JdkRoot)) { $JdkRoot = 'C:\Program Files\Android\Android Studio\jbr' }
}
if (-not $NdkRoot) {
    $ndk = Get-ChildItem -LiteralPath (Join-Path $SdkRoot 'ndk') -Directory |
        Sort-Object { [version]$_.Name } -Descending | Select-Object -First 1
    if ($ndk) { $NdkRoot = $ndk.FullName }
}
if (-not $NdkRoot) { throw 'Install NDK (Side by side) in Android Studio SDK Manager first.' }
$androidCmake = Get-ChildItem -LiteralPath (Join-Path $SdkRoot 'cmake') -Directory | Sort-Object { [version]$_.Name } -Descending | Select-Object -First 1
if (-not $androidCmake) { throw 'Install CMake in Android Studio SDK Manager first.' }
$cmakeBin = Join-Path $androidCmake.FullName 'bin'
foreach ($required in @(
    (Join-Path $SdkRoot 'platform-tools\adb.exe'),
    (Join-Path $JdkRoot 'bin\java.exe'),
    (Join-Path $NdkRoot 'toolchains\llvm\prebuilt\windows-x86_64\bin\clang.exe')
)) {
    if (-not (Test-Path -LiteralPath $required)) { throw "Missing build tool: $required" }
}
if (-not (Get-Command dx -ErrorAction SilentlyContinue)) { throw 'Install Dioxus CLI 0.7.2 first.' }
$installedTargets = @(rustup target list --installed)
if ($Target -notin $installedTargets) { throw "Install Rust target first: rustup target add $Target" }
$originalEnvironment = @{}
$bindgenTargetKey = 'BINDGEN_EXTRA_CLANG_ARGS_' + $Target.Replace('-', '_')
$originalEnvironment[$bindgenTargetKey] = [Environment]::GetEnvironmentVariable($bindgenTargetKey, 'Process')
$bindgenExactKey = 'BINDGEN_EXTRA_CLANG_ARGS_' + $Target
$originalEnvironment[$bindgenExactKey] = [Environment]::GetEnvironmentVariable($bindgenExactKey, 'Process')
foreach ($key in @('JAVA_HOME', 'ANDROID_HOME', 'ANDROID_SDK_ROOT', 'NDK_HOME', 'ANDROID_NDK_HOME', 'ANDROID_NDK', 'ANDROID_API_LEVEL', 'LIBCLANG_PATH', 'CMAKE_GENERATOR', 'JAVA_TOOL_OPTIONS', 'GRADLE_OPTS', 'GRADLE_USER_HOME', 'LEDGER_ANDROID_PROJECT_ROOT', 'LEDGER_ANDROID_ABI', 'PATH')) {
    $originalEnvironment[$key] = [Environment]::GetEnvironmentVariable($key, 'Process')
}
Push-Location (Join-Path $projectRoot 'apps\android')
try {
    $env:GRADLE_USER_HOME = Join-Path $projectRoot '.tools/android-gradle'
    $env:LEDGER_ANDROID_PROJECT_ROOT = $projectRoot
    $env:LEDGER_ANDROID_ABI = if ($Target -eq 'x86_64-linux-android') { 'x86_64' } else { 'arm64-v8a' }
    $env:JAVA_HOME = $JdkRoot
    $env:ANDROID_HOME = $SdkRoot
    $env:ANDROID_SDK_ROOT = $SdkRoot
    $env:NDK_HOME = $NdkRoot
    $env:ANDROID_NDK_HOME = $NdkRoot
    $env:ANDROID_NDK = $NdkRoot
    $env:ANDROID_API_LEVEL = '24'
    $env:LIBCLANG_PATH = Join-Path $NdkRoot 'toolchains\llvm\prebuilt\windows-x86_64\bin'
    # NDK 30 requires the API level in the clang target, including bindgen.
    [Environment]::SetEnvironmentVariable($bindgenTargetKey, "--target=${Target}24", 'Process')
    [Environment]::SetEnvironmentVariable($bindgenExactKey, "--target=${Target}24", 'Process')
    $env:CMAKE_GENERATOR = 'Ninja'
    $env:PATH = "$cmakeBin;$(Join-Path $JdkRoot 'bin');$(Join-Path $SdkRoot 'platform-tools');$env:PATH"
    # Windows AF_UNIX sockets can fail with an 8.3 TEMP path (e.g. HASHTA~1).
    # Keep Java's temporary socket paths explicit and scoped to this build.
    $javaTemp = Join-Path $projectRoot '.tools\java-tmp'
    New-Item -ItemType Directory -Path $javaTemp -Force | Out-Null
    $tempOptions = '-Djava.io.tmpdir="{0}" -Djdk.net.unixdomain.tmpdir="{0}"' -f $javaTemp
    $env:JAVA_TOOL_OPTIONS = "$($originalEnvironment['JAVA_TOOL_OPTIONS']) $tempOptions".Trim()
    $env:GRADLE_OPTS = "$($originalEnvironment['GRADLE_OPTS']) -Dorg.gradle.daemon=false".Trim()
    & dx build --platform android --target $Target --package ledgers-bro-android --no-default-features --features mobile --locked
    if ($LASTEXITCODE -ne 0) { throw "Android build failed (exit $LASTEXITCODE)." }
    $builtApk = Join-Path $projectRoot 'target\dx\ledgers-bro-android\debug\android\app\app\build\outputs\apk\debug\app-debug.apk'
    if (-not (Test-Path -LiteralPath $builtApk)) { throw 'Build returned success without the expected debug APK.' }
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $apkArchive = [IO.Compression.ZipFile]::OpenRead($builtApk)
    try {
        $nativeLibraries = @($apkArchive.Entries | Where-Object { $_.FullName -match '^lib/[^/]+/[^/]+\.so$' })
        $expectedLibrary = "lib/$($env:LEDGER_ANDROID_ABI)/libdioxusmain.so"
        if ($expectedLibrary -notin $nativeLibraries.FullName) { throw "APK is missing $expectedLibrary." }
        $unexpectedLibraries = @($nativeLibraries | Where-Object { -not $_.FullName.StartsWith("lib/$($env:LEDGER_ANDROID_ABI)/", [StringComparison]::Ordinal) })
        if ($unexpectedLibraries.Count -gt 0) { throw 'APK contains libraries for another Android ABI.' }
    } finally { $apkArchive.Dispose() }
    $apkDirectory = Join-Path $projectRoot 'target\android'
    New-Item -ItemType Directory -Path $apkDirectory -Force | Out-Null
    $abiLabel = if ($Target -eq 'x86_64-linux-android') { 'x86_64' } else { 'arm64' }
    $outputApk = Join-Path $apkDirectory "ledgers-bro-test-$abiLabel.apk"
    Copy-Item -LiteralPath $builtApk -Destination $outputApk -Force
    $apkHash = (Get-FileHash -LiteralPath $outputApk -Algorithm SHA256).Hash.ToLowerInvariant()
    "$apkHash  $([IO.Path]::GetFileName($outputApk))" | Set-Content -LiteralPath ([IO.Path]::ChangeExtension($outputApk, '.sha256'))
    Write-Output "Test APK: $outputApk"
    Write-Output 'Separate test app with a debug signature; not a Store release.'
} finally {
    Pop-Location
    foreach ($key in $originalEnvironment.Keys) {
        [Environment]::SetEnvironmentVariable($key, $originalEnvironment[$key], 'Process')
    }
}
