# Import, export and device migration

## CSV import

Download the import template from the app and use its exact columns:

```csv
format,external_id,date,kind,account,destination,amount,currency,category,note
```

| Column | Contract |
| --- | --- |
| `format` | `ledgers-bro-v1` |
| `external_id` | Unique 1–100-byte identifier; reuse with identical content is idempotent, conflicting reuse is rejected |
| `date` | Gregorian `YYYY-MM-DD`, not in the future |
| `kind` | `expense`, `income` or `transfer` |
| `account` | Existing compatible account name; coin portfolios are excluded |
| `destination` | Required for transfers; otherwise empty |
| `amount` | Positive decimal with at most 2 decimal places, no grouping commas or currency symbols |
| `currency` | Must match the ledger currency |
| `category` | Expenses: `food`, `snacks`, `rent`, `luxury`, `supplies`, `medical`, `other_expense`; income: `salary`, `freelance`, `interest`, `other_income`; transfers: empty |
| `note` | Optional description |

Files use comma-separated UTF-8; a BOM is accepted. Imports are limited to 1,000 rows and 4 MB and are committed atomically after review. They do not create accounts, classify taxes or link recurring bills. Arbitrary bank CSVs and exported accounting reports are not import templates.

## Spreadsheet reports

Export transactions or [double-entry reports](accounting-export.md) for spreadsheets. Files use UTF-8 with a BOM, quoted CSV fields and protection against formula-like user text. Choose the file language where offered; user-authored names and descriptions remain unchanged. A CSV report is not a full restorable backup.

## Full ledger backup

Open **Settings → Data** and export a `.lbro` file. It covers the current user's saved ledger: accounts, journals/postings, credit-card cycles and settlements, recurring plans, receivables, tax income details, crypto quantities/history, cached prices and saved ledger preferences.

It excludes other local users, passwords, remembered-login tokens, AI models, temporary receipt images and unsaved drafts or tax worksheets. The file is not encrypted and contains financial data; store it privately.

To move data to another device, create a local user with an empty ledger, then restore the file in **Settings → Data**. Restore validates the archive, supported schema and accounting constraints before replacing the empty ledger. It does not merge into or overwrite an existing populated ledger. Backups are limited to 32 MB; foreign, newer or invalid data is rejected.

The new-year reminder asks for a backup while preserving history. Browse by year instead of deleting old entries. Upgrades retain data when installed over the same app; do not uninstall or clear Android storage before backing up.
