# Version 0.1.13

Receivable cards now support correcting an existing debt without deleting and recreating it. Edit the debtor, description, original principal, opening date, collection schedule and funding account directly from its card.

The review shows changed values, principal already collected, the new outstanding balance and any effect on account balances. Confirm saves and refreshes the page; cancel leaves the ledger unchanged. Existing repayments and interest remain intact. Corrections are atomic and reject stale reviews, invalid dates and principal below the amount already collected.

The ledger schema and installed data location are unchanged. Windows MSI/ZIP and Android ARM64/x86_64 test APKs are included. Windows packages remain unsigned, and Android packages retain the existing test application ID and signing identity.
