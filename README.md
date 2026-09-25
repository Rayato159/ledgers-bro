# Ledgers Bro

A local-first personal finance app built with **Rust, Dioxus, and SQLite**, with Thai and English interfaces, light and dark themes, and Lookhin's original uncle-crab mascot.

## Download

**[Get version 0.1.7](https://github.com/Rayato159/ledgers-bro/releases/tag/v0.1.7)** or open **More → Settings → App updates** in an existing installation.

| Platform | Package |
| --- | --- |
| Windows x64 | `.msi` installer or portable `.zip` |
| Android phone | ARM64 test `.apk` |
| Android x86_64 emulator | x86_64 test `.apk` |

Windows packages are unsigned. Android packages use the existing debug/test signing identity and are not Play Store releases. No macOS, Linux, or iOS packages are provided.

Extract the complete portable ZIP and keep `assets`, `ocr`, and `licenses` beside `ledgers-bro.exe`. Windows receipt OCR is included; optional AI models download separately inside the app. SHA-256 checksums and build details accompany each release.

Back up in **Settings → Data** before upgrading. Install over the existing app to retain users and data. On Android, do not uninstall or clear storage. The in-app updater downloads the matching package, verifies its SHA-256, and opens the system installer for OS confirmation.

## Features

- **Local users:** create separate users, edit credentials, switch users, and sign out from **Settings → General**. Optional **Remember me** keeps a login for seven days.
- **Accounts and transactions:** track cash, banks, credit cards, investments, and BTC/SOL holdings. Record income, expenses, transfers, and card repayments. Each ledger uses one currency; language changes do not convert money.
- **Bills and receivables:** manage recurring plans, installments, overdue bills, money lent, and repayments. Change a plan from a selected month, with confirmation before saving and an automatic refresh afterward. Earlier periods and payment history retain their original terms.
- **Reports:** browse grouped views and yearly history, inspect expense and debt breakdowns, and hover pie-chart slices for names, amounts, and percentages. Select a slice or label to open details. “Other expenses” use their descriptions where available.
- **Quick entry:** write a Thai prompt, attach receipts, or fill the form manually. AI output opens as editable review cards before saving. On-device speech input is available on supported Android devices.
- **Optional local AI:** choose Qwen3 0.6B, 1.7B, 4B, 8B, or 14B in Q4_K_M format. The app estimates memory and disk requirements before downloading. Downloads continue across tabs while the app session stays open. Remove models in **Settings → On-device AI**.
- **Import and export:** import supported CSV files using the provided template, export CSV for spreadsheets, or export a `.lbro` backup to move saved ledger data to another device. Create a local user with an empty ledger before restoring. Backups cover the current user's saved ledger, not every profile on the device; passwords and AI model files are excluded.
- **Notifications:** the bell shows available updates, bills due, and collections due. Opening a notification marks it read and archives it. Settings control reminder types and optional system banners. The last 256 archived notifications are saved per user.
- **Optional Thai tax tools:** classify taxable income and record withholding and deductions in a THB ledger. These tools do not submit tax returns.

## What's new in 0.1.7

- Model downloads no longer disappear or cancel when switching tabs. Progress, completion, and errors remain available after returning.
- The recurring-plan editor lets you choose the effective month and the plan that applies then, instead of trapping the month picker inside an old stopped plan.
- All pie charts show immediate tooltips and support mouse, keyboard, and touch interaction. Tiny segments no longer show a neighbouring item's value.

See the [0.1.7 update guide](docs/update-0.1.7.md).

## Data and current limits

Ledgers stay on the device. There is no cloud sync, and the local database is not encrypted. Separate profiles are a convenience on a trusted device, not protection against someone who can read its files. Make regular backups; the new-year reminder keeps history rather than deleting it.

The app fetches public crypto prices, release metadata, and model downloads when those features are used. Transaction prompts and receipt images are processed locally. Notifications currently run while the app is open; closed-app background reminders are not implemented. Local AI runs on the CPU, and hardware estimates do not guarantee speed or accuracy.

Supported ledger currencies: THB, USD, EUR, GBP, AUD, CAD, SGD, CNY. Automatic currency conversion is not supported. Thai tax calculations and receipt parsing target THB. Speech support depends on the Android device; desktop voice transcription is not available in the packaged build.

On Windows, the default data folder is `%LOCALAPPDATA%\Dancing With My Code\Ledgers Bro\data`. Downloaded AI models are `.gguf` files inside its `models` subfolder. Prefer removing them through Settings; do not delete the data folder to remove a model.

## Build from source

Use **Rust 1.88+**, **Dioxus CLI 0.7.2**, and Python 3.10+ for packaging. Native builds require CMake and libclang. Windows also requires MSVC C++ tools and WebView2. Run commands from the repository root.

### Windows

Run a separate development ledger with a clearly named test profile and synthetic records only. Do not copy a real ledger into the test directory.

```sh
cargo run -p ledgers-bro --locked -- --data-dir .data/sandbox
cargo build -p ledgers-bro --release --locked
```

Add `--mobile-preview` to the run command to inspect a narrow desktop layout; this is not an Android test. To package an MSI, first prepare the [OCR runtime](docs/receipt-ocr.md), then run:

```sh
python scripts/build-windows.py --check
python scripts/build-windows.py
```

The installer is written to `target/installers/`. See the [Windows installation guide](docs/windows-install.md).

### Android

Install Android Studio, JDK 21, SDK platform tools, NDK, and CMake. Set `JAVA_HOME` and `ANDROID_HOME` to their installation folders.

```sh
cargo install dioxus-cli --version 0.7.2 --locked
rustup target add aarch64-linux-android
python scripts/build-android.py --check
python scripts/build-android.py
adb install -r target/android/ledgers-bro-test-arm64.apk
```

For an x86_64 emulator, install the `x86_64-linux-android` Rust target, pass `--target x86_64-linux-android` to the build script, and install `target/android/ledgers-bro-test-x86_64.apk`. Test with an isolated emulator and synthetic data. See the [Android build and emulator guide](docs/android-emulator.md).

### Checks

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

## Guides and licenses

[Documentation index](docs/README.md)

- [CSV import, export, and device migration](docs/data-transfer.md)
- [Local users and login](docs/local-users.md)
- [Model selection and hardware estimates](docs/local-model-selection.md)
- [Credit cards and financial reports](docs/credit-cards.md)
- [Taxable income entries](docs/tax-income-entries.md)
- [Mascot artwork and credits](design/uncle-crab/README.md)

Source code: [MIT](LICENSE). Third-party [model and OCR notices](licenses/) and [font licenses](crates/ui/assets/fonts/LICENSE.txt) apply to their respective assets.
