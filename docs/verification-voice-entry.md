# Voice entry verification — 2026-09-21

## Implemented

- Android on-device Thai speech adapter, request-time microphone permission, explicit offline model download, status/cancel/stop controls, and editable text before the existing deterministic review/confirmation flow.
- Separate UI session state, native JNI/Android effect adapter, and application text interpretation. No changes to persistence or monetary invariants.
- Dropped tasks release capture; native callbacks are session-scoped, time-bounded, and cancelled on Activity stop/destroy. Audio and transcripts are not logged or saved as files by the app. No cloud recognition fallback.
- Numeric Thai shorthand accepts a baht suffix and missing word spaces, but rejects ambiguous alternatives, number words, foreign currencies, fractional satang, or trailing instructions.

## Checks actually run

- `cargo fmt --all -- --check` passed.
- Workspace Clippy, all targets, `-D warnings`, locked/offline passed.
- Android-target Clippy with the installed x86_64 NDK toolchain passed, including JNI-only code.
- Workspace tests: **76 passed, 1 optional installed-runtime OCR test ignored**. Seven voice tests cover exact/oversized transcripts, cancellation while waiting for permission/results, stale-session identities, release after completion, no automatic ledger commands, and separation of model download from microphone requests. Two parser tests cover spoken shorthand and rejection of ambiguous/unsafe tails.
- Windows debug build and Android x86_64 debug APK built successfully. Kotlin compiled in the Android Gradle build.
- Final APK: `target/android/ledgers-bro-test-x86_64.apk`; SHA-256 `48AE8E06C0D21E5FCDD56AA13352EA0CC6EDD321E22D11A630907CA3247E1ECF`. Debug build, not a Store artifact.

## Interactive verification

Android: existing Pixel_6 emulator, Android 17/API 37, x86_64 16 KB image. Updated `com.dancingwithmycode.ledgersbro.test` in place; did not uninstall or wipe data.

- Voice button switches to “พูดแทนข้อความ” when a draft exists. Missing Thai model is reported and leaves `กาแฟ80.50บาท` intact.
- System capability query identified Thai as downloadable but not installed. The download button reached Android's own **Download ไทย (ประเทศไทย) update** dialog, which reported **37.38 MB**. Download was requested/confirmed; a later capability query reported **pending installation**, not readiness. Download size/status are specific to this image/provider.
- `RECORD_AUDIO` remained `granted=false` after model setup. Host microphone input was not enabled and no user audio was recorded.
- Manual submission of the speech-shaped text `กาแฟ80.50บาท` produced an uncommitted **80.50 THB** draft; account/category were not guessed and the preview button remained disabled until completed.
- Net assets stayed **920.00 THB**, with one existing test account and the pre-existing 80.00 THB expense. Neither model setup nor draft interpretation created a ledger entry.
- Android WebView had no horizontal overflow. Voice controls use the existing dark slate/neutral palette and rounded button style.
- Windows UI showed its voice button disabled with an explicit explanation that a local desktop recognizer is not installed/implemented. Text and receipt entry remain available.
- Android screenshot: `.preview/voice-entry-android.png`. Windows UI was inspected through its DOM; screenshot capture timed out on the background WebView2 surface, so no Windows screenshot is claimed.

## Not yet verified

No real speech recognition accuracy test, physical Android/ARM64 test, permission grant/denial recording flow, noisy-room/number confusion test, offline audio test, or real-microphone lifecycle test was performed. Cancellation contracts were tested with a fake adapter; emulator model availability/download and draft UI were exercised against the real host. Model download requested/pending is not proof it installed successfully.

Windows has no ASR adapter. Android uses the system speech model, not an embedded cross-platform model or LLM. This remains a development/test APK, not a production or Store release. Follow [the voice test checklist](voice-entry-th.md) on supported hardware before release.
