# Android development build

The experimental Android host shares the Rust ledger and UI with desktop. It uses the test package `com.dancingwithmycode.ledgersbro.test`. The APK has a debug signature; this is not a Play Store release.

## Build on macOS, Linux, or Windows

Install Python 3.10+, Rust, JDK 21, Android SDK platform tools, NDK (side by side), and CMake through Android Studio's SDK Manager. Configure `ANDROID_HOME` and `JAVA_HOME`, or pass `--sdk` and `--jdk` to the build script. `--ndk` selects a specific NDK installation.

```sh
cargo install dioxus-cli --version 0.7.2 --locked
rustup target add aarch64-linux-android
python3 scripts/build-android.py --check
python3 scripts/build-android.py
adb devices
adb install -r target/android/ledgers-bro-test-arm64.apk
```

Use `python` if that is your Python executable's name. Connect an unlocked device with USB debugging enabled and accept its debugging prompt. For multiple devices, add `adb -s SERIAL`. See [Android's device guide](https://developer.android.com/studio/run/device).

For an x86_64 emulator:

```sh
rustup target add x86_64-linux-android
python3 scripts/build-android.py --target x86_64-linux-android
adb install -r target/android/ledgers-bro-test-x86_64.apk
```

The helper uses a project-local Gradle home and checked OCR dependencies, preserves the parent shell environment, and verifies that the APK contains only the requested ABI. The SHA-256 sidecar is written beside the APK. Gradle may need internet for its initial dependency downloads. Never delete app data to solve a build problem.

## Verification scope

Earlier Windows builds were opened on an Android emulator. The latest multiple-receipt Kotlin changes and portable Python build helper have not been compiled or run on Android in the current macOS session: no Android SDK/NDK is installed here. Desktop tests and the mobile-width screenshot do not constitute Android device verification.

Before distributing an APK, verify receipt selection/cancellation, voice permissions, lifecycle recovery, export, and persistence on a real device. Signing and Store packaging remain separate work.
