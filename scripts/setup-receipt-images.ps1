# Pinned portable decoder, local to this workspace; no Windows codec installation.
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$downloads = Join-Path $root '.tools/downloads'
$decoder = Join-Path $root '.tools/ocr/image-decoder'
New-Item -ItemType Directory -Force -Path $downloads | Out-Null
$archive = Join-Path $downloads 'ImageMagick-7.1.2-31-portable-Q8-x64.7z'
$hash = '4EB7914050902C52BF388BAE188FDBDB04154CA89D105B2F6200A29CB774241B'
if (-not (Test-Path -LiteralPath $archive) -or (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ne $hash) {
    Invoke-WebRequest 'https://github.com/ImageMagick/ImageMagick/releases/download/7.1.2-31/ImageMagick-7.1.2-31-portable-Q8-x64.7z' -OutFile $archive
}
if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ne $hash) { throw 'Image decoder checksum mismatch.' }
New-Item -ItemType Directory -Force -Path $decoder | Out-Null
& tar -xf $archive -C $decoder
if ($LASTEXITCODE -ne 0) { throw 'Image decoder extraction failed.' }
& (Join-Path $decoder 'magick.exe') -version
if ($LASTEXITCODE -ne 0) { throw 'Image decoder verification failed.' }
