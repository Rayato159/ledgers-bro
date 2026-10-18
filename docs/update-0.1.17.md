# Updating to 0.1.17

## Changes

- Card-associated recurring bills ask whether you paid the bill with the card or repaid existing card debt. Paying a bill with a card uses that card directly; repaying card debt still requires a bank or cash funding account. The review explains the balance changes before confirmation.
- Manual entry supports up to eight editable cards. Add or remove entries, review each card, and confirm the complete batch once. Saving a card only updates its draft. A failed batch does not partially save entries, and retries retain the same submission IDs.
- The Transport expense category is available in manual entry, batch review, recurring plans, reports, exports and supported prompt classification.

## Installation and data

Back up the ledger in **Settings → Data**, then install over the existing app. Keep the same Windows user or Android package; do not uninstall or clear storage.

This release advances the ledger format from schema 10 to 11 to identify the Transport category and expense-share journal entries. Existing financial rows and preferences are preserved. Schema 10 backups remain importable; backups made by this release require 0.1.17 or later. User profiles and passwords remain at registry schema 2.

Download the Windows x64 MSI or portable ZIP, Android ARM64 test APK, or Android x86_64 emulator APK from the release. Packages retain the existing upgrade identity and Android test signing key. Windows packages are unsigned; Android packages remain test builds. SHA-256 checksums and build details accompany the downloads.

## Shared expenses and receivable filters

Open a recorded expense in Transactions and choose **Split into a receivable**. Enter the debtor and either divide the original amount equally (including yourself in the people count) or enter that person's amount. Review and confirm. Add one debtor at a time from the same expense; combined active shares cannot exceed the original total. Equal shares round down to the smallest currency unit and leave any remainder in your share.

This reclassifies the shared portion from expenses to receivables without another bank debit or card charge. The original payment remains intact, and reports show your remaining personal expense. Principal repayments increase the receiving account and reduce the receivable without creating income. Cancel repayments before cancelling a related receivable; cancel linked receivables before cancelling the original expense. Editing a shared receivable preserves its source and original date.

The Receivables list supports partial debtor-name searches and inclusive **Debt opened from / through** dates. Combine both filters, select the same date for one day, or leave either end open. Clear filters restores the full list. The collection overview remains an unfiltered summary.
