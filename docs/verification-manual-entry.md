# Manual entry and centered accounts — 2026-09-21

The header's **กรอกเอง** button and the shortcut above quick-entry chat open a dedicated form. It reuses the existing preview/commit use cases for expense, income, and transfer; the manual page does not mount chat, model settings, or OCR. The date defaults to the dashboard's current date, and account/category choices remain explicit. An active interpretation can be cancelled before opening a fresh manual draft; an active database operation still blocks resetting the form.

## Checks completed

- `cargo fmt --all -- --check` passed.
- `cargo test -p ledger-ui -p ledger-application --locked`: 54 tests passed. The new regression covers opening without AI, escaping a stalled model, and protecting an active save. Existing cancellation tests reject late proposals.
- `cargo clippy -p ledger-ui --all-targets --locked -- -D warnings` passed.
- `scripts/build-android.ps1` succeeded; installed the debug x86_64 APK on Pixel_6 / `emulator-5554`.
- Both manual entry points opened the form, with `2026-09-21` selected and no horizontal overflow. Incomplete forms disabled preview.
- Through the emulator UI, created a temporary account with THB 500, manually saved an expense of THB 80.25 with a Thai note (balance 419.75), and manually saved income of THB 100.75 (balance 520.50). Each required an explicit preview and confirmation.
- Transfer controls excluded the source account from destination choices; the shared form produced the correct THB 25 transfer preview. This transfer was not committed.
- Removed the temporary account and its two test transactions through the confirmation dialog. The existing cash account remained at THB 1,000.
- Account cards fill the mobile grid. Measured card/icon/button centers at approximately 205.71 CSS pixels and checked the centered layout visually.

Screenshots at the repository root: `.preview/manual-form.png`, `.preview/account-centered.png`. APK: `target/android/ledgers-bro-test-x86_64.apk`, SHA-256 `ecaec5620f898fa55332c661688aed5ada7d14a0b3721d571951c05eb1ff5d61`.

These checks cover the emulator and targeted UI/application behavior. Physical Android devices and iOS were not tested in this change.
