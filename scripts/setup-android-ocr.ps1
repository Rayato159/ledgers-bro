$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$downloads = Join-Path $root '.tools/downloads'
$assets = Join-Path $root '.tools/android-ocr/assets/tessdata'
New-Item -ItemType Directory -Force -Path $downloads, $assets | Out-Null
function Get-ReceiptDependency([string]$Uri, [string]$Path, [string]$Hash) {
    if (-not (Test-Path -LiteralPath $Path) -or (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash -ne $Hash) {
        Invoke-WebRequest -Uri $Uri -OutFile $Path
    }
    if ((Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash -ne $Hash) { throw "Receipt dependency checksum mismatch: $Path" }
}
Get-ReceiptDependency 'https://jitpack.io/cz/adaptech/tesseract4android/tesseract4android/4.9.0/tesseract4android-4.9.0.aar' (Join-Path $downloads 'tesseract4android-4.9.0.aar') 'BCE5D6413A1A5AE3D7240033FBBC851BA3217D0A08D9769400E17A077F42CB2A'
$revision = '87416418657359cb625c412a48b6e1d6d41c29bd'
$hashes = @{
    tha = '294227CC2D1292B0ACB28D61D4115C88252B96D466CA90B417CF4CF0C67BF07C'
    eng = '7D4322BD2A7749724879683FC3912CB542F19906C83BCC1A52132556427170B2'
}
foreach ($language in $hashes.Keys) {
    $target = Join-Path $assets "$language.traineddata"
    $desktopModel = Join-Path $root ".tools/ocr/tessdata/$language.traineddata"
    if (-not (Test-Path -LiteralPath $target) -and (Test-Path -LiteralPath $desktopModel) -and (Get-FileHash -LiteralPath $desktopModel -Algorithm SHA256).Hash -eq $hashes[$language]) {
        Copy-Item -LiteralPath $desktopModel -Destination $target
    }
    Get-ReceiptDependency "https://raw.githubusercontent.com/tesseract-ocr/tessdata_fast/$revision/$language.traineddata" $target $hashes[$language]
}
# Dioxus regenerates Gradle files; use an init script in an isolated Gradle home.
# No writes to the user's global init scripts or caches.
$gradleHome = Join-Path $root '.tools/android-gradle'
$init = Join-Path $gradleHome 'init.d'
New-Item -ItemType Directory -Force -Path $init | Out-Null
Copy-Item -LiteralPath (Join-Path $root 'apps/android/native/receipt.init.gradle') -Destination (Join-Path $init 'receipt.gradle') -Force
# Reuse the already installed wrapper distribution without another large download.
$previousGradle = if ($env:GRADLE_USER_HOME) { $env:GRADLE_USER_HOME } else { Join-Path $env:USERPROFILE '.gradle' }
$wrapper = Join-Path $previousGradle 'wrapper/dists/gradle-9.1.0-bin'
$targetWrapper = Join-Path $gradleHome 'wrapper/dists/gradle-9.1.0-bin'
if ((Test-Path -LiteralPath $wrapper) -and -not (Test-Path -LiteralPath $targetWrapper)) {
    New-Item -ItemType Directory -Force -Path (Split-Path $targetWrapper -Parent) | Out-Null
    Copy-Item -LiteralPath $wrapper -Destination $targetWrapper -Recurse
}
