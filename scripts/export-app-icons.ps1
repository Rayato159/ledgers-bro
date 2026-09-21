param([string]$Magick)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
if (-not $Magick) { $Magick = Join-Path $root '.tools/ocr/image-decoder/magick.exe' }
if (-not (Test-Path -LiteralPath $Magick)) { throw 'Install ImageMagick with scripts/setup-receipt-images.ps1 or pass -Magick.' }
$source = Join-Path $root 'design/app-icon/tanuki-icon-source.png'
$res = Join-Path $root 'apps/android/native/res'
$master = Join-Path $root 'design/app-icon/launcher-1024.png'
if (-not (Test-Path -LiteralPath $source)) { throw 'Missing generated icon source.' }

function Export-Icon([string[]]$Arguments) {
    & $Magick @Arguments
    if ($LASTEXITCODE -ne 0) { throw 'App icon export failed.' }
}

# Mechanical asset packaging only: retain the generated drawing and its alpha.
# Android masks the separate foreground/background layers at runtime.
$foreground = Join-Path $res 'drawable-nodpi'
New-Item -ItemType Directory -Force -Path $foreground | Out-Null
Export-Icon @($source, '-trim', '+repage', '-resize', '232x232', '-gravity', 'center', '-background', 'none', '-extent', '432x432', '-strip', (Join-Path $foreground 'ledger_launcher_foreground.png'))
Export-Icon @($source, '-trim', '+repage', '-resize', '820x820', '-gravity', 'center', '-background', '#FFE3A5', '-extent', '1024x1024', '-alpha', 'remove', '-alpha', 'off', '-strip', $master)

$sizes = @{ mdpi = 48; hdpi = 72; xhdpi = 96; xxhdpi = 144; xxxhdpi = 192 }
foreach ($density in $sizes.Keys) {
    $directory = Join-Path $res "mipmap-$density"
    New-Item -ItemType Directory -Force -Path $directory | Out-Null
    $size = $sizes[$density]
    Export-Icon @($master, '-resize', "${size}x${size}", '-strip', (Join-Path $directory 'ledger_launcher.png'))
}
Export-Icon @($master, '-resize', '512x512', '-strip', (Join-Path $root 'design/app-icon/launcher-512.png'))
Write-Output 'Generated launcher master, adaptive foreground, and five legacy density resources.'
