# Android test host verification — 21 September 2026

## Artifact and environment

- Debug package: `com.dancingwithmycode.ledgersbro.test`, launcher label **Ledgers Bro Test**.
- APK: `target/android/ledgers-bro-test-x86_64.apk`, 163,118,830 bytes.
- SHA-256: `EAAB81E43C8E5047FE534A24E4D44656C368C44E62187AD1333D9A7E13FC6AB4`.
- Tested device: `emulator-5554`, Pixel 6 AVD, Android 17 / API 37, x86_64, 16 KB page-size image.
- Windows host; Rust 1.95, Dioxus CLI 0.7.2, JDK 21.0.2, NDK 30.0.16248370.
- CLI-generated Gradle 9.1.0 / Android Gradle Plugin 8.7.0 / Kotlin 2.0.20; compileSdk and targetSdk 33, minSdk 24. This is a debug test build, not a Play Store release configuration.

## Implemented boundary

`apps/android` composes the existing domain, application, SQLite worker and Dioxus UI. Safe JNI calls obtain Android's private no-backup directory and invoke a small Kotlin CSV save adapter. Kotlin does not calculate balances or journal entries. The Android database is separate from Windows data, and the test package does not alter other apps.

CSV saving uses `ACTION_CREATE_DOCUMENT` and a background writer through `ContentResolver`. The UI reports success only after the write closes and uses the provider's saved display name. Cancel returns to the form without a success message. OCR capability is explicitly unavailable on Android; the manual form remains usable. Art and TH Sarabun New are bundled.

The INTERNET permission is required by this Dioxus version's authenticated UI WebSocket on `127.0.0.1`; cleartext network configuration is limited to that loopback domain. There is no ledger backend or cloud sync. A user-selected cloud document provider can upload an exported file.

## Checks actually run

| Check | Observed result |
|---|---|
| `scripts/build-android.ps1` | APK built and copied successfully; final build log: `.preview/android-build-final.log` |
| `scripts/run-android.ps1 -Device emulator-5554` | Installation over the same package and launch succeeded, including the bounded launch retry |
| `cargo test --workspace --locked --offline` | 67 passed; 1 optional native OCR integration test ignored; `.preview/android-workspace-tests.log` |
| Workspace Clippy with `-D warnings` | Passed after initial Android host integration; `.preview/android-clippy.log` |
| Android x86_64 Clippy with `-D warnings` | Passed again after final Rust edits; `.preview/android-target-clippy.log` |
| `cargo fmt --all -- --check` | Passed after final Rust edits |
| PowerShell parser: build/run/check scripts | No syntax errors |

Workspace tests ran before the last small UI wording and exported-filename adapter edits. The final APK build, Android-target Clippy and functional export check include those edits. No new business logic was introduced.

## Emulator flows observed

1. Launched the installed APK with Thai UI, bundled font and mascot art.
2. Created the test cash account **เงินสดทดสอบ Android**, opening balance **1,000.00 THB**.
3. Added a food expense of **80.00 THB**, note **ทดสอบ Android: กาแฟ 80 บาท**. The date field defaulted to **2026-09-21**, the local test date. Preview/confirmation saved the entry; overview showed **920.00 THB** and monthly expenses **80.00 THB**.
4. Reinstalled the updated APK over the same package and restarted it. The balance and expense persisted.
5. Opened CSV export, cancelled the Android save picker with Back, then reopened it and saved successfully to Downloads. The form remained usable after cancellation.
6. Pulled the actual device file `/sdcard/Download/สมุดรายวันทั่วไป-2026-09-21.csv` to `.preview/android-journal-th.csv`. Verified UTF-8 BOM, Thai column labels and note, four journal posting rows, and independently summed **debits = credits = 1,080.00 THB** (opening balance plus the expense).
7. Enabled airplane mode, disabled Wi-Fi, force-stopped only the test package and cold-started it. The persisted **920.00 THB** balance remained visible and was checked through the WebView. Screenshot: `.preview/android-offline-overview.png`. Restored airplane mode to off and Wi-Fi to on afterward.
8. Ran the final install/launch script again and left the test app open. Test data and the exported file remain available for the user.

## Known limits

- Only the x86_64 emulator APK was built and exercised. ARM64 build support in the scripts has not been validated on an ARM64 device or emulator.
- This is a debug build with a debug signing key and a large unoptimized APK. Release signing, size/memory work, supported SDK policy and Play Store delivery remain outstanding.
- Android receipt OCR is not implemented. Local LLM, PIN/biometrics, database encryption, recoverable backup, PDF, a production Thai tax engine and bank notification import remain unavailable.
- Auto Backup is disabled and the database uses a no-backup directory. Uninstalling or clearing app data removes it. CSV exports are reports, not a complete backup/restore format.
- Rotation, activity/process recreation during export, interrupted writes, low-storage providers, Thai IME edge cases and all transfer/deletion flows have not been exhaustively exercised on Android. The desktop/shared test suite is not evidence that every mobile interaction works.
- The selected journal sample balances. This does not certify every accounting scenario or statutory tax compliance.

See [Android setup and run guide](android-emulator-th.md) for repeatable commands.
