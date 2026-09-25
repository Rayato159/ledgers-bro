# Taxable income entries

When entering income, enable its inclusion in Thai tax calculations and select the section 40(1)–(8) classification from the supporting evidence. The app does not infer legal classification from the spending category. Use the actual receipt date.

The transaction amount is cash received. Additional tax fields record gross income excluding VAT, withholding, VAT received and other deductions. All must reconcile exactly:

```text
gross income + VAT - withholding - other deductions = cash received
```

For example, synthetic service income of 10,000 plus VAT of 700 minus withholding of 300 receives 10,400. Synthetic salary of 30,000 minus withholding of 500 and other deductions of 750 receives 28,750. These illustrate reconciliation, not a general tax-rate determination.

The yearly worksheet aggregates tagged gross income by section and the corresponding withholding credit. Untagged, cancelled and other-year entries are excluded. Manually entered worksheet income/withholding means additional amounts not already in tagged transactions; do not enter the same annual certificate total again.

VAT is excluded from PIT income. Other deductions do not automatically become tax allowances. Users still confirm eligibility, documented expenses and allowances in the worksheet. This metadata does not create VAT returns or compound tax postings. See the [implemented coverage and limitations](tax-engine.md).
