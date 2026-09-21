param(
    [string]$Device,
    [ValidateSet('x86_64', 'arm64')]
    [string]$Abi = 'x86_64',
    [string]$SdkRoot = $env:ANDROID_HOME
)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
if (-not $SdkRoot) { $SdkRoot = Join-Path $env:LOCALAPPDATA 'Android\Sdk' }
$adb = Join-Path $SdkRoot 'platform-tools\adb.exe'
if (-not (Test-Path -LiteralPath $adb)) { throw 'ADB is missing. Install SDK Platform-Tools first.' }
$apk = Join-Path $projectRoot "target\android\ledgers-bro-test-$Abi.apk"
if (-not (Test-Path -LiteralPath $apk)) { throw "Build the $Abi APK with scripts/build-android.ps1 first." }
$deviceLines = @(& $adb devices)
if ($LASTEXITCODE -ne 0) { throw 'Cannot list Android devices.' }
$onlineDevices = @($deviceLines | ForEach-Object {
    if ($_ -match '^(\S+)\s+device$') { $Matches[1] }
})
if (-not $Device) {
    if ($onlineDevices.Count -ne 1) { throw 'Specify -Device using a connected serial from adb devices.' }
    $Device = $onlineDevices[0]
}
if ($Device -notin $onlineDevices) { throw "Device $Device is not online or has not authorized this computer." }
$deviceAbis = (& $adb -s $Device shell getprop ro.product.cpu.abilist).Trim().Split(',')
if ($LASTEXITCODE -ne 0) { throw 'Cannot read device architecture.' }
$requiredAbi = if ($Abi -eq 'arm64') { 'arm64-v8a' } else { 'x86_64' }
if ($requiredAbi -notin $deviceAbis) { throw "APK architecture $requiredAbi does not match the device ($deviceAbis)." }
& $adb -s $Device install -r $apk
if ($LASTEXITCODE -ne 0) { throw 'APK install failed. Existing app data was not removed.' }
# Package replacement can leave an old Activity briefly on top. Restart only this
# test package, retaining its database, then wait for the new Activity to open.
& $adb -s $Device shell am force-stop 'com.dancingwithmycode.ledgersbro.test'
if ($LASTEXITCODE -ne 0) { throw 'Cannot restart the test app after installation.' }
$runningApp = $null
for ($attempt = 0; $attempt -lt 4; $attempt++) {
    & $adb -s $Device shell am start -W -n 'com.dancingwithmycode.ledgersbro.test/dev.dioxus.main.MainActivity'
    if ($LASTEXITCODE -ne 0) { throw 'APK installed but the test app could not be launched.' }
    $runningApp = & $adb -s $Device shell pidof 'com.dancingwithmycode.ledgersbro.test'
    if ($runningApp) { break }
    # Android may acknowledge the old top Activity while replacement is finishing.
    Start-Sleep -Milliseconds 300
}
if (-not $runningApp) { throw 'APK installed but the app process did not start. Check adb logcat.' }
