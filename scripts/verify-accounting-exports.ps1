param([string]$Directory = (Join-Path $PSScriptRoot '../.preview/accounting-exports'))
$ErrorActionPreference = 'Stop'
$files = @(Get-ChildItem -LiteralPath $Directory -Filter '*.csv' -File)
if ($files.Count -ne 6) { throw 'Expected exactly six synthetic report files.' }
$culture = [Globalization.CultureInfo]::InvariantCulture
foreach ($file in $files) {
    $bytes = [IO.File]::ReadAllBytes($file.FullName)
    if ($bytes.Length -lt 3 -or $bytes[0] -ne 239 -or $bytes[1] -ne 187 -or $bytes[2] -ne 191) { throw "Missing UTF-8 BOM: $($file.Name)" }
    $strictUtf8 = [Text.UTF8Encoding]::new($false, $true)
    $null = $strictUtf8.GetString($bytes)
    $rows = @(Import-Csv -LiteralPath $file.FullName -Encoding utf8)
    $thai = $file.Name -notmatch '^(General|Trial)'
    $trial = $file.Name -match '^(งบทดลอง|Trial)'
    $debit = if ($thai) { if ($trial) { 'ยอดรวมเดบิต (บาท)' } else { 'เดบิต (บาท)' } } else { if ($trial) { 'Debit turnover (THB)' } else { 'Debit (THB)' } }
    $credit = if ($thai) { if ($trial) { 'ยอดรวมเครดิต (บาท)' } else { 'เครดิต (บาท)' } } else { if ($trial) { 'Credit turnover (THB)' } else { 'Credit (THB)' } }
    [decimal]$sumDr = 0; [decimal]$sumCr = 0
    foreach ($row in $rows[0..($rows.Count - 2)]) {
        $sumDr += [decimal]::Parse($row.$debit, $culture)
        $sumCr += [decimal]::Parse($row.$credit, $culture)
    }
    if ($sumDr -ne 2170.25 -or $sumCr -ne 2170.25) { throw "Incorrect fixture totals: $($file.Name)" }
    if ($sumDr -ne [decimal]::Parse($rows[-1].$debit, $culture) -or $sumCr -ne [decimal]::Parse($rows[-1].$credit, $culture)) { throw 'Total row differs from parsed data.' }
    if (-not $trial) {
        $noteColumn = if ($thai) { 'คำอธิบาย' } else { 'Description' }
        $notes = @($rows | ForEach-Object { $_.$noteColumn })
        $expected = "รายการจากใบเสร็จ`n• ข้าว, ชา `"เย็น`" — 80.25 บาท`nยอดสุทธิ 80.25 บาท"
        if ($expected -notin $notes) { throw "Thai bullets, quotes, commas or newlines failed to round trip: $($file.Name)" }
        if (-not ($notes | Where-Object { $_.StartsWith("'=HYPERLINK(") })) { throw 'Spreadsheet formula protection missing.' }
    } else {
        $balanceDr = if ($thai) { 'ยอดคงเหลือเดบิต (บาท)' } else { 'Debit balance (THB)' }
        $balanceCr = if ($thai) { 'ยอดคงเหลือเครดิต (บาท)' } else { 'Credit balance (THB)' }
        if ($rows[-1].$balanceDr -ne '1800.00' -or $rows[-1].$balanceCr -ne '1800.00') { throw 'Trial balance does not reconcile.' }
    }
    Write-Output "PASS: $($file.Name) — Thai UTF-8, parsed rows, debit = credit = 2170.25"
}
