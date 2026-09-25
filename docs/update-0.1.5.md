# Update to 0.1.5

This release adds platform updates and reminders, simplifies entry review, improves financial details and replaces the mascot with Lookhin's original uncle crab.

## What changed

- **Settings → App updates** checks this repository's latest release, downloads the matching Windows MSI or Android APK and verifies its SHA-256 before handing it to the system installer. Installation still requires OS confirmation. Android also validates the package, signing certificate and newer version.
- The **notification bell** lists available versions, unpaid bills due within three days or overdue, and receivables whose collection date has arrived. Read status and preferences are per user. Optional system banners omit amounts and debtor names. Reminders run while the app is open; no background scheduling while closed is included.
- **Compose / Manual entry** replaces the extra quick-entry tabs. Receipt, voice and example controls sit beside the composer. Examples and parsed review cards open in dialogs. The compact model selector supports downloading; downloaded models can be deleted in **Settings → On-device AI**.
- Edits request **Confirm / Cancel** and reload saved ledger data after success. Canceled settings retain their previous values. Editing a stopped recurring plan respects its active months without reopening it.
- Transaction rows align their titles, canceled badges and amounts. Monthly bill bars and rows show amount percentages, separately from the count-based progress ring. Linked card repayments show their allocation to recorded charges; legacy free-text payments cannot reconstruct missing purchase detail.
- Notification settings use aligned descriptions and switches. Tabs remain scrollable on narrow screens.
- Six uncle-crab poses replace the former cast in page branding, login, overview and empty states. Windows and Android launcher icons use the same character. [Artwork and credits](../design/uncle-crab/README.md).

## Install over the existing version

On Windows, run the new MSI over the existing installation. Portable users should extract the entire ZIP and keep its `assets`, `ocr` and `licenses` folders beside the executable. Keep using the same data directory.

On Android, install the matching APK over the existing app. Do not uninstall or clear storage. The package remains `com.dancingwithmycode.ledgersbro.test`, with the existing test signing identity and version code 1005. These are sideload/test APKs, not Play Store builds. Use ARM64 on a compatible phone and x86_64 on an x86_64 Android Studio emulator.

The new updater becomes available after installing 0.1.5. Version 0.1.4 needs this one manual update. Notification and updater files do not replace the ledger. Financial schema 10 and profile schema 2 are unchanged; users, passwords, transactions, balances, recurring plans and receivables remain in place. A full `.lbro` export in **Settings → Data** provides an independent recovery copy.

Windows binaries remain unsigned. Android requires permission to install packages from this app before its updater can open an APK. OS permission and installer prompts are part of the update flow.

## Verification scope

Automated tests, strict Clippy and formatting checks run against the release source. Native UI verification uses independent synthetic profiles and directories, never personal ledgers. Android testing uses the dedicated Android Studio emulator; a physical-phone test and a full Windows MSI installation are not claimed. `BUILDINFO.txt` and `SHA256SUMS.txt` in the release identify the exact source, packaged binaries and checks performed.
