# Tanuki launcher icon

Generated with Codex's built-in image generation using the app's existing tanuki artwork as a reference. The drawing uses a warm cream, chocolate brown, yellow, and sage palette to match the app. The exact generation prompt is in [prompt.txt](prompt.txt).

- `tanuki-icon-source.png`: original generated RGBA artwork, retained unchanged.
- `launcher-1024.png`: square master with a warm yellow background.
- `launcher-512.png`: smaller preview.
- `../../apps/android/native/res/`: adaptive icon layers and legacy launcher PNGs for five Android densities.

Regenerate the Android resources and previews with `./scripts/export-app-icons.ps1`. This script uses ImageMagick only for mechanical trimming, resizing, padding, and export; it does not redraw the artwork.

Android's manifest points both `icon` and `roundIcon` to `ledger_launcher`. The Gradle initialization hook adds the source-controlled resource directory to the generated Android project so the icon survives `cargo clean` and subsequent Dioxus builds.

The adaptive icon keeps the illustration inside the center safe area and leaves masking to the launcher. See the [Android adaptive icon documentation](https://developer.android.com/develop/ui/compose/system/icon_design_adaptive). A separate monochrome layer is not supplied.

## Verification — 2026-09-21

- `cargo fmt --all -- --check` passed.
- Ran `cargo clean` once: 45,708 files / 27.3 GiB removed.
- Fresh x86_64 Android debug build succeeded via `scripts/build-android.ps1`.
- Inspected the APK resource table: the manifest resolves to the custom adaptive icon, its foreground/background, and all five legacy density resources.
- Installed with `adb install -r` on `emulator-5554` (API 37); checked the new circular launcher icon visually and opened the app by tapping it.
- Dashboard opened with the existing one-account balance of THB 1,000 retained. Screenshots: `.preview/icon-launcher-home.png` and `.preview/icon-app-check.png` at the project root.
- APK: `target/android/ledgers-bro-test-x86_64.apk`. SHA-256: `69ea5a868c6f6f93a4ab2237041199fa02147a159209ef250c49f6a7cdae098a`.

This verifies the emulator debug build and circular launcher mask; other launcher masks and physical devices were not exercised in this change.
