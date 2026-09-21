# Synthetic data only. Produces a deterministic, high-contrast bilingual OCR fixture.
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$fixtureRoot = Join-Path (Split-Path -Parent $PSScriptRoot) 'tests/fixtures'
New-Item -ItemType Directory -Force -Path $fixtureRoot | Out-Null
$canvas = New-Object System.Drawing.Bitmap(1000, 1200)
$graphics = [System.Drawing.Graphics]::FromImage($canvas)
$font = New-Object System.Drawing.Font('Tahoma', 30)
try {
    $graphics.Clear([System.Drawing.Color]::White)
    $graphics.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
    $receiptLines = @('TEST CAFE', 'ใบเสร็จรับเงิน', '20/09/2569', '', 'Coffee             80.00', 'Lunch              70.00', '', 'Subtotal          150.00', 'VAT 7%             10.50', 'Grand Total       160.50', 'Cash              200.00', 'Change             39.50', '', 'SYNTHETIC TEST - NOT A TAX INVOICE')
    $offsetY = 50
    foreach ($receiptLine in $receiptLines) {
        $graphics.DrawString($receiptLine, $font, [System.Drawing.Brushes]::Black, 55, $offsetY)
        $offsetY += 72
    }
    $canvas.Save((Join-Path $fixtureRoot 'receipt-th-en.png'), [System.Drawing.Imaging.ImageFormat]::Png)
} finally { $font.Dispose(); $graphics.Dispose(); $canvas.Dispose() }
