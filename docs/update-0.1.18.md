# Updating to 0.1.18

## Changes

- Receivables have separate Outstanding debts, By debtor, Debt history and Collection overview tabs.
- Fully repaid and cancelled debts move out of the active list automatically. Partial repayments remain active; reversing a payment restores an outstanding debt to the list. Nothing is deleted or rewritten to archive a debt.
- By debtor groups remaining balances by full name, ignoring case and extra whitespace. Name/date filters also constrain these totals. Expand a group to inspect or collect a specific debt.
- The global + Add entry button opens manual cards directly. Repeated presses add drafts while preserving existing edits. Up to eight entries are reviewed and saved together; incomplete rows cannot be saved.

## Updating and data

Back up in Settings > Data, then install over the existing app without uninstalling or clearing storage. Ledger schema remains 11 and user registry remains 2. Existing users, passwords, balances and records are unchanged by this update. Schema 10 and 11 backups remain importable.

Windows x64 packages are unsigned MSI and portable ZIP files. Android ARM64 and x86_64 APKs retain the existing test package/signing identity; they are not Play Store releases. SHA256SUMS.txt and BUILDINFO.txt accompany the release. Native checks use isolated fictional profiles and a dedicated Android emulator, not production data or a physical phone.
