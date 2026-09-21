# Run once during development/packaging. Receipt OCR itself does not use a network.
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$runtimeRoot = Join-Path $projectRoot '.tools/ocr'
$downloadRoot = Join-Path $projectRoot '.tools/downloads'
New-Item -ItemType Directory -Force -Path $downloadRoot | Out-Null
function Get-VerifiedFile([string]$Uri, [string]$Destination, [string]$Sha256) {
    if ((Test-Path -LiteralPath $Destination) -and (Get-FileHash -LiteralPath $Destination -Algorithm SHA256).Hash -eq $Sha256) { return }
    Invoke-WebRequest -Uri $Uri -OutFile $Destination
    if ((Get-FileHash -LiteralPath $Destination -Algorithm SHA256).Hash -ne $Sha256) { throw ('Checksum mismatch: ' + $Destination) }
}
if (-not (Test-Path -LiteralPath (Join-Path $runtimeRoot 'tesseract.exe'))) {
    if (Test-Path -LiteralPath $runtimeRoot) { throw 'OCR directory already exists without a runtime. Inspect it before retrying; setup will not delete it.' }
    $installerPath = Join-Path $downloadRoot 'tesseract-setup.exe'
    Get-VerifiedFile 'https://github.com/tesseract-ocr/tesseract/releases/download/5.5.3/tesseract-ocr-w64-setup-5.5.3.20260724.exe' $installerPath 'BEE9E3434BD94FD65387D9BE28CD467A41F61B1275383B55B0F59A1331270AE4'
    $installerProcess = Start-Process -FilePath $installerPath -ArgumentList '/S', '/CURRENTUSER', ('/D=' + $runtimeRoot) -WindowStyle Hidden -Wait -PassThru
    if ($installerProcess.ExitCode -ne 0) { throw ('OCR setup failed: ' + $installerProcess.ExitCode) }
}
$modelRevision = '87416418657359cb625c412a48b6e1d6d41c29bd'
$modelRoot = Join-Path $runtimeRoot 'tessdata'
New-Item -ItemType Directory -Force -Path $modelRoot | Out-Null
Get-VerifiedFile "https://raw.githubusercontent.com/tesseract-ocr/tessdata_fast/$modelRevision/tha.traineddata" (Join-Path $modelRoot 'tha.traineddata') '294227CC2D1292B0ACB28D61D4115C88252B96D466CA90B417CF4CF0C67BF07C'
Get-VerifiedFile "https://raw.githubusercontent.com/tesseract-ocr/tessdata_fast/$modelRevision/eng.traineddata" (Join-Path $modelRoot 'eng.traineddata') '7D4322BD2A7749724879683FC3912CB542F19906C83BCC1A52132556427170B2'
$modelRevision | Set-Content -LiteralPath (Join-Path $modelRoot 'model-revision.txt')
& (Join-Path $runtimeRoot 'tesseract.exe') --list-langs --tessdata-dir $modelRoot
if ($LASTEXITCODE -ne 0) { throw 'OCR verification failed.' }
& (Join-Path $PSScriptRoot 'setup-receipt-images.ps1')
