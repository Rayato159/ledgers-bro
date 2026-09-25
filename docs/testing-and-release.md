# Testing and releases

## Isolate all test data

Use generated fixtures and clearly named test users in a dedicated `--data-dir`. Android tests use a dedicated AVD with synthetic data. Never copy a user's real ledger, receipt, backup or installed model into tests. Never reset or uninstall a user's app to make a test pass.

Keep captures, logs, fixture databases, exports, downloaded test weights and signing credentials in ignored private directories. Public source fixtures must be recognizably synthetic. Before staging or publishing, inspect tracked and staged files as well as package contents: ignore rules alone do not protect already tracked files.

## Core checks

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

Native OCR/inference tests that require external runtimes or weights may be ignored by default. Record what actually ran. Use targeted accounting, recurrence, backup/migration and UI lifecycle tests when changing those behaviors; compile and lint success do not prove native app behavior.

## Native release verification

1. Build the [Windows package](windows-install.md) and both [Android ABIs](android-emulator.md).
2. Check package versions, Android application ID, ABI and signing-certificate continuity. Retain Windows upgrade identity.
3. Open the final Windows executable against a synthetic data directory. Navigate real screens and exercise the changed flows.
4. Install the x86_64 APK in the dedicated Android Studio AVD. Compare populated synthetic database snapshots before and after upgrading, then exercise the APK UI. ARM64 build/signature checks are distinct from physical-device tests.
5. Check narrow layouts, keyboard/focus behavior and touch interactions relevant to the change. Use actual financial invariants and unchanged-row checks where appropriate.
6. State unresolved limits precisely, including physical voice/OCR coverage, installer interaction and untested hardware. Never describe an emulator as a real phone.

## Publication and cleanup

Update the workspace version, lockfile, README and current update guide. Audit the final source diff for private data, then commit it. Package from that exact clean commit using explicit allowlists. Verify archive contents, checksums and version metadata before upload.

Publish the tag and GitHub Release with platform packages, SHA-256 checksums and build/verification details. Verify public downloads against the local hashes and confirm that the latest-release endpoint points to the new version.

After successful verification, stop only test-owned processes. Resolve cleanup targets and confirm that they stay inside the intended workspace before deleting build output. Remove only models downloaded specifically for tests; preserve installed user models, real data, source assets, signing identity and release deliverables.
