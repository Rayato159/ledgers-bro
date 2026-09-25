# Remembered login and Settings alignment verification

Checked on Windows and an isolated Android x86_64 emulator on 25 September 2026 with disposable fixture data.

- `cargo test -p ledger-infrastructure --test profiles --locked`: 13 passed. Covers legacy adoption, registry v1-to-v2 migration, isolation, password verification, restart, opt-in/opt-out, revocation and replay after logout/password change, renaming, switching users, malformed/missing credentials, expiration, clock rollback, and credential-write failure.
- `cargo test -p ledger-application remembered_login --locked`: fixed seven-day expiry boundary passed, including one second before expiry and the exact deadline.
- `cargo build -p ledgers-bro --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, formatting, and diff checks passed.
- Native Windows app: selected Remember me, signed in, closed the process and reopened it using the same test directory; it signed in automatically. Signed out, closed and reopened again; it stayed at the login screen and the remembered credential was absent.
- Thai and English login layouts at widths 360, 430 and 1285 passed: 20-pixel checkbox, at least 44-pixel label target, no horizontal overflow, and clicking the label toggles the checkbox. Remember me is unchecked by default.
- Settings heading, tab strip and active content panel share both horizontal edges at widths 360, 430, 760, 1285 and 1440 across all four tabs. No horizontal overflow in those 20 checks. Thai/light and English/dark layouts were inspected in the Windows WebView.

For release 0.1.3, the full workspace suite passed: **227 passed, 0 failed, 2 ignored** (separate OCR runtime/fixture cases). Clippy and formatting passed. The Windows MSI/portable app and Android ARM64/x86_64 test APKs were built. The actual Windows release binary passed login and the same 20 Settings layout checks.

The Android emulator was updated in place from the published 0.1.2 APK. All original rows and columns matched across four ledgers and the user registry: four users, sixteen financial accounts, thirty-six journal entries and seventy-two postings, plus saved preferences, bill plans and ledger settings. The registry migrated from version 1 to 2; the ledger schema remained 10. Android Remember me survived a process restart; after sign-out and another restart it stayed at the login screen. The native token file had mode `0600` and length 32 bytes.

No physical Android-device test or complete interactive Windows installation test is claimed. The seven-day limit applies to automatic sign-in on opening the app; it does not terminate an already-open work session.
