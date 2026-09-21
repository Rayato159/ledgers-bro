# Account deletion verification — 2026-09-20

Implemented: **บัญชี → ลบบัญชี → ตรวจจำนวนรายการ/ผลต่อบัญชีคู่โอน → ลบบัญชีและรายการทั้งหมด**. The dialog initially focuses **เก็บบัญชีไว้**. Cancel/Escape dismiss it without a mutation. The account and related transactions are permanently removed from the live ledger. Whole transfers and their reversals are removed; independent counterparty transactions stay intact.

## Automated checks

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` passed.
- `cargo test --workspace --locked --offline`: 60 passed, 1 optional native OCR test ignored. OCR was not changed in this feature.
- Debug build and `scripts/package-desktop.ps1` release build/package passed.
- Six new SQLite/application integration tests cover:
  - Complete deletion across income, expense, transfers in both directions, and reversed expense/transfer entries; exact retained entries and balances; reopening; foreign-key and posting checks.
  - Deleting the last account, empty totals, rejecting repeated confirmation, and reusing a name with a new identity.
  - Another connection adding a transaction on the target or counterparty after preview: reject stale review without deleting anything, then accept a fresh review.
  - Injected failure at the final account DELETE after all related posting/journal deletes: all data survives rollback and reopening; retry succeeds once the fault is removed.
  - Old prepared drafts and replayed deleted transfers cannot reintroduce transactions referencing a deleted account.
  - Removing a transfer cannot leave another account's balance outside supported money bounds.

## Actual Windows app / WebView2

Used a separate synthetic ledger at `.data/account-deletion-ui-test`, not the personal database. Through UI controls, created cash 1,000 and another account 500, transferred 200 to the second account, and recorded expenses of 80 and 40 respectively. Created an unsaved draft for 12 with a note before deletion.

- Preview correctly showed 2 related transactions, including 1 transfer, and counterparty balance **660.00 → 460.00**.
- Initial focus was the keep-account action; cancel preserved both accounts.
- Reopened and confirmed through the visible button at 390 × 844 viewport: dialog closed, success displayed, only the remaining account with 460.00 remained. Its independent expense of 40 remained in history.
- Unsaved draft retained its amount and note, cleared the deleted account selection, and could not be confirmed until a valid account was chosen again.
- Captured and visually inspected desktop and phone-width dialogs: `.preview/account-deletion-desktop.png`, `.preview/account-deletion-mobile.png`. No horizontal overflow at 390 px; no browser errors reported.
- Reproduction scripts: `.preview/verify-account-deletion.js` (fresh synthetic ledger only), `.preview/verify-account-deletion-result.js` (after confirming that test deletion).

Mobile width is a desktop WebView layout check, not Android/iOS device validation. Deletion is logical removal from the live database, not guaranteed forensic erasure from SQLite storage, OS backups, or exported files.
