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

For release 0.1.0, both ARM64 and x86_64 APKs were compiled on Windows with the shared Rust code and Kotlin receipt bridge. Their package ID, version, ABI, and APK signatures were checked. Earlier builds were opened on an emulator, but no fresh physical-device or full Android UI verification is claimed for these release artifacts. Desktop tests do not constitute Android device verification.

Before distributing an APK, verify receipt selection/cancellation, voice permissions, lifecycle recovery, export, and persistence on a real device. Signing and Store packaging remain separate work.
