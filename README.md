# Ledgers Bro ✨

Your money ghosted you. Let's find it.

An open-source, local-first ledger built with **Rust + Dioxus + SQLite**. Thai / English UI, light / dark themes. Your wallet, your device, your business.

## Download & install 📦

Download **[0.1.6](https://github.com/Rayato159/ledgers-bro/releases/tag/v0.1.6)**. No Rust or build tools needed to try the packaged apps.

- **Windows x64:** install the `.msi`, or extract the portable `.zip` and open `ledgers-bro.exe` inside it. Keep its `ocr` and `licenses` folders alongside the executable.
- **Android phone:** use the **arm64** `.apk`. This is an experimental, debug-signed test app, not a Play Store build.
- **Android emulator:** use the **x86_64** `.apk` for an x86_64 virtual device.
- **macOS / Linux / iOS:** no ready-to-install packages announced yet.

The release includes SHA-256 checksums. Windows packages include receipt OCR; download the optional local AI model inside the app. Windows packages are not code-signed. Upgrade the existing app in place; do not uninstall the Android app or clear its storage. Version 0.1.6 moves user editing and sign-out into Settings to keep every page header clear. It keeps existing users, passwords and ledger data. The financial ledger remains at schema 10 and the user registry at version 2. See [what changed and how to update](docs/update-0.1.6.md), [local model selection](docs/local-model-selection.md), [local users](docs/local-users-th.md), and [CSV / device migration](docs/data-transfer-th.md). Keep a pre-update backup if you need to roll back. The source code below is for building it yourself. 🛠️

## Version 0.1.6

The signed-in username, **Edit user**, and **Switch user / Sign out** now live in **More → Settings → General**. They no longer take up space above each page.

Opening a notification now marks it read and moves it into **Archived**. The bell has separate **Unread** and **Archived** views, a wider dialog, and a **Read and archive all** action. Each user keeps their last 256 read notifications, including the original due dates and amounts after a bill is paid.

## Version 0.1.5 highlights

- The app's mascot is now Lookhin's original uncle crab: a cheeky, kind, thrifty salaryman. Six colored poses appear in the existing page branding, login and empty states, with matching Windows and Android launcher icons. [Artwork, credits and generation prompts](design/uncle-crab/README.md).
- Settings can check the latest public GitHub release, download the matching Windows MSI or Android APK, and verify its SHA-256 before opening the system installer. Installation requires OS confirmation. Android also checks the package identity, signing certificate, and version. Update the existing installation to retain data; uninstalling or clearing Android storage removes local data.
- A bell collects new-version, unpaid-bill, and collection reminders. Notification preferences and read status belong to each local user. Optional device banners omit amounts and debtor names. Reminders currently run while the app is open.
- Quick entry has Write / Manual tabs, compact receipt and voice controls, a model selector, and prompt examples in a dialog. Parsed entries open as review cards before anything is saved. Downloaded models can be removed in Settings.
- Editing asks for confirmation and reloads the latest ledger after a successful save. Bill amounts show their percentage of the monthly plan, separate from the count of paid bills. Linked credit-card payments show their allocation to recorded charges; unlinked legacy amounts cannot reconstruct missing purchase details.

## The receipts 📸

Real app captures with synthetic data. The mobile image shows the responsive layout in a narrow desktop window, not an iOS build.

![Desktop overview in English](docs/images/desktop-overview-en.png)

<table>
  <tr>
    <td width="72%"><img src="docs/images/desktop-receivables-dark-en.png" alt="English receivables page in dark mode" /></td>
    <td width="28%"><img src="docs/images/mobile-accounts-en.png" alt="English accounts page at mobile width" /></td>
  </tr>
</table>

## Build & try it 🧑‍💻

Clone the repository and install **Rust 1.88+**. Run commands from the repository root unless stated otherwise.

### Desktop — macOS, Windows, Linux

Install your platform's [Dioxus native prerequisites](https://dioxuslabs.com/learn/0.7/getting_started/), plus CMake and libclang:

| Platform | Native tools |
| --- | --- |
| macOS | Xcode Command Line Tools, CMake, LLVM |
| Windows | MSVC C++ Build Tools, CMake, LLVM, WebView2 |
| Linux | C/C++ toolchain, CMake, Clang, WebKitGTK 4.1 and the Dioxus system libraries |

```sh
cargo run -p ledgers-bro --locked -- --data-dir .data/sandbox
```

Use a clearly named test profile such as `ui_test_only` inside this separate directory, and populate it with synthetic data. Do not copy a real ledger or backup into a test directory. Verify the running process has the expected `--data-dir` before automated UI actions; a separate profile alone is not a substitute for a separate test directory.

Add `--mobile-preview` after `--data-dir .data/sandbox` to check the phone-width layout on desktop. For an optimized build:

```sh
cargo build -p ledgers-bro --release --locked
```

Run `target/release/ledgers-bro` (`ledgers-bro.exe` on Windows). Optional receipt scanning needs the [OCR runtime setup](docs/receipt-ocr.md).

To build a Windows `.msi` with the receipt runtime and application icon, install Python 3.10+
and Dioxus CLI 0.7.2, prepare the OCR runtime above, then run:

```sh
python scripts/build-windows.py --check
python scripts/build-windows.py
```

Open the resulting `.msi` in `target/installers/`, follow the installer, then launch
**Ledgers Bro** from Start Menu. [Windows build and installation guide (Thai)](docs/windows-install-th.md).

### Android — device or emulator (experimental)

1. Install Python 3.10+, JDK 21, and Android Studio. In SDK Manager, install SDK platform tools, NDK, and CMake. Set `JAVA_HOME` and `ANDROID_HOME` to their installation folders.
2. Install the Rust target and pinned Dioxus CLI, then build:

```sh
cargo install dioxus-cli --version 0.7.2 --locked
rustup target add aarch64-linux-android
python3 scripts/build-android.py --check
python3 scripts/build-android.py
```

3. Connect a phone with USB debugging enabled, accept its debugging prompt, and install the test APK:

```sh
adb devices
adb install -r target/android/ledgers-bro-test-arm64.apk
```

4. Open **Ledgers Bro** on the device. This APK uses a debug signature.

Use `python` if that is your Python executable's name. For an x86_64 emulator, install the `x86_64-linux-android` Rust target, build with `--target x86_64-linux-android`, and install `target/android/ledgers-bro-test-x86_64.apk`. [Full Android guide](docs/android-emulator-th.md).

### iOS

No iOS host or working build steps yet, so there is no IPA to run on an iPhone. The “soon™” era continues. 🫠

### Tests

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

## Tiny but relevant 🧾

In **More → Settings**, choose a currency before creating the first account. Each ledger uses one currency; no automatic conversion. Supported: THB, USD, EUR, GBP, AUD, CAD, SGD, CNY. Desktop can use separate `--data-dir` folders for separate ledgers. Thai tax is optional and THB-only. Prompts and speech currently use Thai; receipt OCR supports THB receipts.

Credit cards require a statement closing day and payment day. **Debts & bills** shows outstanding statement balances, monthly plans, and payment progress; card repayments are transfers, so expenses are counted once. [Credit billing and financial overview (Thai)](docs/credit-cards-th.md).

[MIT](LICENSE). Fork it, fix it, make your wallet less embarrassing. Third-party [models / OCR](licenses/) and [fonts](crates/ui/assets/fonts/LICENSE.txt) retain their own licenses. Early software: try synthetic data first; the local database is not encrypted.

## Version 0.1.1 highlights

- Make full or partial credit card repayments from the entry form, with a chart showing debt by card.
- Classify taxable income and record its gross amount, VAT and withholding tax for annual tax calculations.
- [Update without losing existing data (Thai)](docs/update-0.1.1-th.md) · [Taxable income entries (Thai)](docs/tax-income-entries-th.md)
