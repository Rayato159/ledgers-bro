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

The helper uses a project-local Gradle home and checked OCR dependencies, preserves the parent shell environment, and verifies that the APK contains only the requested ABI. Android's version name follows the workspace version; its version code is `major * 1000000 + minor * 1000 + patch`. The SHA-256 sidecar is written beside the APK. Gradle may need internet for its initial dependency downloads. Never delete app data to solve a build problem.

## Verification scope

Use a dedicated Android Studio AVD containing only synthetic users and data. Install over the previous test package with `adb install -r`, preserving its signing identity. Verify old rows after the upgrade before adding new test records, then navigate the actual APK and exercise changed flows.

Every release's BUILDINFO file records the checks actually performed. An x86_64 emulator smoke test does not verify an ARM64 physical device, microphone language support or all vendor image decoders. Physical-device and production signing/Store packaging checks remain separate work. See [testing and releases](testing-and-release.md).
