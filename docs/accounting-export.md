# Double-entry CSV reports

Open the transaction export dialog, choose a report and file language, then select a destination. Thai is the default export language; English changes headers, system-account labels and status text, not user-authored names or descriptions.

| Report | Contents |
| --- | --- |
| General journal | Date, entry ID/type/status, reversal reference, account ID/name, debit, credit and description; debits precede credits |
| General ledger | Postings grouped by account with a running debit/credit balance |
| Trial balance | Debit/credit activity and closing balances by account, plus balancing totals |

Reports cover recorded history through today, not just the dashboard's visible month. Dates use Gregorian `YYYY-MM-DD` and amounts use the ledger currency. There is no reporting-period lock or selectable closing period.

Opening balances are equity, not income. Credit-card liability display does not reverse accounting signs. Cancelled originals and their reversals remain in the report so both sides are traceable. Entries permanently removed with an account cannot appear in later exports.

Values come from saved postings and exact integer arithmetic, never model calculations. Stable application account IDs are not a certified statutory chart of accounts. These reports do not implement a full statutory accounting system, invoice accruals, VAT subledgers or withholding remittance. Receipt VAT notes do not automatically create tax postings; see [tax coverage](tax-engine.md).

CSV does not embed fonts. For older spreadsheet software, import as UTF-8 with comma separators and preserve dates/IDs as text where needed. See [data transfer](data-transfer.md) for restorable backups.

Run `cargo test -p ledger-application --test accounting_export --locked` for report calculations. The infrastructure example `export_accounting_samples` can create synthetic reports in a new private test directory; never publish real exported ledgers as fixtures.
