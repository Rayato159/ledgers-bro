# Version 0.1.2 verification

Checked on Windows and an isolated Android x86_64 emulator on 25 September 2026, using synthetic ledgers. No personal ledger was used in these checks.

- `cargo fmt --all -- --check` passed.
- `cargo test --workspace --locked`: 217 passed, 0 failed, 2 ignored. The ignored cases require separate OCR runtimes/fixtures.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` passed.
- Windows release MSI/portable application and Android ARM64/x86_64 test APKs built successfully.
- Both APKs report version 0.1.2 / code 1002, retain the existing package ID and signing certificate, and pass signature verification.

## Data and native UI checks

The emulator was updated in place from the published 0.1.1 APK. A schema-9 fixture with 4 accounts, 7 journal entries, 14 postings, transaction tax details and a recurring bill migrated to schema 10 and was adopted by the legacy local user. Every original column and row matched the pre-update snapshot. The same comparison passed on Windows.

The Android system document picker was used to import a two-row Thai CSV, export a saved-ledger backup, and restore it into another user. Re-exported financial tables matched the original backup exactly; the only preference difference was an intentionally changed theme. Subsequent user switching and restores passed after combining completion status and payload retrieval in one Android UI-thread dispatch. Backup filenames retain the `.lbro` extension.

Windows UI checks covered legacy-user setup and login, navigation, recurring-plan editing, next-month defaults for passed due days, the in-app CSV example, and creating a portfolio with `0.01234567 BTC` and `1.123456789 SOL`. The application fetched both public THB quotes and displayed their timestamp. Offline/error handling remains covered by tests and visible status messages; prices are periodically refreshed estimates.

The data-transfer layout was checked in light and dark themes on desktop and mobile: action placement, full-width mobile buttons, dismissible feedback, 24-pixel spacing after the card grid, and no horizontal page overflow. The new character assets render in the native apps without color filters.

## Limits

No new physical Android-device test, complete interactive MSI installation test, iOS build, or complete tax-law certification is claimed. Local passwords do not encrypt SQLite files or exported backups. Update the existing Android installation without uninstalling it or clearing its storage to retain local data.
